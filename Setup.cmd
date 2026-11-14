@echo off
rem Double-click to install the build tools and project packages.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup.ps1" %*
echo.
pause
