# Waypoint map data

Waypoint and Waypoint Map Downloader share this folder when they run from the repository.

Downloaded regions are stored under `regions/<area-name>-<bounds-hash>/` and are intentionally ignored by Git. Each complete region contains:

```text
manifest.json
basemap.pmtiles
terrain.pmtiles
```

The downloader writes temporary `.part` files and exposes a region only after both archives pass verification and `manifest.json` is written. Do not commit downloaded map archives: they can be large and are regeneratable.

Bundled offline attribution and license notices live in `licenses/README.md`; each region manifest also records provider-specific notices and source versions.

## Area coordinates

Enter bounds in WGS84 longitude/latitude order:

```text
west,south,east,north
```

Example:

```text
-121.95,35.95,-121.55,36.35
```

Version 1 accepts ordinary rectangles that do not cross the antimeridian. Latitude is clamped to the Web Mercator limit, and invalid, zero-area, reversed, or excessively large requests are rejected before download files are created.
