# Troubleshooting

## Setup window closes or reports an error

Open PowerShell in the repository and run setup directly so the full message remains visible:

```powershell
.\scripts\setup.ps1 -InstallMissing
```

If Windows blocks the script, use the provided **Setup Waypoint.cmd** entry point or run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup.ps1 -InstallMissing
```

## WinGet is unavailable

Install or update **App Installer** from Microsoft Store, restart PowerShell, and run setup again. You can also install Node.js, Rust, FFmpeg, and Visual Studio C++ Build Tools manually, then run:

```powershell
.\scripts\setup.ps1
```

## A prerequisite was installed but is still not found

Restart Windows so new PATH entries are available, then run **Setup Waypoint.cmd** again.

You can inspect the prerequisite check without changing anything:

```powershell
.\scripts\setup.ps1 -CheckOnly
```

## Corepack reports EPERM for `C:\Program Files\nodejs`

Older versions of Waypoint setup used `corepack enable`, which could try to write package-manager shims into the protected Node.js installation folder. Pull the latest repository changes and run **Setup Waypoint.cmd** again.

Current setup installs pnpm into the current user's local Waypoint tools folder and does not need permission to modify `C:\Program Files\nodejs`.

## Waypoint has not been built

Run **Setup Waypoint.cmd**. A successful build produces:

```text
target\release\waypoint-desktop.exe
```

## The application opens with a blank window

Install current Windows updates and repair or install the Microsoft Edge WebView2 Runtime. Waypoint uses WebView2 for its embedded interface.

## A GPX file will not import

Confirm that:

- the file extension is `.gpx`;
- the file is valid XML;
- it contains a track with at least two `<trkpt>` elements;
- each track point has numeric `lat` and `lon` attributes.

The status bar displays the parser's error when import fails.

## Export says FFmpeg is unavailable

Run:

```powershell
ffmpeg -version
ffprobe -version
```

If either command is missing, rerun **Setup Waypoint.cmd** or install FFmpeg with:

```powershell
winget install --id Gyan.FFmpeg --exact
```

Restart Windows afterward if the commands are still unavailable.

## Export destination already exists

Waypoint intentionally does not overwrite videos. Select **Export video** again and choose a new filename.

## Export is slow

The current proof renders deterministic software-generated frames before H.264 encoding. Vertical and longer videos contain more pixel work. Keep the application open and avoid sleep mode until the export finishes.

## Verify the repository

Developers can run the complete available check suite with:

```powershell
.\scripts\verify.ps1
```
