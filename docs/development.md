# Development guide

## Start development mode

From PowerShell:

```powershell
.\scripts\setup.ps1 -InstallMissing
.\scripts\dev.ps1                  # main app, Vite port 1420
.\scripts\dev-map-downloader.ps1   # downloader, Vite port 1430
```

`dev.ps1` verifies the environment, installs locked dependencies when needed, starts Vite, compiles the Tauri host, and opens the desktop window with hot reload.

## Useful commands

```powershell
# Interface only
pnpm dev

# Native desktop development
pnpm tauri:dev
pnpm tauri:dev:maps

# Verify and build both release executables
.\scripts\build-dist.ps1

# All available repository checks
.\scripts\verify.ps1

# GPX-to-MP4 CLI proof
cargo run -p render-activity -- sample-data\sample.gpx output.mp4
```

## Verification

The verification script runs:

1. Vitest interface tests for both apps.
2. TypeScript and Vite production builds for both apps.
3. Rust workspace tests with the committed lockfile.
4. Rust formatting checks.
5. Clippy with warnings denied when the Clippy component is installed.

Install the optional Clippy component with:

```powershell
rustup component add clippy
```

## Repository structure

```text
apps/desktop/          Tauri 2 + React desktop application
apps/map-downloader/   Focused Tauri 2 + React map downloader
crates/activity-core/  GPX parsing, activity model, statistics
crates/map-assets/     Bounds, manifests, PMTiles reads/downloads, region discovery
crates/project-core/   Project model and atomic saves
crates/render-core/    Deterministic frames and FFmpeg pipeline
tools/render-activity/ Minimal GPX-to-MP4 command
scripts/               Windows setup, launch, development, and verification
sample-data/           Synthetic GPX fixture
maps/                  Tracked schema/docs plus ignored downloaded regions
docs/                  User and engineering documentation
```

## Dependency policy

Both `pnpm-lock.yaml` and `Cargo.lock` are committed. Setup and verification use locked dependency resolution. pnpm permits the required `esbuild` native setup script and keeps other dependency build scripts blocked by default.
