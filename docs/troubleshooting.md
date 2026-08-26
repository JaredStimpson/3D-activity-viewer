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

For immediate development use, run **Run Waypoint from Source.cmd**. To create both release programs, run **Build Distribution.cmd**. A successful distribution build produces:

```text
target\release\waypoint-desktop.exe
target\release\waypoint-map-downloader.exe
```

The two **Launch** scripts only start existing release executables; they never compile.

## Waypoint says map data is required

Copy the exact `west,south,east,north` bounds shown by Waypoint into Map Downloader. Choose a rectangle with a little margin around the route, complete the download, then select **Refresh map data** in Waypoint.

If a region is listed but not selected, its manifest, byte size, PMTiles header, or SHA-256 hash may not verify. Run the downloader again for that area under a new name after moving the damaged region out of `maps\regions`.

## A map download is rejected before starting

Check coordinate order: west longitude, south latitude, east longitude, north latitude. Antimeridian-crossing, zero-area, reversed, or overly large rectangles are not supported. Latitudes beyond Web Mercator limits are clamped to approximately ±85.05113°.

## A map download fails midway

Check network access and free disk space, then retry. Transient tile requests are attempted three times. Cancelling or failing removes the incomplete staging directory; completed regions are not overwritten silently.

## A map download appears stuck

Select **Live diagnostics** in Map Downloader. The browser readout updates every second and shows the most recent provider probe, archive open, tile retry/progress, finalization, hash, or verification operation. The preferred address is `http://127.0.0.1:4765/`, but use the exact address displayed in the app because it chooses another local port when 4765 is occupied.

Copy the last several lines when reporting a problem. Long pauses after an HTTP failure may be the configured provider timeout and retry cycle. The diagnostics server is local-only, retains at most 4,000 in-memory lines, and stops with the downloader.

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

Waypoint renders a full local MapLibre frame and terrain tiles before each H.264 frame. Large output formats and longer videos require more GPU, CPU, and disk throughput. Keep the application open and avoid sleep mode until export finishes.

## Verify the repository

Developers can run the complete available check suite with:

```powershell
.\scripts\verify.ps1
```
