[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$env:WAYPOINT_MAPS_DIR = Join-Path $RepositoryRoot "maps"

& (Join-Path $PSScriptRoot "setup.ps1") -CheckOnly
if ($LASTEXITCODE -ne 0) {
    throw "Development prerequisites are missing. Run .\scripts\setup.ps1 -InstallMissing."
}

Set-Location -LiteralPath $RepositoryRoot
pnpm.cmd install --frozen-lockfile
if ($LASTEXITCODE -ne 0) { throw "Dependency installation failed." }
pnpm.cmd tauri:dev
if ($LASTEXITCODE -ne 0) { throw "Waypoint source launch failed." }
