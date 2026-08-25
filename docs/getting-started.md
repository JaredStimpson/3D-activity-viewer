# Getting started on Windows

These instructions build and launch Waypoint from the repository. Setup is designed for Windows 11 x86-64.

## Fastest setup

1. Download or clone the repository to a normal folder such as `Documents\Waypoint`.
2. Double-click **Setup Waypoint.cmd**.
3. Approve any Windows prompts for missing prerequisites.
4. Leave the setup window open while it installs dependencies and compiles the application. The first build can take several minutes.
5. When setup reports success, double-click **Launch Waypoint.cmd**.

The setup script checks for and can install:

- Node.js LTS
- Rust
- FFmpeg and ffprobe
- Visual Studio C++ Build Tools
- pnpm installed under the current user's local application-data folder

It then installs the exact locked project dependencies and creates:

```text
target\release\waypoint-desktop.exe
```

Later launches use that executable directly and do not repeat setup.

Setup does not use `corepack enable` or write package-manager shims into `C:\Program Files\nodejs`. pnpm is installed in a user-writable Waypoint tools folder, so this step does not require an additional administrator prompt.

## Setup from PowerShell

Open PowerShell in the repository and run:

```powershell
.\scripts\setup.ps1 -InstallMissing
.\scripts\launch.ps1
```

To check prerequisites without installing or building anything:

```powershell
.\scripts\setup.ps1 -CheckOnly
```

To prepare dependencies but skip the release build:

```powershell
.\scripts\setup.ps1 -InstallMissing -SkipBuild
```

## Update an existing checkout

After pulling new code, run **Setup Waypoint.cmd** again. Locked dependencies will be refreshed and the release executable will be rebuilt.

## Uninstall

Waypoint does not currently install a Windows service or background process. Delete the repository folder to remove the source build. Any GPX files or exported videos you selected remain in their original locations.

WinGet-installed development tools are shared system tools and are not removed automatically.
