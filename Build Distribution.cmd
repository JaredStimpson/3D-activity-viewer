@echo off
setlocal
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-dist.ps1"
if errorlevel 1 (
  echo.
  echo Distribution build failed. See docs\troubleshooting.md.
  pause
  exit /b 1
)
echo.
echo Both release executables were built successfully.
pause
