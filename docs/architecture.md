# Architecture

Waypoint separates activity understanding, project state, scene evaluation, and rendering so each part can evolve independently.

```text
GPX source
   │
   ▼
activity-core ── normalized points, statistics, distance progress
   │
   ├────────────► React preview (interactive)
   │
   └────────────► render-core (deterministic frames)
                         │
                         ▼
                      FFmpeg
                         │
                         ▼
                  verified local MP4
```

The React layer owns editor interaction only. Rust owns source parsing, filesystem operations, project durability, and final render orchestration. Preview and export both evaluate progress by cumulative physical route distance.

## Current boundary

The technical proof uses a deterministic procedural terrain surface so it is fully local and carries no map-provider dependency. The next renderer milestone replaces that surface with MapLibre backed by installed PMTiles and terrain archives. The Asset Manager will supply local asset handles; the renderer will never download geography directly.

