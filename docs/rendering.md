# Deterministic rendering

Every frame is evaluated from explicit animation time:

```text
time = frame_number / fps
progress = timeline_progress(time, duration)
scene = evaluate(activity, progress)
```

`render-core` writes RGB frames directly to FFmpeg stdin, avoiding a permanent PNG sequence. FFmpeg writes a temporary MP4. After a successful exit, ffprobe verifies that a video stream exists at the requested dimensions. Only then is the temporary file renamed to the user-selected final path.

The same project snapshot should always produce the same frame sequence. Wall-clock time is never read during export.

