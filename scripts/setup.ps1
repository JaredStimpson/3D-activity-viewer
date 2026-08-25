[CmdletBinding()]
param(
    [switch]$InstallMissing,
    [switch]$SkipBuild,
    [switch]$CheckOnly
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$PnpmVersion = "11.19.0"

function Write-Step([string]$Message) {
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}
function Refresh-ProcessPath {
    $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $env:Path = "$machinePath;$userPath"
}

function Test-Command([string]$Name) {
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-MsvcBuildTools {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path -LiteralPath $vswhere)) {
        return $false
    }
    $installation = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    return -not [string]::IsNullOrWhiteSpace($installation)
}

function Install-WingetPackage([string]$Id, [string[]]$AdditionalArguments = @()) {
    if (-not (Test-Command "winget")) {
        throw "WinGet is required for automatic setup. Install 'App Installer' from Microsoft Store, then run setup again."
    }
    $arguments = @(
        "install", "--id", $Id, "--exact", "--source", "winget",
        "--accept-source-agreements", "--accept-package-agreements", "--silent"
    ) + $AdditionalArguments
    & winget @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "WinGet could not install $Id (exit code $LASTEXITCODE)."
    }
}

if (-not $IsWindows -and $PSVersionTable.PSEdition -eq "Core") {
    throw "This setup script currently supports Windows only."
}

Write-Step "Checking Windows build prerequisites"
$missing = [System.Collections.Generic.List[string]]::new()
if (-not (Test-Command "node")) { $missing.Add("Node.js") }
if (-not (Test-Command "cargo") -or -not (Test-Command "rustc")) { $missing.Add("Rust") }
if (-not (Test-Command "ffmpeg") -or -not (Test-Command "ffprobe")) { $missing.Add("FFmpeg") }
if (-not (Test-MsvcBuildTools)) { $missing.Add("Visual Studio C++ Build Tools") }

if ($missing.Count -gt 0) {
    Write-Host "Missing: $($missing -join ', ')" -ForegroundColor Yellow
    if ($CheckOnly) {
        exit 1
    }
    if (-not $InstallMissing) {
        Write-Host "Run .\scripts\setup.ps1 -InstallMissing to install them automatically."
        exit 1
    }

    if ($missing.Contains("Node.js")) {
        Write-Step "Installing Node.js LTS"
        Install-WingetPackage "OpenJS.NodeJS.LTS"
    }
    if ($missing.Contains("Rust")) {
        Write-Step "Installing Rust"
        Install-WingetPackage "Rustlang.Rustup"
    }
    if ($missing.Contains("FFmpeg")) {
        Write-Step "Installing FFmpeg"
        Install-WingetPackage "Gyan.FFmpeg"
    }
    if ($missing.Contains("Visual Studio C++ Build Tools")) {
        Write-Step "Installing Visual Studio C++ Build Tools"
        Install-WingetPackage "Microsoft.VisualStudio.2022.BuildTools" @(
            "--override",
            "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
        )
    }
    Refresh-ProcessPath
}

if (-not (Test-Command "node") -or -not (Test-Command "cargo") -or -not (Test-Command "ffmpeg")) {
    throw "A prerequisite was installed but is not available yet. Restart Windows, then run setup again."
}

$nodeMajor = [int]((& node --version).TrimStart("v").Split(".")[0])
if ($nodeMajor -lt 20) {
    throw "Waypoint needs Node.js 20 or newer. The installed version is $(& node --version)."
}

if (-not (Test-Command "pnpm")) {
    if (-not (Test-Command "corepack")) {
        throw "Corepack was not found. Reinstall the current Node.js LTS release, then run setup again."
    }
    Write-Step "Activating pnpm $PnpmVersion"
    & corepack enable
    if ($LASTEXITCODE -ne 0) { throw "Corepack could not enable pnpm." }
    & corepack prepare "pnpm@$PnpmVersion" --activate
    if ($LASTEXITCODE -ne 0) { throw "Corepack could not activate pnpm $PnpmVersion." }
    Refresh-ProcessPath
}

Write-Host "Node:    $(& node --version)" -ForegroundColor Green
Write-Host "pnpm:    $(& pnpm --version)" -ForegroundColor Green
Write-Host "Rust:    $(& rustc --version)" -ForegroundColor Green
Write-Host "FFmpeg:  $((& ffmpeg -version | Select-Object -First 1) -replace '^ffmpeg version ', '')" -ForegroundColor Green
Write-Host "MSVC:    installed" -ForegroundColor Green

if ($CheckOnly) {
    Write-Host "`nWaypoint prerequisites are ready." -ForegroundColor Green
    exit 0
}

Set-Location -LiteralPath $RepositoryRoot
Write-Step "Installing locked project dependencies"
& pnpm install --frozen-lockfile
if ($LASTEXITCODE -ne 0) { throw "JavaScript dependency installation failed." }

Write-Step "Preparing Rust dependencies"
& cargo fetch --locked
if ($LASTEXITCODE -ne 0) { throw "Rust dependency preparation failed." }

if (-not $SkipBuild) {
    Write-Step "Building the Waypoint Windows application"
    & pnpm tauri:build
    if ($LASTEXITCODE -ne 0) { throw "Waypoint build failed." }

    $executable = Join-Path $RepositoryRoot "target\release\waypoint-desktop.exe"
    if (-not (Test-Path -LiteralPath $executable)) {
        throw "The build completed but the Waypoint executable was not found."
    }
    Write-Host "`nWaypoint is ready:" -ForegroundColor Green
    Write-Host $executable
} else {
    Write-Host "`nDependencies are ready. The application build was skipped." -ForegroundColor Green
}
