[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location -LiteralPath $RepositoryRoot

Write-Host "Running interface tests..." -ForegroundColor Cyan
pnpm.cmd test
if ($LASTEXITCODE -ne 0) { throw "Interface tests failed." }
pnpm.cmd --filter @activity-video/map-downloader test
if ($LASTEXITCODE -ne 0) { throw "Map downloader interface tests failed." }

Write-Host "Building the interface..." -ForegroundColor Cyan
pnpm.cmd build
if ($LASTEXITCODE -ne 0) { throw "Interface build failed." }
pnpm.cmd build:maps
if ($LASTEXITCODE -ne 0) { throw "Map downloader interface build failed." }

Write-Host "Running Rust tests..." -ForegroundColor Cyan
cargo test --workspace --locked
if ($LASTEXITCODE -ne 0) { throw "Rust tests failed." }

Write-Host "Checking Rust formatting..." -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw "Rust formatting check failed." }

if ((rustup component list --installed) -match '^clippy-') {
    Write-Host "Running Clippy..." -ForegroundColor Cyan
    cargo clippy --workspace --all-targets --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "Clippy failed." }
} else {
    Write-Host "Clippy is not installed; skipping that optional check." -ForegroundColor Yellow
}

Write-Host "All available checks passed." -ForegroundColor Green
