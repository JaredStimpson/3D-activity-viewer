# Deterministic rendering

Preview and production export use the same TypeScript scene evaluator and the same MapLibre style. Every exported frame is evaluated from its integer frame number:

```text
time = frame_number / fps
progress = timeline_progress(time, duration)
scene = evaluate(activity, progress)
```

The scene contains the camera, full and progressive route lines, current marker, endpoints, basemap, hillshade, Terrarium terrain, and building extrusion. Map assets are read from validated region IDs through an optimized Rust byte-range command; the frontend cannot request arbitrary filesystem paths.

For export, Waypoint creates an attached offscreen MapLibre canvas at the requested dimensions, applies the deterministic scene, waits for local tiles, and composites titles/statistics into a capture canvas. Raw RGBA bytes are sent through Tauri binary IPC into an FFmpeg stdin session—there is no base64 conversion or permanent PNG sequence.

FFmpeg writes a temporary H.264 MP4. After a successful exit, ffprobe verifies the video stream and requested dimensions. Only then is the temporary file renamed to the selected final path. Existing files are never overwritten silently.

The legacy procedural Rust renderer remains available only to the `render-activity` technical-proof CLI; it is not used by the main app's supported export path.
