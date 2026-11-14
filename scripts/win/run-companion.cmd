@echo off
rem Runs the companion app on this PC (for trying it without a phone).
cd /d "%~dp0..\.."
if not exist _logs mkdir _logs
npm run tauri -w apps/companion -- dev > _logs\companion-dev.log 2>&1
