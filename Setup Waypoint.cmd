@echo off
setlocal
cd /d "%~dp0"

echo Waypoint setup
echo ==============
echo This will check prerequisites, install missing tools with WinGet, and build Waypoint.
echo.

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\setup.ps1" -InstallMissing
if errorlevel 1 (
  echo.
  echo Setup did not complete. See docs\troubleshooting.md for help.
  pause
  exit /b 1
)

echo.
echo Setup complete. You can now double-click "Launch Waypoint.cmd".
pause
