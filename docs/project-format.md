# Project format

Project files are versioned JSON instructions, not containers for source media, maps, render frames, or final videos.

```json
{
  "formatVersion": 1,
  "projectId": "9f8b02ae",
  "revision": 1,
  "title": "Big Sur Ridge Ride",
  "activitySource": "activity/activity.gpx",
  "video": {
    "aspectRatio": "16:9",
    "width": 1920,
    "height": 1080,
    "fps": 30,
    "durationSeconds": 32
  }
}
```

`project-core::save_atomic` writes a temporary file, flushes it, validates it, moves the previous project to a backup, and only then installs the new file.

When map selection is added to the durable project model, it will store only a verified region ID. Projects and manifests must never contain a machine-specific absolute path to `maps`, `basemap.pmtiles`, or `terrain.pmtiles`.
