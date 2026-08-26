# Getting started on Windows

Waypoint is a Windows PC application. The main app and its focused Map Downloader companion can run directly from source during development; release executables are only created when you explicitly build a distribution.

## First-time setup

1. Clone or download the repository to a normal user-writable folder.
2. Double-click **Setup Waypoint.cmd**.
3. Approve Windows prompts for any missing prerequisites.

Setup checks or installs Node.js LTS, pnpm 11.19.0, Rust, Visual Studio C++ Build Tools, FFmpeg/ffprobe, and locked project dependencies. It does **not** compile release executables.

pnpm is installed in a user-writable Waypoint tools folder. Setup does not use `corepack enable` or attempt to create shims in `C:\Program Files\nodejs`.

## Run from source

Double-click either launcher:

- **Run Map Downloader from Source.cmd** — downloads an offline region into `maps\regions`.
- **Run Waypoint from Source.cmd** — opens the main editor.

Both launchers set `WAYPOINT_MAPS_DIR` to the repository's `maps` folder. Vite live-reloads React changes. Tauri performs an initial debug Rust compile, then only recompiles Rust when relevant source changes. No release build occurs.

For the first activity, run Map Downloader first and enter a bounding box covering the route. See [Map Downloader](map-downloader.md).

## Build and launch release executables

Double-click **Build Distribution.cmd**. It runs verification and creates:

```text
target\release\waypoint-desktop.exe
target\release\waypoint-map-downloader.exe
```

After that, **Launch Waypoint.cmd** and **Launch Map Downloader.cmd** start the existing executables immediately. They never compile automatically and tell you to build when an executable is missing.

The raw executables require WebView2 and FFmpeg on the target machine.

## PowerShell equivalents

```powershell
.\scripts\setup.ps1 -InstallMissing
.\scripts\dev-map-downloader.ps1
.\scripts\dev.ps1
.\scripts\build-dist.ps1
```

Check prerequisites without installing or building:

```powershell
.\scripts\setup.ps1 -CheckOnly
```

After pulling changes, rerun setup to restore locked dependencies. Rebuild only when you want updated release executables.
