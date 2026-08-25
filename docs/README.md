# Waypoint documentation

Waypoint is a Windows desktop application for turning GPX activities into local, animated route videos. Start with the guide that matches what you want to do.

## Use Waypoint

- [Getting started](getting-started.md) — first-time setup and launch
- [User guide](user-guide.md) — import an activity, adjust the preview, and export an MP4
- [Troubleshooting](troubleshooting.md) — common setup, launch, GPX, and export problems

## Work on Waypoint

- [Project vault](../vault.md) — current working memory, invariants, workflows, and change ledger; read this first
- [Development guide](development.md) — development environment, commands, checks, and repository layout
- [Architecture](architecture.md) — boundaries between activity parsing, project state, preview, and rendering
- [Activity and geographic data requirements](data-requirements.md) — activity fields, offline map assets, and the source of real 3D terrain
- [Project format](project-format.md) — durable JSON project model
- [Deterministic rendering](rendering.md) — frame evaluation and FFmpeg pipeline
- [Roadmap](roadmap.md) — progression from the current proof to the complete application

## Current release status

Waypoint is pre-alpha software. The current build proves the local GPX-to-video pipeline. It does not yet include real offline map packages, photo matching, saved project reopening, or resumable exports.
