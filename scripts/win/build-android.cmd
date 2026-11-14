@echo off
rem Generates the Android project (first time) and builds a debug APK for arm64 phones.
cd /d "%~dp0..\.."
if not exist _logs mkdir _logs
if "%ANDROID_HOME%"=="" set "ANDROID_HOME=%LOCALAPPDATA%\Android\Sdk"
if "%JAVA_HOME%"=="" set "JAVA_HOME=%ProgramFiles%\Android\Android Studio\jbr"
if "%NDK_HOME%"=="" for /d %%d in ("%ANDROID_HOME%\ndk\*") do set "NDK_HOME=%%d"
rem The NDK linker wrapper breaks on apostrophes in paths ("coach's playbook"),
rem so Android build output goes to a folder without one.
set "CARGO_TARGET_DIR=%LOCALAPPDATA%\coachs-playbook\target"
echo CARGO_TARGET_DIR=%CARGO_TARGET_DIR% ANDROID_HOME=%ANDROID_HOME% NDK_HOME=%NDK_HOME% > _logs\android.log
if not exist apps\companion\src-tauri\gen\android (
  echo generating Android project...
  call npm run tauri -w apps/companion -- android init --ci >> _logs\android.log 2>&1
)
echo building APK...
call npm run tauri -w apps/companion -- android build --apk --debug --target aarch64 >> _logs\android.log 2>&1
echo exit %errorlevel% >> _logs\android.log
echo Done.
