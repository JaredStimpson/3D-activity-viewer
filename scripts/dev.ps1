[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

& (Join-Path $PSScriptRoot "setup.ps1") -CheckOnly
if ($LASTEXITCODE -ne 0) {
    throw "Development prerequisites are missing. Run .\scripts\setup.ps1 -InstallMissing."
}

Set-Location -LiteralPath $RepositoryRoot
pnpm install --frozen-lockfile
if ($LASTEXITCODE -ne 0) { throw "Dependency installation failed." }
pnpm tauri:dev
