@echo off
setlocal
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\dev.ps1"
if errorlevel 1 (
  echo.
  echo Waypoint source launch failed. See docs\troubleshooting.md.
  pause
  exit /b 1
)
