# User guide

## Launch the application

Double-click **Launch Waypoint.cmd** in the repository folder. If the release executable does not exist yet, the launcher starts first-time setup automatically.

The editor opens with a sample Big Sur route so you can explore the interface immediately. The sample is a visual demonstration; import your own GPX before exporting.

## Import a GPX activity

1. Select **Import GPX** in the upper-right corner.
2. Choose a `.gpx` file containing at least two track points.
3. Wait for the status bar to confirm that the file was imported.

Waypoint reads the GPX without changing it. It calculates route distance, elevation gain and loss, duration when timestamps are available, and geographic bounds locally.

## Preview the activity

Use the round play/pause button below the preview to control playback. Drag the timeline scrubber to inspect a specific point.

The Inspector provides the controls currently available:

- **Map style:** Outdoor, Dark, or Topographic
- **Camera:** Smooth Follow, Cinematic Chase, or Route Overview
- **Terrain:** adjust the procedural terrain exaggeration
- **Format:** landscape `16:9`, vertical `9:16`, or square `1:1`
- **Duration:** choose a video length from 20 to 60 seconds

The route animation advances by cumulative physical distance rather than raw point count, which keeps movement stable across uneven GPS sampling.

## Export an MP4

1. Import a GPX file. The built-in sample cannot be exported by itself.
2. Choose the format and duration.
3. Select **Export video**.
4. Choose a new `.mp4` filename and destination.
5. Leave Waypoint open until the status bar reports completion.

Waypoint creates deterministic RGB frames, streams them to FFmpeg, and writes a temporary video. It verifies the video stream and requested dimensions with ffprobe before moving the result to the final filename.

Existing files are never overwritten. Choose a different filename if the destination already exists.

## Privacy and local files

- GPX parsing and video rendering happen on this computer.
- Waypoint does not require an account.
- The application has no analytics, cloud sync, or proprietary backend.
- Imported GPX files are read-only inputs.
- Your selected export stays in the destination you choose.

## Current limitations

This pre-alpha build uses deterministic procedural terrain, not real map tiles. Saved projects, offline PMTiles regions, photo/video matching, music, privacy-radius transformations, progress cancellation, and export history are planned but not implemented yet.
