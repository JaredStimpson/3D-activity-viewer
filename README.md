# Waypoint

Waypoint is a local-first Windows desktop application that turns GPX activities into animated route videos. It uses a Tauri 2 desktop shell, a React/TypeScript editor, Rust activity processing, and FFmpeg video encoding.

The repository now includes the technical proof plus a focused offline map downloader and repo-local MapLibre terrain renderer.

## What works now

- Import a GPX file without modifying the source.
- Parse and normalize track points in Rust.
- Calculate distance, elevation gain/loss, bounds, and distance-based route progress.
- Download a bounded Protomaps basemap and Mapterhorn terrain region with a companion Tauri GUI.
- Preview the verified local PMTiles region with MapLibre terrain and progressive route animation.
- Change map style, camera preset, terrain exaggeration, duration, and aspect ratio.
- Render deterministic MapLibre RGBA frames to H.264 through FFmpeg binary IPC.
- Verify rendered dimensions with ffprobe before publishing the final file.
- Keep downloaded map archives repo-local and Git-ignored.

The `render-activity` CLI remains a procedural technical proof. The supported desktop path uses the offline MapLibre scene for both preview and export.

## Quick start on Windows

1. Double-click **Setup Waypoint.cmd** once.
2. Double-click **Run Map Downloader from Source.cmd** and download a region.
3. Double-click **Run Waypoint from Source.cmd**.

Setup checks the machine, offers to install missing prerequisites through WinGet, and installs locked dependencies without compiling a release. Use **Build Distribution.cmd** only when you want both release executables. See the [getting started guide](docs/getting-started.md).

## Requirements

- Windows 11 x86-64
- Node.js 20 or newer and pnpm 11
- Rust stable
- FFmpeg and ffprobe available on `PATH`
- Microsoft Edge WebView2 (included with current Windows 11 installations)

## Use the app

See the [user guide](docs/user-guide.md) for GPX import, preview controls, MP4 export, privacy behavior, and current limitations. The [documentation index](docs/README.md) links all user and developer references.

## Development

```powershell
.\scripts\setup.ps1 -InstallMissing
.\scripts\dev.ps1
.\scripts\dev-map-downloader.ps1
```

Run all checks:

```powershell
.\scripts\verify.ps1
```

Verify and build both release executables:

```powershell
.\scripts\build-dist.ps1
```

## Command-line proof

```powershell
cargo run -p render-activity -- sample-data/sample.gpx output.mp4
```

The CLI uses the same `activity-core` and `render-core` crates as the desktop application.

## Repository layout

```text
apps/desktop/          Tauri 2 + React desktop application
apps/map-downloader/   Focused Tauri map downloader
crates/activity-core/  GPX parsing, normalized activity model, statistics
crates/map-assets/     Bounds, downloads, manifests, PMTiles, coverage selection
crates/project-core/   Versioned project model and atomic saves
crates/render-core/    Deterministic frames, FFmpeg pipe, ffprobe validation
tools/render-activity/ Minimal GPX-to-MP4 proof command
sample-data/           Synthetic sample GPX
maps/                  Map schema/docs and ignored downloaded regions
docs/                  Architecture and implementation notes
scripts/               Windows setup, launch, development, and verification
```

## Privacy

Waypoint has no account system, analytics, cloud sync, or proprietary backend. Imported GPS data and rendered video remain on the user's computer. Source activity files are read-only inputs.

## Status

Pre-alpha (`0.1.0`). See [docs/roadmap.md](docs/roadmap.md) for the sequence from this technical proof to the full local-first editor.
