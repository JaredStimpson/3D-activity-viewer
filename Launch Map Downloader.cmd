@echo off
setlocal
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\launch-map-downloader.ps1"
if errorlevel 1 (
  echo.
  echo Map Downloader could not start. See docs\troubleshooting.md.
  pause
  exit /b 1
)
