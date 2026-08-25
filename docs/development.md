# Development guide

## Start development mode

From PowerShell:

```powershell
.\scripts\setup.ps1 -InstallMissing -SkipBuild
.\scripts\dev.ps1
```

`dev.ps1` verifies the environment, installs locked dependencies when needed, starts Vite, compiles the Tauri host, and opens the desktop window with hot reload.

## Useful commands

```powershell
# Interface only
pnpm dev

# Native desktop development
pnpm tauri:dev

# Release executable
pnpm tauri:build

# All available repository checks
.\scripts\verify.ps1

# GPX-to-MP4 CLI proof
cargo run -p render-activity -- sample-data\sample.gpx output.mp4
```

## Verification

The verification script runs:

1. Vitest interface tests.
2. TypeScript and Vite production build.
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
crates/activity-core/  GPX parsing, activity model, statistics
crates/project-core/   Project model and atomic saves
crates/render-core/    Deterministic frames and FFmpeg pipeline
tools/render-activity/ Minimal GPX-to-MP4 command
scripts/               Windows setup, launch, development, and verification
sample-data/           Synthetic GPX fixture
docs/                  User and engineering documentation
```

## Dependency policy

Both `pnpm-lock.yaml` and `Cargo.lock` are committed. Setup and verification use locked dependency resolution. pnpm permits the required `esbuild` native setup script and keeps other dependency build scripts blocked by default.
