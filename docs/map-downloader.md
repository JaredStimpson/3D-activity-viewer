# Waypoint Map Downloader

Map Downloader is a small companion PC app whose only job is to install offline basemap and 3D terrain data for Waypoint.

## Download a region

1. Launch **Run Map Downloader from Source.cmd**, or build once and use **Launch Map Downloader.cmd**.
2. Enter a descriptive area name.
3. Paste an EPSG:4326 bounding box in `west,south,east,north` order, for example:

   ```text
   -121.95,35.95,-121.55,36.35
   ```

4. Select **Estimate download**. Review the basemap estimate, terrain estimate, and available space.
5. Select **Download Map Data** and leave the app open until verification finishes.
6. In Waypoint, select **Refresh map data**.

Version 1 accepts ordinary rectangles that do not cross the antimeridian. Latitude is constrained to the Web Mercator range, reversed and zero-area bounds are rejected, and very large requests are rejected before a destination is created.

## What is downloaded

Standard quality is fixed:

- Protomaps vector basemap tiles through zoom 15.
- Mapterhorn Terrarium PMTiles extracts through zoom 14 where the machine-readable archive list provides coverage, falling back to the highest available zoom.

The downloader freezes the source versions at job start, reads only the byte ranges and tiles needed from the dated Protomaps build and intersecting Mapterhorn PMTiles archives, retries transient requests up to three times, validates the PMTiles headers, records SHA-256 hashes and licenses, then installs the manifest last. Cancelling removes incomplete `.part` data. A completed region is never silently overwritten.

## Live diagnostics

The downloader starts a local terminal-style diagnostics page with the application. Select **Live diagnostics** or **Open readout** to open it in the default browser.

- The preferred address is `http://127.0.0.1:4765/`.
- If that port is already occupied, the downloader tries ports 4766 through 4774 and then an available dynamic port. The exact active address is always shown in the downloader window.
- The page refreshes once per second. Its `/logs` endpoint provides the same output as plain text.
- Entries include elapsed timestamps, source-version probes, provider HTTP results, archive opening, tile progress and retry failures, PMTiles finalization, verification, hashing, installation, cancellation, and the final error.
- The server listens only on the local loopback interface. It is not reachable from another computer and stops when the downloader closes.
- Logs are held in memory, limited to the most recent 4,000 lines, and are not written to disk. Source launches also print the same entries in their terminal window.

If a download appears stuck, leave the diagnostics page open and note the last line plus its elapsed timestamp. A retry can legitimately wait for the provider HTTP timeout, but the last line identifies which provider, archive, or tile operation is waiting.

Each completed region is stored as:

```text
maps\regions\<area-name>-<bounds-hash>\
  manifest.json
  basemap.pmtiles
  terrain.pmtiles
```

Downloaded region content is Git-ignored. `maps/README.md`, the manifest schema, and `maps/regions/.gitkeep` remain tracked.

## Where the 3D comes from

`basemap.pmtiles` provides roads, water, land, labels, and building attributes. It does not create terrain. `terrain.pmtiles` contains Terrarium-encoded elevation pixels; MapLibre turns that raster DEM into the raised terrain mesh. Activity elevation is still used for route statistics and is separate from the terrain surface.

## Map folder resolution

Both apps resolve the same map root in this order:

1. `WAYPOINT_MAPS_DIR`.
2. A `maps` folder beside the executable.
3. The repository `maps` folder used by source launchers.

Projects and manifests store region IDs and relative filenames, never machine-specific absolute paths.
