# Offline map notices

Waypoint displays these notices without requiring network access. Each downloaded `manifest.json` also records the exact provider version, source URL, attribution, and license description used for that region.

## Protomaps basemap

- Map data: © OpenStreetMap contributors.
- Basemap tiles are distributed by Protomaps as an Open Database License 1.0 Produced Work. OpenStreetMap attribution is required.
- The bundled `@protomaps/basemaps` style code is BSD-3-Clause licensed.
- License references: <https://docs.protomaps.com/basemaps/downloads>, <https://www.openstreetmap.org/copyright>, and <https://github.com/protomaps/basemaps>.

## Mapterhorn terrain

- Terrain tiles: Mapterhorn and its attributed upstream elevation sources.
- Mapterhorn aggregates several elevation datasets; their provider-specific attribution and terms remain applicable.
- Attribution and data access references: <https://mapterhorn.com/attribution/> and <https://mapterhorn.com/data-access/>.

## Rendering libraries

- MapLibre GL JS: BSD-3-Clause.
- PMTiles JavaScript and Rust libraries: BSD-3-Clause or the license declared by the locked package version.

See the root `LICENSE`, `pnpm-lock.yaml`, and `Cargo.lock` for the application license and exact dependency versions.
