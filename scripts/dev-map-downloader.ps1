[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$env:WAYPOINT_MAPS_DIR = Join-Path $RepositoryRoot "maps"

& (Join-Path $PSScriptRoot "setup.ps1") -CheckOnly
if ($LASTEXITCODE -ne 0) {
    throw "Development prerequisites are missing. Run 'Setup Waypoint.cmd'."
}

Set-Location -LiteralPath $RepositoryRoot
pnpm.cmd install --frozen-lockfile
if ($LASTEXITCODE -ne 0) { throw "Dependency installation failed." }
pnpm.cmd tauri:dev:maps
if ($LASTEXITCODE -ne 0) { throw "Map Downloader source launch failed." }
