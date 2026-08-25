[CmdletBinding()]
param(
    [switch]$SetupIfMissing,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Executable = Join-Path $RepositoryRoot "target\release\waypoint-desktop.exe"

if (-not (Test-Path -LiteralPath $Executable)) {
    if (-not $SetupIfMissing) {
        throw "Waypoint has not been built. Run 'Setup Waypoint.cmd' first."
    }
    Write-Host "Waypoint is not built yet. Starting first-time setup..." -ForegroundColor Yellow
    & (Join-Path $PSScriptRoot "setup.ps1") -InstallMissing
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $Executable)) {
        throw "Waypoint setup did not produce an executable."
    }
}

if ($DryRun) {
    Write-Host "Waypoint is ready to launch: $Executable" -ForegroundColor Green
    exit 0
}

Write-Host "Launching Waypoint..." -ForegroundColor Cyan
Start-Process -FilePath $Executable -WorkingDirectory $RepositoryRoot
