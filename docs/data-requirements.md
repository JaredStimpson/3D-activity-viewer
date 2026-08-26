# Activity and geographic data requirements

This report defines the data Waypoint needs to turn an outdoor activity into an offline 3D route video. It also separates the data itself from the file formats and rendering software that carry or display it.

## Short answer: is it just PMTiles?

No. **PMTiles is a single-file archive format for tiled data; it is not the map, the elevation model, or the 3D renderer.** A complete offline 3D scene needs several layers:

1. Activity data describes where and when the person moved.
2. Basemap tiles provide roads, water, land, labels, and other visible context.
3. A digital elevation model (DEM) provides the height of the surrounding ground.
4. A map style, fonts, and icons decide how the map looks.
5. MapLibre turns the DEM into a terrain mesh and draws the map and route on it.
6. Waypoint supplies the camera and animation timeline, then FFmpeg encodes the frames as video.

PMTiles can package the basemap. A separate PMTiles archive can also package compatible DEM tiles. The important requirement for 3D terrain is the DEM content, not the `.pmtiles` extension.

## What comes from the activity file

### Required activity data

| Field | Why Waypoint needs it | GPX source |
| --- | --- | --- |
| Latitude and longitude | Draws the route and determines the geographic region to install | Track point attributes |
| Ordered track points | Preserves the path through the activity | Track/segment order |

A location-only activity can be displayed, but it cannot be synchronized reliably to real elapsed time unless timestamps are available.

### Strongly recommended activity data

| Field | Why it matters | Behavior when absent |
| --- | --- | --- |
| Timestamp per point | Duration, pace/speed, media synchronization, and time-based playback | Animate by distance with no trustworthy wall-clock timeline |
| Recorded elevation per point | Elevation profile, ascent/descent, and route altitude | Sample the terrain DEM as an estimate or omit elevation statistics |

Recorded GPX elevation describes the height **along the track only**. It cannot describe the valleys, ridges, and slopes around the track, so it is not enough to create a real terrain surface.

### Optional activity data

These fields improve overlays and storytelling but are not required to draw the route:

- activity name, description, sport/type, and device metadata;
- instantaneous speed or pace;
- heart rate;
- cycling cadence and power;
- temperature;
- laps, pauses, and segment boundaries.

GPX extensions vary by device. FIT and TCX support is planned but is not part of the current technical proof.

### Data derived by Waypoint

Waypoint should derive and cache regeneratable scene inputs rather than modify the source activity:

- cleaned, smoothed, resampled, and render-simplified track variants;
- cumulative distance for deterministic route progress;
- distance, duration, elevation gain/loss, and bounds;
- a padded coverage boundary large enough for pitched and moving cameras;
- stable heading/look-ahead data for camera motion;
- route progress, statistic events, and finish timing on the scene timeline.

## Extra geographic data

### 1. Basemap tiles

The basemap supplies visible geographic context such as:

- roads, trails, paths, railways, and boundaries;
- land, water, parks, land cover, and place labels;
- building footprints and attributes when the selected dataset includes them.

For the planned renderer, the preferred package is an offline vector-tile archive such as `basemap.pmtiles`. Vector tiles normally contain Mapbox Vector Tile (MVT) data. Raster imagery can also be tiled, but it is less flexible to restyle and does not by itself contain elevation.

### 2. Terrain elevation tiles

Real surrounding terrain requires a **digital elevation model** covering the full camera-visible region, not just the route line. MapLibre accepts a `raster-dem` source whose pixels encode elevations, including Mapbox Terrain RGB or Mapzen Terrarium encoding. It then creates a 3D terrain mesh from that source.

Recommended package choices are:

- `terrain.pmtiles` containing lossless PNG DEM tiles in a compatible encoding; or
- `terrain.mbtiles` containing the same kind of DEM tiles behind Waypoint's local asset interface.

The archive format is an implementation choice. Required terrain metadata includes:

- geographic bounds and minimum/maximum zoom;
- source dataset and license/attribution;
- horizontal resolution and elevation units;
- elevation encoding and no-data behavior;
- vertical datum, where known;
- package version, checksum, and installation state.

Lossless tiles are required for RGB-encoded DEM values; lossy JPEG compression would change pixel values and therefore corrupt elevations.

### 3. Style, labels, and icons

Vector tiles do not define their own finished appearance. An offline style package must include or reference only local resources:

- MapLibre style JSON;
- local glyph/font PBF ranges used for labels;
- sprite JSON and sprite PNG/WebP sheets for icons;
- the exact source-layer names expected by the style;
- attribution and license notices.

Without these assets the geometry may still exist, but labels, icons, or the intended visual design can be missing.

### 4. Optional 3D buildings and models

Terrain and buildings are separate sources of 3D geometry:

- **Terrain** comes from DEM elevation samples.
- **Buildings** come from vector polygons with height and optional minimum-height attributes. MapLibre can extrude them with a `fill-extrusion` layer.
- **Custom landmarks or objects** can later come from 3D model files rendered through a custom layer. They are not required for the first map renderer.

A basemap without building-height attributes can still produce real 3D terrain, but its buildings will remain flat unless Waypoint applies a fallback height.

## Where the 3D comes from

The target rendering sequence is:

```text
activity coordinates ───────────────► animated route line and marker
vector basemap tiles + style ───────► roads, land, water, labels, buildings
DEM terrain tiles ──────────────────► decoded elevation grid
                                           │
                                           ▼
                                 MapLibre terrain mesh
                                           │
camera pitch + bearing + perspective ──────┤
terrain exaggeration ──────────────────────┤
                                           ▼
                                 rendered 3D scene frames
                                           │
                                           ▼
                                      FFmpeg video
```

For each DEM tile, MapLibre decodes the elevation value represented by each pixel, constructs a triangulated ground mesh, and positions that mesh geographically. It then renders the styled map and animated route in the same geographic scene. Camera pitch, bearing, perspective, and optional terrain exaggeration make the elevation visible. Building extrusion or custom model layers can add above-ground objects.

### What the app does today

The supported desktop preview and export now read verified repo-local PMTiles through `map-assets`. MapLibre renders the Protomaps vector basemap and converts Mapterhorn Terrarium elevation pixels into the real terrain mesh. `apps/desktop/src/lib/scene.ts` evaluates camera and route state for both preview and export.

The procedural Rust renderer remains only as the `render-activity` command-line technical proof. It is not used by the main desktop export path.

## Recommended offline region package

One installed region should be a logical bundle, even when it contains more than one tile archive:

```text
regions/<region-id>/
├── manifest.json
├── basemap.pmtiles
└── terrain.pmtiles
```

The implementation uses separate basemap and terrain archives because a PMTiles archive declares one tile type while vector map data and raster DEM data require different decoding. Bundled Protomaps style code and locally generated glyphs avoid separate online style, sprite, or font requests.

The region manifest should record:

- region ID, human-readable name, bounds, and zoom coverage;
- paths or opaque local handles for every asset;
- tile types, DEM encoding, versions, byte sizes, and checksums;
- source attribution and license text;
- install status and last verification time.

The renderer receives a verified region ID and fixed asset kind. Rust validates the manifest and exposes bounded binary range reads; the frontend never receives an arbitrary filesystem path. The main app never fetches missing geography during preview or export.

## Other local runtime data

The application also needs local state and software that are not map tiles:

| Input or dependency | Purpose |
| --- | --- |
| Project JSON | Activity reference, style choices, camera preset/keyframes, timeline, output settings, media references, and region ID |
| Optional photos/video/audio | Media events, overlays, and soundtrack |
| Media metadata | Capture time, GPS coordinates, orientation, duration, and user-adjusted clock offset |
| Frozen render snapshot | Guarantees an export does not change when the editable project changes |
| MapLibre and a PMTiles adapter/local tile API | Reads local tile packages and renders the geographic scene |
| WebView2 and a working GPU path | Hosts and accelerates the desktop map renderer |
| FFmpeg and ffprobe | Encodes and verifies the final video |
| Temporary frame/cache space | Holds regeneratable render data without mixing it with final exports |

## Minimum useful data combinations

| Available data | Result |
| --- | --- |
| Activity coordinates only | Route animation on a plain or procedural background |
| Activity + basemap | Geographically correct flat map scene |
| Activity + basemap + DEM | Geographically correct 3D terrain scene |
| Above + building heights | 3D terrain with extruded buildings |
| Above + timestamps/media metadata | Time-synchronized photos, clips, metrics, and music events |

## Asset readiness checks

Before enabling preview or export for a real-map project, Waypoint should verify:

- the activity parses and has a usable geographic extent;
- the selected region covers the route plus its camera padding;
- both basemap and DEM archives open and contain the required zooms;
- the style references only installed sources, sprites, and glyphs;
- the DEM encoding matches the MapLibre source configuration;
- attribution and license metadata are present;
- checksums match and sufficient temporary/export disk space is available;
- FFmpeg, ffprobe, WebView2, and GPU rendering are available.

## Design decisions for the next renderer milestone

1. Treat PMTiles as a transport/storage container, not as the source of 3D.
2. Use independent basemap and terrain sources behind one verified `map-assets` region ID.
3. Require a real DEM for any feature labeled "real terrain" or "3D terrain."
4. Keep recorded activity elevation for statistics and route altitude; do not stretch it into surrounding terrain.
5. Keep map styling assets fully local so preview and export remain offline and reproducible.
6. Evaluate camera, route, terrain exaggeration, and overlays from one deterministic scene timeline shared by preview and export.

## Primary references

- [PMTiles v3 specification](https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md) — archive layout, tile types, and Terrarium DEM metadata
- [PMTiles MapLibre example](https://github.com/protomaps/PMTiles/blob/main/js/examples/maplibre.html) — registering the PMTiles protocol and using a PMTiles vector source
- [MapLibre style specification: sources](https://maplibre.org/maplibre-style-spec/sources/) — `vector`, `raster`, and RGB-encoded `raster-dem` sources
- [MapLibre RasterDEMTileSource API](https://maplibre.org/maplibre-gl-js/docs/API/classes/RasterDEMTileSource/) — DEM sources for hillshade and 3D terrain
- [MapLibre Map API](https://maplibre.org/maplibre-gl-js/docs/API/classes/Map/) — `setTerrain` and terrain configuration
- [MapLibre 3D buildings example](https://maplibre.org/maplibre-gl-js/docs/examples/display-buildings-in-3d/) — extrusion from building polygons and height attributes
- [MapLibre 3D models on terrain example](https://maplibre.org/maplibre-gl-js/docs/examples/adding-3d-models-using-threejs-on-terrain/) — optional custom model layer over DEM terrain
