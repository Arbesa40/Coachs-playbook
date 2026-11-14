@echo off
rem Writes the versions of the build tools on this PC to _logs\tools.log
cd /d "%~dp0..\.."
if not exist _logs mkdir _logs
(
  echo == %date% %time%
  for %%c in (git node npm rustup rustc cargo ffmpeg winget java) do (
    echo -- %%c
    where %%c 2^>nul
  )
  node -v 2>&1
  rustc --version 2>&1
  cargo --version 2>&1
  rustup show 2>&1
  echo JAVA_HOME=%JAVA_HOME%
  echo ANDROID_HOME=%ANDROID_HOME%
  echo NDK_HOME=%NDK_HOME%
  dir /b "%LOCALAPPDATA%\Android\Sdk" 2>&1
  dir /b "%LOCALAPPDATA%\Android\Sdk\ndk" 2>&1
  dir /b "%ProgramFiles%\Android" 2>&1
  "%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -products * -property displayName 2>&1
  reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv 2>&1
) > _logs\tools.log 2>&1
echo Done. You can close this window.
