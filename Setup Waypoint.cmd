@echo off
setlocal
cd /d "%~dp0"

echo Waypoint setup
echo ==============
echo This checks prerequisites, installs missing tools with WinGet, and restores dependencies.
echo It does not build release executables.
echo.

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\setup.ps1" -InstallMissing
if errorlevel 1 (
  echo.
  echo Setup did not complete. See docs\troubleshooting.md for help.
  pause
  exit /b 1
)

echo.
echo Setup complete. Use a "Run ... from Source" launcher, or build a distribution.
pause
