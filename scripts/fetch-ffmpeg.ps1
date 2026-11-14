<#
  Copies ffmpeg.exe into the desktop app's bundled resources for release builds.

  Size matters: the installer target is ~100 MB. Full FFmpeg builds are 80-130 MB.
  For releases, use a minimal LGPL/GPL build with only what the app needs
  (decoders for common camera/phone formats, libx264 + aac encoders, mp4/mov/mkv/avi
  demuxers, mp4 muxer, scale/overlay filters), which is ~15-25 MB.
  Pass -Source to point at such a build; otherwise the ffmpeg on PATH is used.
#>
param([string]$Source)

$dest = Join-Path $PSScriptRoot "..\apps\desktop\src-tauri\resources\ffmpeg"
New-Item -ItemType Directory -Force -Path $dest | Out-Null

if (-not $Source) {
  $cmd = Get-Command ffmpeg -ErrorAction SilentlyContinue
  if (-not $cmd) { throw "ffmpeg not found. Run setup.ps1 first or pass -Source C:\path\to\ffmpeg.exe" }
  $Source = $cmd.Source
  # winget's Gyan build puts a shim on PATH; find the real exe next to it if needed.
  if ((Get-Item $Source).Length -lt 1MB) {
    $real = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Recurse -Filter ffmpeg.exe -ErrorAction SilentlyContinue | Sort-Object Length -Descending | Select-Object -First 1
    if ($real) { $Source = $real.FullName }
  }
}

Copy-Item -LiteralPath $Source -Destination (Join-Path $dest "ffmpeg.exe") -Force
$mb = [math]::Round((Get-Item (Join-Path $dest "ffmpeg.exe")).Length / 1MB, 1)
Write-Host "Copied ffmpeg.exe ($mb MB) into resources\ffmpeg"
if ($mb -gt 40) { Write-Host "Warning: this build is large; use a minimal build for releases to stay under ~100 MB." -ForegroundColor Yellow }
