<#
  Coach's Playbook - developer setup for Windows 10/11.

  Installs (only what's missing) with winget:
    Git, Node.js LTS, Rust (MSVC), Visual Studio 2022 C++ Build Tools,
    WebView2 runtime, FFmpeg, and for the Android app: JDK 17 + Android SDK/NDK
  Then installs the project's npm packages and runs the core tests.

  Usage (PowerShell, from the project folder):
    powershell -ExecutionPolicy Bypass -File .\setup.ps1
    powershell -ExecutionPolicy Bypass -File .\setup.ps1 -SkipAndroid
#>
param(
  [switch]$SkipAndroid,
  [switch]$SkipBuildTools
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
Set-Location -LiteralPath $PSScriptRoot
New-Item -ItemType Directory -Force -Path "$PSScriptRoot\_logs" | Out-Null
Start-Transcript -Path "$PSScriptRoot\_logs\setup.log" -Force | Out-Null

function Say($msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Ok($msg) { Write-Host "    OK  $msg" -ForegroundColor Green }
function Warn($msg) { Write-Host "    !!  $msg" -ForegroundColor Yellow }

function Refresh-Path {
  $machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
  $user = [Environment]::GetEnvironmentVariable("Path", "User")
  $env:Path = "$machine;$user;$env:USERPROFILE\.cargo\bin"
}

function Has($cmd) { return [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }

function Winget-Install($id, $extra = @()) {
  Write-Host "    installing $id ..." -ForegroundColor DarkGray
  $wargs = @("install", "--id", $id, "-e", "--accept-source-agreements", "--accept-package-agreements", "--silent") + $extra
  & winget @wargs
  if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne -1978335189) {
    # -1978335189 = already installed / no upgrade available
    Warn "winget returned $LASTEXITCODE for $id"
  }
  Refresh-Path
}

if (-not (Has "winget")) {
  throw "winget is missing. Install 'App Installer' from the Microsoft Store, then run this script again."
}

Refresh-Path

# ---------------------------------------------------------------- core tools
Say "Git"
if (Has "git") { Ok (git --version) } else { Winget-Install "Git.Git" }

Say "Node.js"
if (Has "node") { Ok "node $(node -v)" } else { Winget-Install "OpenJS.NodeJS.LTS" }

Say "Rust"
if (-not (Has "rustup")) { Winget-Install "Rustlang.Rustup" }
if (Has "rustup") {
  rustup default stable-msvc | Out-Null
  rustup update stable | Out-Null
  Ok (rustc --version)
} else { Warn "rustup not on PATH yet. Open a new terminal and re-run this script." }

Say "Visual Studio C++ Build Tools (needed to compile Rust on Windows)"
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasVC = $false
if (Test-Path $vswhere) {
  $found = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  if ($found) { $hasVC = $true }
}
if ($hasVC) { Ok "C++ build tools found" }
elseif ($SkipBuildTools) { Warn "Skipped. Rust builds will fail without the MSVC linker." }
else {
  Write-Host "    This is a large download (~2-4 GB). Accept the Windows admin prompt when it appears." -ForegroundColor Yellow
  Winget-Install "Microsoft.VisualStudio.2022.BuildTools" @("--override", "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended")
}

Say "WebView2 runtime"
$wv = Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
if ($wv) { Ok "WebView2 $($wv.pv)" } else { Winget-Install "Microsoft.EdgeWebView2Runtime" }

Say "FFmpeg (video proxies and clip export)"
if (Has "ffmpeg") { Ok ((ffmpeg -hide_banner -version | Select-Object -First 1)) } else { Winget-Install "Gyan.FFmpeg" }

# ---------------------------------------------------------------- android
if (-not $SkipAndroid) {
  Say "Android toolchain (companion app)"

  # Java 17: prefer Android Studio's bundled JBR, else install Microsoft OpenJDK 17.
  $javaHome = $env:JAVA_HOME
  $studioJbr = "$env:ProgramFiles\Android\Android Studio\jbr"
  if (-not $javaHome -or -not (Test-Path "$javaHome\bin\java.exe")) {
    if (Test-Path "$studioJbr\bin\java.exe") { $javaHome = $studioJbr }
    else {
      Winget-Install "Microsoft.OpenJDK.17"
      $jdk = Get-ChildItem "$env:ProgramFiles\Microsoft" -Directory -Filter "jdk-17*" -ErrorAction SilentlyContinue | Select-Object -Last 1
      if ($jdk) { $javaHome = $jdk.FullName }
    }
  }
  if ($javaHome) {
    [Environment]::SetEnvironmentVariable("JAVA_HOME", $javaHome, "User")
    $env:JAVA_HOME = $javaHome
    Ok "JAVA_HOME = $javaHome"
  } else { Warn "Could not find Java 17. Install Android Studio, then re-run." }

  $sdk = $env:ANDROID_HOME
  if (-not $sdk) { $sdk = "$env:LOCALAPPDATA\Android\Sdk" }
  if (-not (Test-Path $sdk)) {
    Warn "Android SDK not found at $sdk"
    Write-Host "    Installing Android Studio. When it opens the first time, finish the setup wizard (it downloads the SDK), then run this script again." -ForegroundColor Yellow
    Winget-Install "Google.AndroidStudio"
  } else {
    [Environment]::SetEnvironmentVariable("ANDROID_HOME", $sdk, "User")
    $env:ANDROID_HOME = $sdk
    Ok "ANDROID_HOME = $sdk"

    $sdkmanager = Get-ChildItem "$sdk\cmdline-tools" -Recurse -Filter "sdkmanager.bat" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($sdkmanager) {
      Write-Host "    installing SDK platform, build tools and NDK (accepting licenses)..." -ForegroundColor DarkGray
      $yes = ("y`n" * 20)
      $yes | & $sdkmanager.FullName --licenses | Out-Null
      & $sdkmanager.FullName "platform-tools" "platforms;android-34" "build-tools;34.0.0" "ndk;27.2.12479018" | Out-Null
    } else {
      Warn "Android 'Command-line Tools' are missing. In Android Studio: Settings > Android SDK > SDK Tools, tick 'Android SDK Command-line Tools' and 'NDK (Side by side)'."
    }
    $ndk = Get-ChildItem "$sdk\ndk" -Directory -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -Last 1
    if ($ndk) {
      [Environment]::SetEnvironmentVariable("NDK_HOME", $ndk.FullName, "User")
      $env:NDK_HOME = $ndk.FullName
      Ok "NDK_HOME = $($ndk.FullName)"
    } else { Warn "No NDK found yet." }
  }

  if (Has "rustup") {
    rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android | Out-Null
    Ok "Rust Android targets added"
  }
}

# ---------------------------------------------------------------- project
Say "Project packages (npm install)"
if (Has "npm") {
  npm install --no-audit --no-fund
  Ok "npm packages installed"
} else { Warn "npm not on PATH yet. Open a new terminal and re-run." }

if (-not (Test-Path ".\LICENSE.txt")) {
  try {
    Invoke-WebRequest "https://www.gnu.org/licenses/gpl-3.0.txt" -OutFile ".\LICENSE.txt" -UseBasicParsing
    Ok "GPLv3 license text saved to LICENSE.txt"
  } catch { Warn "Couldn't download the GPLv3 text (offline?)." }
}

Say "Core tests (cargo test -p coach-core)"
if (Has "cargo") {
  cmd /c "cargo test -p coach-core > _logs\core-test.log 2>&1"
  Get-Content _logs\core-test.log -Tail 15
  if ($LASTEXITCODE -eq 0) { Ok "core tests passed" } else { Warn "core tests failed - see output above" }
}

Say "Done"
Write-Host @"

  Run the desktop app:        npm run desktop
  Build the Windows installer: npm run desktop:build
  Android (first time):       npm run android:init
  Run on a phone (USB debug): npm run android
  Build the APK:              npm run android:build

  If a tool was just installed, close this window and open a new terminal first.
"@
Stop-Transcript | Out-Null
