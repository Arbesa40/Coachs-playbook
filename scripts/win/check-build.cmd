@echo off
rem Builds and tests everything, writing logs to _logs\ (used while developing).
cd /d "%~dp0..\.."
if not exist _logs mkdir _logs
echo running core tests...
call cargo test -p coach-core > _logs\core-test.log 2>&1
echo exit %errorlevel% >> _logs\core-test.log
echo building desktop UI...
call npm run build -w apps/desktop > _logs\ui-build.log 2>&1
echo exit %errorlevel% >> _logs\ui-build.log
echo building companion UI...
call npm run build -w apps/companion > _logs\ui-companion.log 2>&1
echo exit %errorlevel% >> _logs\ui-companion.log
echo compiling desktop app...
call cargo test -p coachs-playbook-desktop > _logs\desktop-build.log 2>&1
echo exit %errorlevel% >> _logs\desktop-build.log
echo compiling companion (desktop target)...
call cargo build -p coachs-playbook-companion > _logs\companion-build.log 2>&1
echo exit %errorlevel% >> _logs\companion-build.log
echo All done. > _logs\check-done.txt
echo Done. You can close this window.
