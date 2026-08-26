[CmdletBinding()]
param([switch]$DryRun)

$ErrorActionPreference = "Stop"
$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Executable = Join-Path $RepositoryRoot "target\release\waypoint-desktop.exe"

if (-not (Test-Path -LiteralPath $Executable)) {
    throw "Waypoint has not been built. Run 'Build Distribution.cmd', or use 'Run Waypoint from Source.cmd'."
}

$env:WAYPOINT_MAPS_DIR = Join-Path $RepositoryRoot "maps"

if ($DryRun) {
    Write-Host "Waypoint is ready to launch: $Executable" -ForegroundColor Green
    exit 0
}

Write-Host "Launching Waypoint..." -ForegroundColor Cyan
Start-Process -FilePath $Executable -WorkingDirectory $RepositoryRoot
