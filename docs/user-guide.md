# User guide

## Prepare offline map data

Waypoint renders only from local map files. Before previewing an activity, use **Waypoint Map Downloader** to install a region that fully covers it. Enter a `west,south,east,north` bounding box; the downloader shows an estimate and stores the completed package under `maps\regions`.

See the [Map Downloader guide](map-downloader.md) for the exact workflow and an example.

## Import a GPX activity

1. Launch Waypoint and select **Import GPX**.
2. Choose a `.gpx` file containing at least two track points.
3. Watch the status bar for the imported activity and map coverage result.

Waypoint reads the source without changing it. Parsing, distance/elevation statistics, map discovery, preview, and export all happen locally.

If no installed region covers the route, Waypoint displays the required bounds. Open Map Downloader, install a covering rectangle, and select **Refresh map data** in Waypoint. The smallest verified region that fully covers the activity is selected automatically.

## Preview the activity

Use play/pause and the scrubber below the preview. The Inspector controls:

- **Map style:** Outdoor, Dark, or Topographic.
- **Camera:** Smooth Follow, Cinematic Chase, or Route Overview.
- **Terrain:** MapLibre terrain exaggeration.
- **Format:** landscape `16:9`, vertical `9:16`, or square `1:1`.
- **Duration:** 20–60 seconds.

The offline MapLibre scene combines the Protomaps vector basemap, Mapterhorn Terrarium elevation mesh, hillshade, route progress, position/end markers, and building extrusion where height data exists. Route movement uses cumulative physical distance.

## Export an MP4

1. Import a GPX and confirm export readiness is complete.
2. Choose format, duration, style, and camera.
3. Select **Export video** and choose a new `.mp4` filename.
4. Leave Waypoint open until completion.

Export renders the same MapLibre scene used by preview at `frameIndex / fps`, waits for local tiles, sends raw RGBA frames to FFmpeg, and verifies the temporary output before publishing it. Existing files are never overwritten.

## Privacy

- Waypoint requires no account and has no analytics or cloud sync.
- Source GPX files are read-only inputs.
- Downloaded map data and exported videos stay in locations on this computer.
- The main renderer never contacts map providers; only the explicit Map Downloader performs network downloads.

## Current limitations

This is a pre-alpha. Project save/open, media matching, music, privacy transformations, resumable map downloads, export cancellation, installers, FIT, and TCX are not complete.
