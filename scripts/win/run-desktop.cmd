@echo off
rem Runs the desktop app in development mode.
cd /d "%~dp0..\.."
if not exist _logs mkdir _logs
npm run desktop > _logs\desktop-dev.log 2>&1
