@echo off
setlocal
cd /d "%~dp0"

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\launch.ps1" -SetupIfMissing
if errorlevel 1 (
  echo.
  echo Waypoint could not start. See docs\troubleshooting.md for help.
  pause
  exit /b 1
)
