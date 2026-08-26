[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location -LiteralPath $RepositoryRoot

& (Join-Path $PSScriptRoot "setup.ps1") -InstallMissing
if ($LASTEXITCODE -ne 0) { throw "Dependency setup failed." }

& (Join-Path $PSScriptRoot "verify.ps1")
if ($LASTEXITCODE -ne 0) { throw "Verification failed; no distribution was built." }

Write-Host "Building Waypoint release executable..." -ForegroundColor Cyan
pnpm.cmd tauri:build
if ($LASTEXITCODE -ne 0) { throw "Waypoint release build failed." }

Write-Host "Building Map Downloader release executable..." -ForegroundColor Cyan
pnpm.cmd tauri:build:maps
if ($LASTEXITCODE -ne 0) { throw "Map Downloader release build failed." }

$WaypointExecutable = Join-Path $RepositoryRoot "target\release\waypoint-desktop.exe"
$DownloaderExecutable = Join-Path $RepositoryRoot "target\release\waypoint-map-downloader.exe"
if (-not (Test-Path -LiteralPath $WaypointExecutable)) { throw "Waypoint executable was not produced." }
if (-not (Test-Path -LiteralPath $DownloaderExecutable)) { throw "Map Downloader executable was not produced." }

Write-Host "`nDistribution executables are ready:" -ForegroundColor Green
Write-Host $WaypointExecutable
Write-Host $DownloaderExecutable
