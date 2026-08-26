use chrono::{DateTime, Utc};
use fs2::available_space;
use futures_util::{stream, StreamExt};
use pmtiles::{
    AsyncBackend, AsyncPmTilesReader, DirectoryCache, HashMapCache, Header, PmTilesWriter,
    TileCoord, TileType,
};
use reqwest::{header::RANGE, Client};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use thiserror::Error;

pub const MANIFEST_VERSION: u32 = 1;
pub const WEB_MERCATOR_MAX_LATITUDE: f64 = 85.051_128_78;
pub const BASEMAP_MAX_ZOOM: u8 = 15;
pub const TERRAIN_MAX_ZOOM: u8 = 14;
pub const MAX_REQUESTED_TILES: u64 = 2_000_000;
pub const BASEMAP_ATTRIBUTION: &str = "© OpenStreetMap contributors; Protomaps basemap";
pub const TERRAIN_ATTRIBUTION: &str =
    "Mapterhorn terrain and its attributed open elevation sources";
const BASEMAP_BYTES_PER_TILE_ESTIMATE: u64 = 8_000;
const TERRAIN_BYTES_PER_TILE_ESTIMATE: u64 = 22_000;
const DOWNLOAD_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GeoBounds(pub f64, pub f64, pub f64, pub f64);

impl GeoBounds {
    pub fn west(self) -> f64 {
        self.0
    }

    pub fn south(self) -> f64 {
        self.1
    }

    pub fn east(self) -> f64 {
        self.2
    }

    pub fn north(self) -> f64 {
        self.3
    }

    pub fn parse(value: &str) -> Result<Self, MapAssetError> {
        let values = value
            .split(',')
            .map(str::trim)
            .map(|part| {
                part.parse::<f64>().map_err(|_| {
                    MapAssetError::InvalidBounds("Bounds must contain four numbers.".into())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() != 4 {
            return Err(MapAssetError::InvalidBounds(
                "Use west,south,east,north.".into(),
            ));
        }
        Self(values[0], values[1], values[2], values[3]).validate()
    }

    pub fn validate(self) -> Result<Self, MapAssetError> {
        if ![self.0, self.1, self.2, self.3]
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(MapAssetError::InvalidBounds(
                "All coordinates must be finite numbers.".into(),
            ));
        }
        if self.west() < -180.0 || self.east() > 180.0 {
            return Err(MapAssetError::InvalidBounds(
                "Longitude must be between -180 and 180 degrees.".into(),
            ));
        }
        if self.west() >= self.east() {
            return Err(MapAssetError::InvalidBounds(
                "West must be less than east; antimeridian regions are not supported yet.".into(),
            ));
        }
        let clamped = Self(
            self.west(),
            self.south()
                .clamp(-WEB_MERCATOR_MAX_LATITUDE, WEB_MERCATOR_MAX_LATITUDE),
            self.east(),
            self.north()
                .clamp(-WEB_MERCATOR_MAX_LATITUDE, WEB_MERCATOR_MAX_LATITUDE),
        );
        if clamped.south() >= clamped.north() {
            return Err(MapAssetError::InvalidBounds(
                "South must be less than north.".into(),
            ));
        }
        let tile_count = tile_count(clamped, BASEMAP_MAX_ZOOM)
            .saturating_add(tile_count(clamped, TERRAIN_MAX_ZOOM));
        if tile_count > MAX_REQUESTED_TILES {
            return Err(MapAssetError::InvalidBounds(format!(
                "This area needs about {tile_count} tiles. Choose a smaller rectangle."
            )));
        }
        Ok(clamped)
    }

    pub fn contains(self, other: Self) -> bool {
        self.west() <= other.west()
            && self.south() <= other.south()
            && self.east() >= other.east()
            && self.north() >= other.north()
    }

    pub fn area(self) -> f64 {
        (self.east() - self.west()) * (self.north() - self.south())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegionAsset {
    pub file: String,
    pub provider: String,
    pub dataset_version: String,
    pub source_url: String,
    pub tile_type: String,
    pub terrain_encoding: Option<String>,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub size_bytes: u64,
    pub sha256: String,
    pub attribution: String,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegionManifest {
    pub format_version: u32,
    pub id: String,
    pub name: String,
    pub bounds: GeoBounds,
    pub basemap: RegionAsset,
    pub terrain: RegionAsset,
    pub created_at: DateTime<Utc>,
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub name: String,
    pub bounds: GeoBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEstimate {
    pub basemap_tiles: u64,
    pub terrain_tiles: u64,
    pub basemap_bytes: u64,
    pub terrain_bytes: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct TerrainArchiveList {
    version: String,
    items: Vec<TerrainArchive>,
}

#[derive(Debug, Clone, Deserialize)]
struct TerrainArchive {
    name: String,
    url: String,
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
    min_zoom: u8,
    max_zoom: u8,
}

#[derive(Debug)]
struct ResolvedTerrainSource {
    version: String,
    max_zoom: u8,
    archives: Vec<TerrainArchive>,
}

impl TerrainArchive {
    fn intersects(&self, bounds: GeoBounds) -> bool {
        self.min_lon < bounds.east()
            && self.max_lon > bounds.west()
            && self.min_lat < bounds.north()
            && self.max_lat > bounds.south()
    }

    fn covers_tile(&self, coordinate: TileCoord) -> bool {
        coordinate.z() >= self.min_zoom
            && coordinate.z() <= self.max_zoom.min(TERRAIN_MAX_ZOOM)
            && self.intersects(tile_bounds(coordinate))
    }
}

fn tile_bounds(coordinate: TileCoord) -> GeoBounds {
    let scale = (1_u64 << coordinate.z()) as f64;
    let longitude = |x: f64| x / scale * 360.0 - 180.0;
    let latitude = |y: f64| {
        (std::f64::consts::PI * (1.0 - 2.0 * y / scale))
            .sinh()
            .atan()
            .to_degrees()
    };
    GeoBounds(
        longitude(coordinate.x() as f64),
        latitude(coordinate.y() as f64 + 1.0),
        longitude(coordinate.x() as f64 + 1.0),
        latitude(coordinate.y() as f64),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum DownloadEvent {
    Diagnostic {
        level: String,
        message: String,
    },
    ResolvingSources {
        message: String,
    },
    LayerStarted {
        layer: String,
        total_tiles: u64,
    },
    Progress {
        layer: String,
        completed_tiles: u64,
        total_tiles: u64,
        downloaded_bytes: u64,
    },
    Verifying {
        layer: String,
    },
    Complete {
        region_path: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Basemap,
    Terrain,
}

impl AssetKind {
    pub fn filename(self) -> &'static str {
        match self {
            Self::Basemap => "basemap.pmtiles",
            Self::Terrain => "terrain.pmtiles",
        }
    }
}

#[derive(Debug, Error)]
pub enum MapAssetError {
    #[error("{0}")]
    InvalidBounds(String),
    #[error("Invalid area name. Enter at least one letter or number.")]
    InvalidName,
    #[error("Invalid region identifier.")]
    InvalidRegionId,
    #[error("Map region was not found: {0}")]
    RegionNotFound(String),
    #[error("Unsupported region manifest version {0}.")]
    UnsupportedManifest(u32),
    #[error("Map archive is invalid: {0}")]
    InvalidArchive(String),
    #[error("Map data error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid region manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Map provider error: {0}")]
    Provider(String),
    #[error("The map download was cancelled.")]
    Cancelled,
    #[error("This map region is already downloaded: {0}")]
    RegionAlreadyExists(String),
    #[error("There is not enough free disk space for this download.")]
    InsufficientSpace,
}

impl DownloadRequest {
    pub fn validate(&self) -> Result<(), MapAssetError> {
        slugify(&self.name)?;
        self.bounds.validate()?;
        Ok(())
    }
}

pub fn estimate_download(
    root: &Path,
    request: &DownloadRequest,
) -> Result<DownloadEstimate, MapAssetError> {
    request.validate()?;
    let bounds = request.bounds.validate()?;
    ensure_maps_layout(root)?;
    let basemap_tiles = tile_count(bounds, BASEMAP_MAX_ZOOM);
    let terrain_tiles = tile_count(bounds, TERRAIN_MAX_ZOOM);
    let basemap_bytes = basemap_tiles.saturating_mul(BASEMAP_BYTES_PER_TILE_ESTIMATE);
    let terrain_bytes = terrain_tiles.saturating_mul(TERRAIN_BYTES_PER_TILE_ESTIMATE);
    Ok(DownloadEstimate {
        basemap_tiles,
        terrain_tiles,
        basemap_bytes,
        terrain_bytes,
        total_bytes: basemap_bytes.saturating_add(terrain_bytes),
        available_bytes: available_space(root)?,
    })
}

pub async fn download_region<F>(
    root: &Path,
    request: DownloadRequest,
    cancelled: Arc<AtomicBool>,
    progress: F,
) -> Result<RegionManifest, MapAssetError>
where
    F: Fn(DownloadEvent) + Send + Sync,
{
    request.validate()?;
    let mut request = request;
    request.bounds = request.bounds.validate()?;
    check_cancelled(&cancelled)?;
    ensure_maps_layout(root)?;
    let estimate = estimate_download(root, &request)?;
    if estimate.available_bytes < estimate.total_bytes.saturating_mul(2) {
        return Err(MapAssetError::InsufficientSpace);
    }
    progress(DownloadEvent::ResolvingSources {
        message: "Finding current basemap and terrain sources…".into(),
    });
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: format!(
            "Validated request '{}' for bounds {},{},{},{} (estimated {} bytes; {} bytes free).",
            request.name,
            request.bounds.west(),
            request.bounds.south(),
            request.bounds.east(),
            request.bounds.north(),
            estimate.total_bytes,
            estimate.available_bytes
        ),
    });
    let client = Client::builder()
        .user_agent("Waypoint-Map-Downloader/0.1")
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| MapAssetError::Provider(error.to_string()))?;
    check_cancelled(&cancelled)?;
    let (basemap_version, basemap_url) = resolve_basemap_source(&client, &progress).await?;
    check_cancelled(&cancelled)?;
    let terrain_source = resolve_terrain_source(&client, request.bounds, &progress).await?;
    check_cancelled(&cancelled)?;
    for existing in list_regions(root)? {
        if existing.bounds == request.bounds
            && existing.basemap.dataset_version == basemap_version
            && existing.terrain.dataset_version == terrain_source.version
            && verify_region(root, &existing.id).is_ok()
        {
            return Err(MapAssetError::RegionAlreadyExists(existing.id));
        }
    }
    let id = region_id(
        &request.name,
        request.bounds,
        &basemap_version,
        &terrain_source.version,
    )?;
    let regions_root = root.join("regions");
    let destination = regions_root.join(&id);
    if destination.exists() {
        if verify_region(root, &id).is_ok() {
            return Err(MapAssetError::RegionAlreadyExists(id));
        }
        return Err(MapAssetError::Provider(format!(
            "The destination {} exists but is incomplete. Remove it before retrying.",
            destination.display()
        )));
    }
    let staging = regions_root.join(format!(".{id}.download"));
    if staging.exists() {
        progress(DownloadEvent::Diagnostic {
            level: "warning".into(),
            message: "Removing a stale incomplete staging directory from an earlier attempt."
                .into(),
        });
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir(&staging)?;
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: format!("Created staging directory for region {id}."),
    });

    let result = async {
        let basemap_path = staging.join("basemap.pmtiles.part");
        download_basemap(
            &client,
            &basemap_url,
            request.bounds,
            &basemap_path,
            &cancelled,
            &progress,
        )
        .await?;
        progress(DownloadEvent::Verifying {
            layer: "basemap".into(),
        });
        progress(DownloadEvent::Diagnostic {
            level: "info".into(),
            message: "Validating the basemap PMTiles header.".into(),
        });
        validate_pmtiles_header(&basemap_path)?;
        let final_basemap = staging.join("basemap.pmtiles");
        fs::rename(&basemap_path, &final_basemap)?;

        let terrain_path = staging.join("terrain.pmtiles.part");
        download_terrain(
            &client,
            request.bounds,
            &terrain_path,
            &terrain_source,
            &cancelled,
            &progress,
        )
        .await?;
        progress(DownloadEvent::Verifying {
            layer: "terrain".into(),
        });
        progress(DownloadEvent::Diagnostic {
            level: "info".into(),
            message: "Validating the terrain PMTiles header.".into(),
        });
        validate_pmtiles_header(&terrain_path)?;
        let final_terrain = staging.join("terrain.pmtiles");
        fs::rename(&terrain_path, &final_terrain)?;

        let now = Utc::now();
        progress(DownloadEvent::Diagnostic {
            level: "info".into(),
            message: "Calculating final archive sizes and SHA-256 hashes.".into(),
        });
        let manifest = RegionManifest {
            format_version: MANIFEST_VERSION,
            id: id.clone(),
            name: request.name.trim().to_string(),
            bounds: request.bounds,
            basemap: RegionAsset {
                file: "basemap.pmtiles".into(),
                provider: "Protomaps".into(),
                dataset_version: basemap_version,
                source_url: basemap_url,
                tile_type: "mvt".into(),
                terrain_encoding: None,
                min_zoom: 0,
                max_zoom: BASEMAP_MAX_ZOOM,
                size_bytes: fs::metadata(&final_basemap)?.len(),
                sha256: sha256_file(&final_basemap)?,
                attribution: BASEMAP_ATTRIBUTION.into(),
                license:
                    "Open Database License 1.0 (OpenStreetMap data); Protomaps distribution terms"
                        .into(),
            },
            terrain: RegionAsset {
                file: "terrain.pmtiles".into(),
                provider: "Mapterhorn".into(),
                dataset_version: terrain_source.version.clone(),
                source_url: "https://download.mapterhorn.com/download_urls.json".into(),
                tile_type: "webp".into(),
                terrain_encoding: Some("terrarium".into()),
                min_zoom: 0,
                max_zoom: terrain_source.max_zoom,
                size_bytes: fs::metadata(&final_terrain)?.len(),
                sha256: sha256_file(&final_terrain)?,
                attribution: TERRAIN_ATTRIBUTION.into(),
                license:
                    "Mapterhorn data terms and the licenses of its attributed elevation sources"
                        .into(),
            },
            created_at: now,
            verified_at: now,
        };
        progress(DownloadEvent::Diagnostic {
            level: "info".into(),
            message: "Writing the region manifest and atomically installing the completed region."
                .into(),
        });
        write_manifest(&staging.join("manifest.json"), &manifest)?;
        fs::rename(&staging, &destination)?;
        progress(DownloadEvent::Complete {
            region_path: format!("maps/regions/{id}"),
        });
        Ok(manifest)
    }
    .await;

    if result.is_err() && staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

async fn resolve_basemap_source<F>(
    client: &Client,
    progress: &F,
) -> Result<(String, String), MapAssetError>
where
    F: Fn(DownloadEvent) + Send + Sync,
{
    for days_ago in 0..8 {
        let date = Utc::now().date_naive() - chrono::Duration::days(days_ago);
        let version = date.format("%Y%m%d").to_string();
        let url = format!("https://build.protomaps.com/{version}.pmtiles");
        progress(DownloadEvent::Diagnostic {
            level: "info".into(),
            message: format!("Probing Protomaps daily build {version}."),
        });
        let response = client.get(&url).header(RANGE, "bytes=0-7").send().await;
        match response {
            Ok(response) if response.status().is_success() => {
                progress(DownloadEvent::Diagnostic {
                    level: "info".into(),
                    message: format!(
                        "Selected Protomaps build {version} (HTTP {}).",
                        response.status()
                    ),
                });
                return Ok((version, url));
            }
            Ok(response) => progress(DownloadEvent::Diagnostic {
                level: "warning".into(),
                message: format!(
                    "Protomaps build {version} returned HTTP {}; trying an earlier build.",
                    response.status()
                ),
            }),
            Err(error) => progress(DownloadEvent::Diagnostic {
                level: "warning".into(),
                message: format!(
                    "Protomaps build {version} probe failed: {error}; trying an earlier build."
                ),
            }),
        }
    }
    Err(MapAssetError::Provider(
        "No current Protomaps daily build was reachable.".into(),
    ))
}

async fn resolve_terrain_source<F>(
    client: &Client,
    bounds: GeoBounds,
    progress: &F,
) -> Result<ResolvedTerrainSource, MapAssetError>
where
    F: Fn(DownloadEvent) + Send + Sync,
{
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: "Requesting Mapterhorn's machine-readable terrain archive list.".into(),
    });
    let response = client
        .get("https://download.mapterhorn.com/download_urls.json")
        .send()
        .await
        .map_err(|error| MapAssetError::Provider(error.to_string()))?
        .error_for_status()
        .map_err(|error| MapAssetError::Provider(error.to_string()))?;
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: format!(
            "Mapterhorn archive list responded with HTTP {}.",
            response.status()
        ),
    });
    let list: TerrainArchiveList = response
        .json()
        .await
        .map_err(|error| MapAssetError::Provider(error.to_string()))?;
    let archives = list
        .items
        .into_iter()
        .filter(|archive| archive.min_zoom <= TERRAIN_MAX_ZOOM && archive.intersects(bounds))
        .collect::<Vec<_>>();
    if !archives
        .iter()
        .any(|archive| archive.name == "planet.pmtiles")
    {
        return Err(MapAssetError::Provider(
            "Mapterhorn's global terrain archive was not listed.".into(),
        ));
    }
    let max_zoom = archives
        .iter()
        .map(|archive| archive.max_zoom)
        .max()
        .unwrap_or(12)
        .min(TERRAIN_MAX_ZOOM);
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: format!(
            "Selected Mapterhorn dataset {} with {} intersecting archive(s), through zoom {}.",
            list.version,
            archives.len(),
            max_zoom
        ),
    });
    Ok(ResolvedTerrainSource {
        version: list.version,
        max_zoom,
        archives,
    })
}

async fn download_basemap<F>(
    client: &Client,
    url: &str,
    bounds: GeoBounds,
    path: &Path,
    cancelled: &Arc<AtomicBool>,
    progress: &F,
) -> Result<(), MapAssetError>
where
    F: Fn(DownloadEvent) + Send + Sync,
{
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: "Opening the remote Protomaps PMTiles archive.".into(),
    });
    let reader =
        AsyncPmTilesReader::new_with_cached_url(HashMapCache::default(), client.clone(), url)
            .await
            .map_err(|error| MapAssetError::Provider(error.to_string()))?;
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: "Remote Protomaps archive opened; creating the local basemap extract.".into(),
    });
    let file = File::create(path)?;
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(0)
        .max_zoom(BASEMAP_MAX_ZOOM)
        .bounds(bounds.west(), bounds.south(), bounds.east(), bounds.north())
        .center(
            (bounds.west() + bounds.east()) / 2.0,
            (bounds.south() + bounds.north()) / 2.0,
        )
        .center_zoom(BASEMAP_MAX_ZOOM.min(12))
        .create(file)
        .map_err(|error| MapAssetError::InvalidArchive(error.to_string()))?;
    let coordinates = tile_coordinates(bounds, BASEMAP_MAX_ZOOM);
    let total_tiles = coordinates.len() as u64;
    progress(DownloadEvent::LayerStarted {
        layer: "basemap".into(),
        total_tiles,
    });
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: format!("Basemap extraction queued {total_tiles} tile coordinates."),
    });
    let mut downloaded_bytes = 0_u64;
    let mut completed = 0_u64;
    let results = stream::iter(coordinates.into_iter().map(|coordinate| {
        let reader = &reader;
        async move {
            let tile =
                retry_pmtiles_tile(reader, coordinate, cancelled, progress, "basemap").await?;
            Ok::<_, MapAssetError>((coordinate, tile))
        }
    }))
    .buffered(DOWNLOAD_CONCURRENCY);
    futures_util::pin_mut!(results);
    while let Some(result) = results.next().await {
        check_cancelled(cancelled)?;
        let (coordinate, tile) = result?;
        if let Some(tile) = tile {
            downloaded_bytes = downloaded_bytes.saturating_add(tile.len() as u64);
            writer
                .add_tile(coordinate, &tile)
                .map_err(|error| MapAssetError::InvalidArchive(error.to_string()))?;
        }
        completed += 1;
        if completed % 32 == 0 || completed == total_tiles {
            progress(DownloadEvent::Progress {
                layer: "basemap".into(),
                completed_tiles: completed,
                total_tiles,
                downloaded_bytes,
            });
        }
    }
    writer
        .finalize()
        .map_err(|error| MapAssetError::InvalidArchive(error.to_string()))?;
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: format!(
            "Finalized basemap extract: {completed} coordinates processed, {downloaded_bytes} tile bytes written."
        ),
    });
    Ok(())
}

async fn download_terrain<F>(
    client: &Client,
    bounds: GeoBounds,
    path: &Path,
    source: &ResolvedTerrainSource,
    cancelled: &Arc<AtomicBool>,
    progress: &F,
) -> Result<(), MapAssetError>
where
    F: Fn(DownloadEvent) + Send + Sync,
{
    let file = File::create(path)?;
    let mut writer = PmTilesWriter::new(TileType::Webp)
        .min_zoom(0)
        .max_zoom(source.max_zoom)
        .bounds(bounds.west(), bounds.south(), bounds.east(), bounds.north())
        .center(
            (bounds.west() + bounds.east()) / 2.0,
            (bounds.south() + bounds.north()) / 2.0,
        )
        .center_zoom(source.max_zoom.min(12))
        .create(file)
        .map_err(|error| MapAssetError::InvalidArchive(error.to_string()))?;
    let mut readers = Vec::with_capacity(source.archives.len());
    for archive in &source.archives {
        check_cancelled(cancelled)?;
        progress(DownloadEvent::Diagnostic {
            level: "info".into(),
            message: format!("Opening Mapterhorn archive {}.", archive.name),
        });
        let reader = AsyncPmTilesReader::new_with_cached_url(
            HashMapCache::default(),
            client.clone(),
            &archive.url,
        )
        .await
        .map_err(|error| {
            MapAssetError::Provider(format!("Could not open {}: {error}", archive.name))
        })?;
        progress(DownloadEvent::Diagnostic {
            level: "info".into(),
            message: format!("Opened Mapterhorn archive {}.", archive.name),
        });
        readers.push((archive, reader));
    }
    let coordinates = tile_coordinates(bounds, source.max_zoom);
    let total_tiles = coordinates.len() as u64;
    progress(DownloadEvent::LayerStarted {
        layer: "terrain".into(),
        total_tiles,
    });
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: format!("Terrain extraction queued {total_tiles} tile coordinates."),
    });
    let mut downloaded_bytes = 0_u64;
    let mut completed = 0_u64;
    let results = stream::iter(coordinates.into_iter().map(|coordinate| {
        let readers = &readers;
        async move {
            for (archive, reader) in readers {
                if archive.covers_tile(coordinate) {
                    if let Some(tile) =
                        retry_pmtiles_tile(reader, coordinate, cancelled, progress, "terrain")
                            .await?
                    {
                        return Ok::<_, MapAssetError>((coordinate, Some(tile)));
                    }
                }
            }
            Ok::<_, MapAssetError>((coordinate, None))
        }
    }))
    .buffered(DOWNLOAD_CONCURRENCY);
    futures_util::pin_mut!(results);
    while let Some(result) = results.next().await {
        check_cancelled(cancelled)?;
        let (coordinate, tile) = result?;
        if let Some(tile) = tile {
            downloaded_bytes = downloaded_bytes.saturating_add(tile.len() as u64);
            writer
                .add_tile(coordinate, &tile)
                .map_err(|error| MapAssetError::InvalidArchive(error.to_string()))?;
        }
        completed += 1;
        if completed % 32 == 0 || completed == total_tiles {
            progress(DownloadEvent::Progress {
                layer: "terrain".into(),
                completed_tiles: completed,
                total_tiles,
                downloaded_bytes,
            });
        }
    }
    writer
        .finalize()
        .map_err(|error| MapAssetError::InvalidArchive(error.to_string()))?;
    progress(DownloadEvent::Diagnostic {
        level: "info".into(),
        message: format!(
            "Finalized terrain extract: {completed} coordinates processed, {downloaded_bytes} tile bytes written."
        ),
    });
    Ok(())
}

async fn retry_pmtiles_tile<B, C, F>(
    reader: &AsyncPmTilesReader<B, C>,
    coordinate: TileCoord,
    cancelled: &AtomicBool,
    progress: &F,
    layer: &str,
) -> Result<Option<bytes::Bytes>, MapAssetError>
where
    B: AsyncBackend + Sync + Send,
    C: DirectoryCache + Sync + Send,
    F: Fn(DownloadEvent) + Send + Sync,
{
    let mut last_error = None;
    for attempt in 0..3 {
        check_cancelled(cancelled)?;
        match reader.get_tile_decompressed(coordinate).await {
            Ok(tile) => return Ok(tile),
            Err(error) => {
                let message = error.to_string();
                progress(DownloadEvent::Diagnostic {
                    level: "warning".into(),
                    message: format!(
                        "{layer} tile z{}/x{}/y{} failed on attempt {}/3: {message}",
                        coordinate.z(),
                        coordinate.x(),
                        coordinate.y(),
                        attempt + 1
                    ),
                });
                last_error = Some(message);
            }
        }
        tokio::time::sleep(Duration::from_millis(250 * (attempt + 1))).await;
    }
    Err(MapAssetError::Provider(
        last_error.unwrap_or_else(|| "Tile request failed.".into()),
    ))
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), MapAssetError> {
    if cancelled.load(Ordering::Relaxed) {
        Err(MapAssetError::Cancelled)
    } else {
        Ok(())
    }
}

pub fn tile_coordinates(bounds: GeoBounds, max_zoom: u8) -> Vec<TileCoord> {
    let mut coordinates = Vec::new();
    for zoom in 0..=max_zoom {
        if let Some(range) = tile_range(bounds, zoom) {
            for x in range.min_x..=range.max_x {
                for y in range.min_y..=range.max_y {
                    if let Ok(coordinate) = TileCoord::new(zoom, x, y) {
                        coordinates.push(coordinate);
                    }
                }
            }
        }
    }
    coordinates
}

pub fn maps_root() -> Result<PathBuf, MapAssetError> {
    if let Some(configured) = env::var_os("WAYPOINT_MAPS_DIR") {
        if !configured.is_empty() {
            return Ok(PathBuf::from(configured));
        }
    }
    let executable = env::current_exe()?;
    if let Some(parent) = executable.parent() {
        let adjacent = parent.join("maps");
        if adjacent.exists() {
            return Ok(adjacent);
        }
    }
    Ok(env::current_dir()?.join("maps"))
}

pub fn ensure_maps_layout(root: &Path) -> Result<(), MapAssetError> {
    fs::create_dir_all(root.join("regions"))?;
    Ok(())
}

pub fn region_id(
    name: &str,
    bounds: GeoBounds,
    basemap_version: &str,
    terrain_version: &str,
) -> Result<String, MapAssetError> {
    let slug = slugify(name)?;
    let canonical = format!(
        "{:.7},{:.7},{:.7},{:.7}|{basemap_version}|{terrain_version}",
        bounds.west(),
        bounds.south(),
        bounds.east(),
        bounds.north()
    );
    let digest = Sha256::digest(canonical.as_bytes());
    Ok(format!("{slug}-{}", &hex::encode(digest)[..8]))
}

pub fn slugify(name: &str) -> Result<String, MapAssetError> {
    let mut slug = String::new();
    let mut pending_dash = false;
    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character.to_ascii_lowercase());
            pending_dash = false;
        } else if !slug.is_empty() {
            pending_dash = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }
    if slug.is_empty() {
        return Err(MapAssetError::InvalidName);
    }
    Ok(slug)
}

pub fn list_regions(root: &Path) -> Result<Vec<RegionManifest>, MapAssetError> {
    let regions_root = root.join("regions");
    if !regions_root.exists() {
        return Ok(Vec::new());
    }
    let mut regions = Vec::new();
    for entry in fs::read_dir(regions_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("manifest.json");
        if !path.is_file() {
            continue;
        }
        if let Ok(manifest) = read_manifest(&path) {
            regions.push(manifest);
        }
    }
    regions.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(regions)
}

pub fn find_covering_region(
    root: &Path,
    bounds: GeoBounds,
) -> Result<Option<RegionManifest>, MapAssetError> {
    let bounds = bounds.validate()?;
    let mut matches = list_regions(root)?
        .into_iter()
        .filter(|region| region.bounds.contains(bounds) && verify_region(root, &region.id).is_ok())
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        left.bounds
            .area()
            .partial_cmp(&right.bounds.area())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(matches.into_iter().next())
}

pub fn read_manifest(path: &Path) -> Result<RegionManifest, MapAssetError> {
    let source = fs::read_to_string(path)?;
    let manifest: RegionManifest = serde_json::from_str(&source)?;
    if manifest.format_version != MANIFEST_VERSION {
        return Err(MapAssetError::UnsupportedManifest(manifest.format_version));
    }
    manifest.bounds.validate()?;
    validate_region_id(&manifest.id)?;
    Ok(manifest)
}

pub fn write_manifest(path: &Path, manifest: &RegionManifest) -> Result<(), MapAssetError> {
    let source = serde_json::to_vec_pretty(manifest)?;
    fs::write(path, source)?;
    Ok(())
}

pub fn verify_region(root: &Path, region_id: &str) -> Result<RegionManifest, MapAssetError> {
    validate_region_id(region_id)?;
    let region_path = root.join("regions").join(region_id);
    let manifest = read_manifest(&region_path.join("manifest.json"))?;
    if manifest.id != region_id {
        return Err(MapAssetError::InvalidRegionId);
    }
    for (kind, asset) in [
        (AssetKind::Basemap, &manifest.basemap),
        (AssetKind::Terrain, &manifest.terrain),
    ] {
        if asset.file != kind.filename() {
            return Err(MapAssetError::InvalidArchive(asset.file.clone()));
        }
        let path = region_path.join(kind.filename());
        validate_pmtiles_header(&path)?;
        let metadata = fs::metadata(&path)?;
        if metadata.len() != asset.size_bytes {
            return Err(MapAssetError::InvalidArchive(format!(
                "{} size does not match its manifest",
                kind.filename()
            )));
        }
        if sha256_file(&path)? != asset.sha256 {
            return Err(MapAssetError::InvalidArchive(format!(
                "{} checksum does not match its manifest",
                kind.filename()
            )));
        }
    }
    Ok(manifest)
}

pub fn read_asset_range(
    root: &Path,
    region_id: &str,
    kind: AssetKind,
    offset: u64,
    length: usize,
) -> Result<Vec<u8>, MapAssetError> {
    validate_region_id(region_id)?;
    if length == 0 || length > 16 * 1024 * 1024 {
        return Err(MapAssetError::InvalidArchive(
            "Requested byte range is invalid.".into(),
        ));
    }
    let region_path = root.join("regions").join(region_id);
    let manifest = read_manifest(&region_path.join("manifest.json"))?;
    if manifest.id != region_id {
        return Err(MapAssetError::InvalidRegionId);
    }
    let asset = match kind {
        AssetKind::Basemap => &manifest.basemap,
        AssetKind::Terrain => &manifest.terrain,
    };
    if asset.file != kind.filename() {
        return Err(MapAssetError::InvalidArchive(asset.file.clone()));
    }
    let path = region_path.join(kind.filename());
    let file_length = fs::metadata(&path)?.len();
    let end = offset
        .checked_add(length as u64)
        .ok_or_else(|| MapAssetError::InvalidArchive("Requested byte range overflowed.".into()))?;
    if end > file_length {
        return Err(MapAssetError::InvalidArchive(
            "Requested byte range is outside the archive.".into(),
        ));
    }
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

pub fn validate_pmtiles_header(path: &Path) -> Result<(), MapAssetError> {
    let mut file = File::open(path)?;
    if file.metadata()?.len() < 127 {
        return Err(MapAssetError::InvalidArchive(path.display().to_string()));
    }
    let mut header = [0_u8; 127];
    file.read_exact(&mut header)?;
    let header = Header::try_from_bytes(bytes::Bytes::copy_from_slice(&header))
        .map_err(|error| MapAssetError::InvalidArchive(format!("{}: {error}", path.display())))?;
    if header.spec_version() != 3
        || header.min_zoom > header.max_zoom
        || header.tile_type == TileType::Unknown
    {
        return Err(MapAssetError::InvalidArchive(path.display().to_string()));
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String, MapAssetError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn tile_count(bounds: GeoBounds, max_zoom: u8) -> u64 {
    (0..=max_zoom)
        .map(|zoom| tile_range(bounds, zoom).map_or(0, |range| range.count()))
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRange {
    pub zoom: u8,
    pub min_x: u32,
    pub max_x: u32,
    pub min_y: u32,
    pub max_y: u32,
}

impl TileRange {
    pub fn count(self) -> u64 {
        u64::from(self.max_x - self.min_x + 1) * u64::from(self.max_y - self.min_y + 1)
    }
}

pub fn tile_range(bounds: GeoBounds, zoom: u8) -> Option<TileRange> {
    if bounds.validate_without_size().is_err() || zoom > 30 {
        return None;
    }
    let scale = (1_u64 << zoom) as f64;
    let x = |longitude: f64| ((longitude + 180.0) / 360.0 * scale).floor() as i64;
    let y = |latitude: f64| {
        let latitude = latitude.to_radians();
        ((1.0 - (latitude.tan() + 1.0 / latitude.cos()).ln() / std::f64::consts::PI) / 2.0 * scale)
            .floor() as i64
    };
    let maximum = (scale as i64 - 1).max(0);
    Some(TileRange {
        zoom,
        min_x: x(bounds.west()).clamp(0, maximum) as u32,
        max_x: x(bounds.east()).clamp(0, maximum) as u32,
        min_y: y(bounds.north()).clamp(0, maximum) as u32,
        max_y: y(bounds.south()).clamp(0, maximum) as u32,
    })
}

impl GeoBounds {
    fn validate_without_size(self) -> Result<Self, MapAssetError> {
        if ![self.0, self.1, self.2, self.3]
            .iter()
            .all(|value| value.is_finite())
            || self.west() < -180.0
            || self.east() > 180.0
            || self.south() < -WEB_MERCATOR_MAX_LATITUDE
            || self.north() > WEB_MERCATOR_MAX_LATITUDE
            || self.west() >= self.east()
            || self.south() >= self.north()
        {
            return Err(MapAssetError::InvalidBounds("Invalid bounds.".into()));
        }
        Ok(self)
    }
}

fn validate_region_id(value: &str) -> Result<(), MapAssetError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.len() > 80
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || !value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
    {
        return Err(MapAssetError::InvalidRegionId);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root(label: &str) -> PathBuf {
        let root = env::temp_dir().join(format!(
            "waypoint-map-assets-{label}-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("regions")).unwrap();
        root
    }

    fn write_archive(path: &Path, tile_type: TileType) {
        let file = File::create(path).unwrap();
        let mut writer = PmTilesWriter::new(tile_type).create(file).unwrap();
        writer
            .add_tile(TileCoord::new(0, 0, 0).unwrap(), &[1, 2, 3, 4])
            .unwrap();
        writer.finalize().unwrap();
    }

    fn install_fixture_region(root: &Path, id: &str, name: &str, bounds: GeoBounds) {
        let path = root.join("regions").join(id);
        fs::create_dir(&path).unwrap();
        let basemap_path = path.join("basemap.pmtiles");
        let terrain_path = path.join("terrain.pmtiles");
        write_archive(&basemap_path, TileType::Mvt);
        write_archive(&terrain_path, TileType::Webp);
        let asset =
            |file: &str, path: &Path, tile_type: &str, encoding: Option<&str>| RegionAsset {
                file: file.into(),
                provider: "fixture".into(),
                dataset_version: "1".into(),
                source_url: "https://example.invalid/fixture".into(),
                tile_type: tile_type.into(),
                terrain_encoding: encoding.map(str::to_string),
                min_zoom: 0,
                max_zoom: 0,
                size_bytes: fs::metadata(path).unwrap().len(),
                sha256: sha256_file(path).unwrap(),
                attribution: "Fixture".into(),
                license: "Test fixture".into(),
            };
        let now = Utc::now();
        write_manifest(
            &path.join("manifest.json"),
            &RegionManifest {
                format_version: MANIFEST_VERSION,
                id: id.into(),
                name: name.into(),
                bounds,
                basemap: asset("basemap.pmtiles", &basemap_path, "mvt", None),
                terrain: asset("terrain.pmtiles", &terrain_path, "webp", Some("terrarium")),
                created_at: now,
                verified_at: now,
            },
        )
        .unwrap();
    }

    #[test]
    fn parses_and_validates_standard_bounds() {
        let bounds = GeoBounds::parse("-121.95,35.95,-121.55,36.35").unwrap();
        assert_eq!(bounds, GeoBounds(-121.95, 35.95, -121.55, 36.35));
        assert!(GeoBounds::parse("121,10,-121,12").is_err());
        assert_eq!(
            GeoBounds::parse("-10,-90,-9.999,-85.04").unwrap().south(),
            -WEB_MERCATOR_MAX_LATITUDE
        );
        assert!(GeoBounds::parse("-180,-85,180,85").is_err());
    }

    #[test]
    fn creates_stable_safe_region_ids() {
        let bounds = GeoBounds(-121.95, 35.95, -121.55, 36.35);
        let first = region_id("Big Sur Ride", bounds, "20260825", "0.0.12").unwrap();
        let second = region_id("Big Sur Ride", bounds, "20260825", "0.0.12").unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("big-sur-ride-"));
        assert_eq!(first.len(), "big-sur-ride-".len() + 8);
    }

    #[test]
    fn counts_tiles_and_finds_covering_region() {
        let bounds = GeoBounds(-121.95, 35.95, -121.55, 36.35);
        assert!(tile_count(bounds, 15) > tile_count(bounds, 10));
        let range = tile_range(bounds, 12).unwrap();
        assert!(range.count() > 0);
    }

    #[test]
    fn resolves_environment_override() {
        let temporary = env::temp_dir();
        env::set_var("WAYPOINT_MAPS_DIR", &temporary);
        assert_eq!(maps_root().unwrap(), temporary);
        env::remove_var("WAYPOINT_MAPS_DIR");
    }

    #[test]
    fn verifies_manifests_and_selects_the_smallest_covering_region() {
        let root = fixture_root("coverage");
        install_fixture_region(
            &root,
            "large-region-bbbbbbbb",
            "Large",
            GeoBounds(-123.0, 35.0, -120.0, 38.0),
        );
        install_fixture_region(
            &root,
            "small-region-aaaaaaaa",
            "Small",
            GeoBounds(-122.0, 36.0, -121.0, 37.0),
        );

        let activity = GeoBounds(-121.8, 36.2, -121.2, 36.8);
        let selected = find_covering_region(&root, activity).unwrap().unwrap();
        assert_eq!(selected.id, "small-region-aaaaaaaa");
        assert_eq!(
            read_asset_range(&root, &selected.id, AssetKind::Basemap, 0, 8).unwrap(),
            b"PMTiles\x03"
        );

        fs::write(
            root.join("regions/small-region-aaaaaaaa/basemap.pmtiles"),
            b"corrupt",
        )
        .unwrap();
        let fallback = find_covering_region(&root, activity).unwrap().unwrap();
        assert_eq!(fallback.id, "large-region-bbbbbbbb");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rejects_pre_cancelled_downloads_without_network_or_staging_files() {
        let root = fixture_root("cancelled");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(download_region(
            &root,
            DownloadRequest {
                name: "Cancelled fixture".into(),
                bounds: GeoBounds(-121.8, 36.27, -121.79, 36.28),
            },
            Arc::new(AtomicBool::new(true)),
            |_| {},
        ));
        assert!(matches!(result, Err(MapAssetError::Cancelled)));
        assert_eq!(fs::read_dir(root.join("regions")).unwrap().count(), 0);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    #[ignore = "contacts the live Protomaps and Mapterhorn services"]
    fn downloads_and_atomically_installs_a_tiny_live_region() {
        let root = fixture_root("live-download");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let manifest = runtime
            .block_on(download_region(
                &root,
                DownloadRequest {
                    name: "Live fixture".into(),
                    bounds: GeoBounds(-121.8000, 36.2700, -121.7999, 36.2701),
                },
                Arc::new(AtomicBool::new(false)),
                |_| {},
            ))
            .unwrap();
        assert!(root
            .join("regions")
            .join(&manifest.id)
            .join("manifest.json")
            .is_file());
        verify_region(&root, &manifest.id).unwrap();
        let region_path = root.join("regions").join(&manifest.id);
        runtime.block_on(async {
            let basemap = AsyncPmTilesReader::new_with_path(region_path.join("basemap.pmtiles"))
                .await
                .unwrap();
            let basemap_coordinate = tile_coordinates(manifest.bounds, BASEMAP_MAX_ZOOM)
                .into_iter()
                .find(|coordinate| coordinate.z() == BASEMAP_MAX_ZOOM)
                .unwrap();
            assert!(!basemap
                .get_tile_decompressed(basemap_coordinate)
                .await
                .unwrap()
                .unwrap()
                .is_empty());

            let terrain = AsyncPmTilesReader::new_with_path(region_path.join("terrain.pmtiles"))
                .await
                .unwrap();
            let terrain_coordinate = tile_coordinates(manifest.bounds, manifest.terrain.max_zoom)
                .into_iter()
                .find(|coordinate| coordinate.z() == manifest.terrain.max_zoom)
                .unwrap();
            assert!(!terrain
                .get_tile_decompressed(terrain_coordinate)
                .await
                .unwrap()
                .unwrap()
                .is_empty());
        });
        assert!(!root
            .join("regions")
            .join(format!(".{}.download", manifest.id))
            .exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
