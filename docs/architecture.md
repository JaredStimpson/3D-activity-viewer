# Architecture

Waypoint separates map acquisition from rendering. The main application never downloads geographic data.

```text
bounds -> Map Downloader -> map-assets -> maps/regions/<region-id>
                                             │
GPX -> activity-core -> shared scene evaluator
                            │                │
                            └────── MapLibre preview/export
                                             │ raw RGBA binary IPC
                                             v
                                      render-core / FFmpeg
                                             │
                                             v
                                      verified local MP4
```

`map-assets` is shared by both Tauri applications. It owns bounding-box validation, deterministic region IDs, provider version resolution, PMTiles creation and validation, manifests, coverage selection, safe byte ranges, hashing, cancellation, and atomic region installation.

The main app asks Rust for the smallest verified region that fully covers the activity. The frontend receives only a manifest and region ID. PMTiles requests specify that validated ID, an asset kind, offset, and length; arbitrary frontend filesystem access is not exposed.

MapLibre uses the vector basemap for roads, land, water, labels, and building data and a Terrarium `raster-dem` archive for hillshade and raised terrain. Preview and export share `apps/desktop/src/lib/scene.ts`; wall-clock time never participates in export evaluation.

Core crates remain independent of Tauri and React. `render-core` owns the checked FFmpeg frame session, temporary output, final verification, and atomic publication.
