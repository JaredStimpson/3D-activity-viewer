# Waypoint project vault

Last updated: 2026-08-25

This file is the durable working memory for the repository. Read it completely before planning or implementing a change. Keep it useful as a current-state guide, not just an append-only diary.

## Vault workflow

For every material task:

1. Read `vault.md` before scanning the wider repository.
2. Check the Git working tree and recent history so uncommitted user work is preserved.
3. Use the current architecture, invariants, and commands below as the starting point.
4. Update the relevant current-state sections when behavior or structure changes.
5. Add one concise entry to the change ledger.
6. Keep secrets, access tokens, personal GPS data, generated videos, and machine-specific absolute paths out of this file.

If the vault conflicts with the code, verify the behavior, fix the stale vault entry in the same change, and treat tested code as the source of truth.

## Product intent

Waypoint is a Windows-first, local-first PC application that turns recorded outdoor activities into animated 3D route videos. It is a Tauri desktop application, not a hosted web app.

Core product rules:

- GPS tracks, media, projects, logs, and exports stay local unless the user explicitly shares them.
- Source GPX files and source media are never modified.
- Preview and export should evaluate the same deterministic scene timeline.
- Export time is `frame_number / fps`; wall-clock time must not affect rendered frames.
- Maps, thumbnails, temporary frames, and render caches must be regeneratable.
- Project state, geographic assets, temporary render data, and final exports remain separate.
- The renderer never downloads geographic data directly; the future Asset Manager supplies local asset handles.
- PMTiles is a tile archive, not the source of 3D. Real terrain requires a compatible DEM; use separate basemap and terrain sources behind one installed-region handle.
- The application should automate creative decisions and hide GPX, map-tile, camera-curve, and FFmpeg complexity from users.

## Current baseline

Version: pre-alpha `0.1.0`, technical proof milestone.

Implemented:

- Tauri 2 Windows desktop shell.
- React and TypeScript editor interface.
- Rust GPX parsing with duplicate removal, normalized points, bounds, distance, duration, and elevation statistics.
- Distance-based route progress shared conceptually by preview and export.
- Procedural local terrain preview with route animation and camera/style controls.
- Deterministic RGB frame generation piped to FFmpeg.
- Temporary MP4 output, H.264 encoding, ffprobe dimension verification, and no silent final-file overwrite.
- Minimal `render-activity` GPX-to-MP4 CLI using the same Rust core as the desktop app.
- Versioned project model and atomic-save foundation; project save/open is not wired into the UI yet.
- Windows double-click setup and launch entry points.
- User, troubleshooting, development, architecture, project-format, rendering, and roadmap documentation.

Not implemented yet:

- MapLibre scene rendering or real offline map/terrain archives.
- Asset Manager, PMTiles/MBTiles storage, coverage planning, downloads, or SQLite index.
- Track jump detection, smoothing, resampling, and render simplification.
- Durable project creation/open/autosave/recovery in the desktop UI.
- Photo/video import, metadata extraction, matching, or media events.
- Music, labels, privacy zones/radius, export progress, cancellation, resume, or history.
- FIT and TCX input.
- Installer packaging or automated release publishing.

## Repository map

| Surface | Responsibility |
| --- | --- |
| `apps/desktop/src/` | React editor, local preview, import interaction, and export controls |
| `apps/desktop/src-tauri/` | Native window, Tauri commands, permissions, and Windows app configuration |
| `crates/activity-core/` | GPX parsing, normalized activity model, distance progression, and statistics |
| `crates/project-core/` | Versioned project model and atomic JSON saving |
| `crates/render-core/` | Deterministic frame generation, FFmpeg pipe, temporary output, and ffprobe verification |
| `tools/render-activity/` | Minimal command-line technical proof |
| `scripts/` | Windows setup, launch, development, and verification flows |
| `sample-data/` | Synthetic non-personal GPX fixture |
| `docs/` | User and engineering documentation, including the canonical activity/geographic data requirements report |

Architectural dependency direction:

```text
desktop UI -> Tauri commands -> activity-core / project-core / render-core
render-activity CLI ---------> activity-core / render-core
future renderer -------------> Asset Manager handles, never direct downloads
```

Keep core crates independent of Tauri and React so they remain reusable by the GUI and CLI.

## How to build on each area

### Activity processing

- Make canonical parsing and statistics changes in `crates/activity-core`.
- Keep the browser-only fallback in `apps/desktop/src/lib/activity.ts` behaviorally aligned until it can be removed.
- Add Rust tests for malformed input, normalization, geometry, and statistics.
- Route animation must use cumulative physical distance, never raw point index.

### Preview and camera

- Interactive preview code currently lives in `apps/desktop/src/components/RoutePreview.tsx`.
- The procedural surface is a proof, not the final offline map implementation.
- Camera work should be smooth, deterministic from evaluated time, and independent of noisy instantaneous GPS heading.
- When MapLibre arrives, preserve a strict boundary between scene decisions and rendering mechanics.
- Keep activity elevation distinct from the surrounding DEM: track elevation supports route altitude and statistics, while DEM tiles create the geographic terrain mesh.

### Export

- Change deterministic rendering in `crates/render-core` and document pipeline changes in `docs/rendering.md`.
- Always render to a temporary file and verify before moving to the final destination.
- Never silently overwrite an existing export.
- Run an actual sample GPX-to-MP4 proof when changing frame generation, FFmpeg arguments, naming, or verification.

### Project durability

- `project-core::save_atomic` is the starting point for project saving.
- Project JSON describes how to rebuild the video; it must not contain maps, full media bytes, render frames, or final video bytes.
- Meaningful edits should increment a revision. Export must eventually use a frozen project snapshot.

### Windows setup and launch

- User entry points are `Setup Waypoint.cmd` and `Launch Waypoint.cmd`.
- `scripts/setup.ps1` may install missing prerequisites with WinGet, restores locked dependencies, and builds the release executable.
- pnpm is installed with npm into the current user's local Waypoint tools folder; setup must not use `corepack enable` or write shims into `C:\Program Files\nodejs`.
- `scripts/launch.ps1` starts `target/release/waypoint-desktop.exe` and can trigger first-time setup.
- Keep `docs/getting-started.md` and `docs/troubleshooting.md` synchronized with script behavior.

### Dependencies

- Package manager: pnpm `11.19.0`.
- Commit both `pnpm-lock.yaml` and `Cargo.lock`.
- Setup uses locked dependency resolution.
- pnpm permits the required `esbuild` setup script; do not broadly enable dependency build scripts.
- Required Windows runtime/build tools are Node.js 20+, Rust stable, Visual Studio C++ Build Tools, FFmpeg/ffprobe, and WebView2.

## Standard commands

User setup and launch:

```powershell
.\scripts\setup.ps1 -InstallMissing
.\scripts\launch.ps1
```

Development:

```powershell
.\scripts\setup.ps1 -InstallMissing -SkipBuild
.\scripts\dev.ps1
```

Repository verification:

```powershell
.\scripts\verify.ps1
```

Manual technical proof:

```powershell
cargo run -p render-activity -- sample-data\sample.gpx output.mp4
```

Release executable:

```text
target\release\waypoint-desktop.exe
```

## Verification baseline

Last fully verified: 2026-08-25 after the user-local pnpm setup fix.

- Interface: 1 Vitest test passed.
- TypeScript and Vite production build passed.
- Rust: 6 workspace tests passed.
- Rust formatting check passed.
- Clippy was skipped because the local Rust toolchain did not include that optional component.
- Tauri release build passed and produced the Windows executable.
- End-to-end sample render produced a verified 24-second, 1280×720, 30 FPS H.264 MP4.
- Setup prerequisite check and launch dry run passed.
- The first-run pnpm recovery path passed with Node under Program Files and pnpm isolated in a user-writable tools folder; locked JavaScript and Windows-target Rust dependency restoration completed successfully.

Update this section whenever validation coverage or results materially change.

## Next recommended product slice

Follow the plan's quality priority instead of expanding the editor shell first:

1. Add track validation, impossible-jump removal, smoothing, resampling, simplification, and tests in `activity-core`.
2. Define a renderer-neutral scene/timeline model shared by preview and export.
3. Integrate MapLibre against a deliberately local asset interface.
4. Build initial smooth camera planning with look-ahead and finish overview.
5. Prove preview/export parity before starting the full Asset Manager or media matching.

Re-evaluate this order if the user explicitly chooses a different milestone.

## Change ledger

### 2026-08-25 — Activity and geographic data requirements report

- Added the canonical report covering required activity fields, derived data, offline basemap, DEM, style, media, and runtime dependencies.
- Established that real 3D terrain comes from a MapLibre `raster-dem` mesh, not from PMTiles itself or the route's recorded elevation.
- Recommended separate basemap and terrain archives grouped behind one Asset Manager region handle and documented the current procedural-terrain limitation.
- Verified all local documentation links and a whitespace-clean Git diff.

### 2026-08-25 — User-local pnpm setup fix

- Replaced the elevated `corepack enable` path with a user-writable npm global prefix for pnpm.
- Setup now persists only the Waypoint tools folder in the user's PATH and supports an isolated override for testing.
- Added recovery instructions for the prior `EPERM` failure in the protected Node.js installation folder.
- Verified the isolated first-run dependency flow and the complete interface/Rust check suite.

### 2026-08-25 — Vault method adopted

- Added this root project memory and required future tasks to read and maintain it.
- Captured current architecture, workflows, verification baseline, limitations, and recommended next slice.

### 2026-08-25 — Streamlined setup and documentation (`ab7f9ac`)

- Added double-click Windows setup and launch entry points.
- Added prerequisite checks/install support, locked dependency restore, development and verification scripts.
- Added the user guide, getting-started guide, troubleshooting guide, and development guide.

### 2026-08-25 — Initial technical proof (`f9cf239`)

- Created the Tauri/React/Rust workspace.
- Implemented GPX parsing/statistics, procedural preview, deterministic FFmpeg export, CLI proof, project model foundation, tests, and architecture documentation.

## Ledger entry template

```markdown
### YYYY-MM-DD — Short change title

- What changed for users or developers.
- Important architecture or behavior decisions.
- Verification performed and any known gap introduced or resolved.
```
