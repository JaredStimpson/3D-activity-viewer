# Waypoint

Waypoint is a local-first Windows desktop application that turns GPX activities into animated route videos. It uses a Tauri 2 desktop shell, a React/TypeScript editor, Rust activity processing, and FFmpeg video encoding.

This initial repository implements the plan's Phase 0 technical proof and lays the foundation for the later editor and offline-map phases.

## What works now

- Import a GPX file without modifying the source.
- Parse and normalize track points in Rust.
- Calculate distance, elevation gain/loss, bounds, and distance-based route progress.
- Preview a stylized local terrain scene with progressive route animation.
- Change map style, camera preset, terrain exaggeration, duration, and aspect ratio.
- Render deterministic H.264 MP4 video frames through FFmpeg.
- Verify rendered dimensions with ffprobe before publishing the final file.
- Run the same render engine from the desktop app or the `render-activity` CLI.

The terrain renderer is intentionally procedural in this proof. Real regional PMTiles/terrain archives and MapLibre integration belong to the next map-renderer and asset-manager milestones.

## Quick start on Windows

1. Double-click **Setup Waypoint.cmd** once.
2. Double-click **Launch Waypoint.cmd** whenever you want to use the app.

Setup checks the machine, offers to install missing prerequisites through WinGet, installs locked dependencies, and builds the native Windows executable. See the [getting started guide](docs/getting-started.md) for PowerShell and troubleshooting options.

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
.\scripts\setup.ps1 -InstallMissing -SkipBuild
.\scripts\dev.ps1
```

Run all checks:

```powershell
.\scripts\verify.ps1
```

Build the desktop executable:

```powershell
.\scripts\setup.ps1
```

## Command-line proof

```powershell
cargo run -p render-activity -- sample-data/sample.gpx output.mp4
```

The CLI uses the same `activity-core` and `render-core` crates as the desktop application.

## Repository layout

```text
apps/desktop/          Tauri 2 + React desktop application
crates/activity-core/  GPX parsing, normalized activity model, statistics
crates/project-core/   Versioned project model and atomic saves
crates/render-core/    Deterministic frames, FFmpeg pipe, ffprobe validation
tools/render-activity/ Minimal GPX-to-MP4 proof command
sample-data/           Synthetic sample GPX
docs/                  Architecture and implementation notes
scripts/               Windows setup, launch, development, and verification
```

## Privacy

Waypoint has no account system, analytics, cloud sync, or proprietary backend. Imported GPS data and rendered video remain on the user's computer. Source activity files are read-only inputs.

## Status

Pre-alpha (`0.1.0`). See [docs/roadmap.md](docs/roadmap.md) for the sequence from this technical proof to the full local-first editor.
