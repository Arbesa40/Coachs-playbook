<p align="center">
  <img src="branding/lockup-dark.png#gh-light-mode-only" alt="Coach's Playbook" height="140">
  <img src="branding/lockup-light.png#gh-dark-mode-only" alt="Coach's Playbook" height="140">
</p>

<p align="center"><b>Free, open-source video analysis and squad management for coaches.</b><br>
Windows desktop app + Android companion. Local-first: no accounts, no cloud, no telemetry.</p>

---

## What it does

**Desktop (Windows)**

- **Video analysis**: import match video, tag events with buttons or hotkeys, timeline with filters, clips and playlists, drawing tools and freeze frames, side-by-side compare, stats, MP4 export, pop-out or mirrored video on a second screen, named analysis sessions.
- **Matches**: fixtures, line-ups grouped by position, and live match recording with each sport's rules (see below). Minutes played are worked out from the log.
- **Players**: profiles, availability and injuries, rating templates and assessments with progress charts.
- **Training**: session planner with drills, timings and instructions for each position group; drill library with sport-specific categories and starter drills; attendance.
- **Tactics**: a library of moves, set pieces and patterns per sport, drawn on an animated tactics board (record player and ball movement step by step) and shown on a second screen for team talks.
- **Reports**: match, player and session reports to print or save as PDF; share packages (ZIP with an offline viewer).
- **Data**: CSV import/export, Sportscode XML export, backup and restore, light and dark themes.

**Android companion**

Pitch-side match recording, training session planning and attendance, the tactics library with a touch tactics board (full screen in landscape), squad availability and assessments, voice/photo notes and video recording. It syncs with the PC over Wi-Fi or the phone's hotspot. No server is involved.

### Sports and formats

| Sport | Formats | Rules built in |
|---|---|---|
| Football | 11-a-side, 7-a-side, futsal | goals and own goals, yellow/red, second yellow, substitution windows, futsal team fouls and time-outs |
| Rugby union | 15s, 7s | try / conversion / penalty / drop goal / penalty try, sin bin (10 min in 15s, 2 min in 7s) with countdown, red card (optional 20-minute red), blood and HIA replacements, TMO |
| Basketball | 5x5, 3x3 | 1-2-3 point scoring (1-2 in 3x3), personal fouls and foul-out, team-foul bonus, time-outs per half, first to 21 in 3x3 |
| Field hockey | 11-a-side, Hockey5s | field goals, penalty corners and strokes, green (2 min), yellow (5/10 min) and red cards |
| Volleyball | Indoor, beach | rally scoring by set (25/15, 21/15, win by 2), sanctions with penalty points, time-outs |
| Netball | Standard | goals by shooters only (GS, GA), optional Super Shot, 2-minute suspensions |

Rules differ between competitions and age groups. Period lengths can be changed per match, and optional variations are switched on per team in **Settings**. If your competition plays differently, please open an issue.

## Install

Release builds are published on the [Releases](../../releases) page (Windows installer and Android APK). To build it yourself, see below.

## Build from source (Windows)

```powershell
# from the project folder, in PowerShell
powershell -ExecutionPolicy Bypass -File .\setup.ps1     # installs the toolchain and packages, runs the tests
npm run desktop                                         # desktop app in development mode
```

`setup.ps1` installs only what's missing: Git, Node.js LTS, Rust (MSVC), Visual Studio 2022 C++ Build Tools, WebView2, FFmpeg and, for Android, Java 17, the Android SDK + NDK and the Rust Android targets. Use `-SkipAndroid` for the desktop only.

| Task | Command |
|---|---|
| Desktop app (development) | `npm run desktop`, or double-click `scripts\win\run-desktop.cmd` |
| Build and test everything (logs in `_logs\`) | double-click `scripts\win\check-build.cmd` |
| Windows installer | `scripts\fetch-ffmpeg.ps1`, then `npm run desktop:build` → `target\release\bundle\nsis\` |
| Core tests | `npm test` |
| Android debug APK | double-click `scripts\win\build-android.cmd` → `_build\` |
| Run on a phone (USB debugging on) | `npm run android` |
| Release APK | `npm run android:build` |

Development builds load the interface from the Vite dev server, so start them with the commands above rather than running `target\debug\*.exe` directly. Avoid apostrophes in the project path: the Windows resource compiler and the Android NDK linker can't handle them.

## Project layout

```
crates/coach-core/     Rust core shared by both apps: SQLite record store with per-field
                       last-writer-wins clocks, sync protocol, pairing, encryption, stats, export
apps/desktop/          Tauri 2 + Svelte 5 Windows app (FFmpeg media queue, share packages, sync server)
apps/companion/        Tauri 2 Android app (same core, shared UI)
packages/ui/           shared Svelte/JS: API wrappers, styles and themes, icons, sport rules
                       (sports.js, match.js), tactics board, match console
templates/             sport templates as data (positions, default tagging panels)
locales/               interface translations
branding/              logo files
samples/               example CSV for squad import
docs/                  status and notes
```

## How data and sync work

Every entity is a record (`kind`, `id`, JSON fields) in one SQLite file per team. Each field carries a hybrid-logical-clock timestamp, so edits to different fields on the PC and the phone both survive, and the newer write wins on the same field. Attendance, notes, injuries, assessments and match events are append-only, so they never conflict. When the same field was changed on both devices since their last sync, the PC shows it under **Sync & data → Needs a decision**.

Pairing: the PC shows a QR code with its LAN addresses, port 47820 and a one-time key. The phone keeps the key; all traffic is encrypted with it and never leaves the local network. Without a network, use **Save/Load .sync file** (USB, Bluetooth, email). If the phone can't connect, allow Coach's Playbook through Windows Firewall on private networks.

Deleting a team moves its database to the `deleted-teams` folder inside the data folder, so it can be recovered.

## Contributing

Issues and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md). Sport rules live in `packages/ui/sports.js`, so corrections from coaches who know a sport well are especially useful. Translations go in `locales/`.

## Licence

[GPL-3.0-or-later](LICENSE). Icons follow the [Lucide](https://lucide.dev) style (ISC licence).
