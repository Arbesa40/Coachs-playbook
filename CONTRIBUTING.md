# Contributing to Coach's Playbook

Thanks for helping. Coaches, developers and translators are all welcome.

## Ways to help

- **Report a bug**: open an issue with what you did, what you expected and what happened. Include the Windows or Android version and, for build problems, the log from `_logs\`.
- **Fix sport rules**: rules, positions, drill categories and tactic types for each sport live in `packages/ui/sports.js`. Match logic (scores, sin bins, fouls, sets) is in `packages/ui/match.js`. If your competition plays differently, describe the rule and where it applies.
- **Translate**: copy `locales/en.json` to your language code (for example `sw.json`) and translate the values.
- **Code**: pick an issue, or open one first for larger changes so we can agree on the approach.

## Development setup

See "Build from source" in the [README](README.md). In short, on Windows:

```powershell
powershell -ExecutionPolicy Bypass -File .\setup.ps1
npm run desktop
```

Before opening a pull request, run `scripts\win\check-build.cmd` (or `npm test` plus `npm run build -w apps/desktop` and `npm run build -w apps/companion`) and make sure everything passes.

## Guidelines

- **Performance**: the targets are 4 GB RAM laptops with integrated graphics and 2 GB Android phones. Avoid heavy dependencies, big shadows, blur and animations that run all the time.
- **Local-first**: no accounts, servers, analytics or network calls beyond the LAN sync between a coach's own devices.
- **Shared UI**: put anything both apps use in `packages/ui`. Use the CSS variables in `packages/ui/styles.css` (never hard-coded colours) so light and dark themes keep working.
- **Dialogs**: in the Tauri apps `window.confirm` returns a Promise. Use `await ask(...)` from `packages/ui/api.js`.
- **Records**: data is stored as records (`db.put(kind, fields, id)`). Prefer new append-only records for events so the PC and phone never conflict.
- **Style**: Svelte 5 runes, plain JavaScript, `cargo fmt` for Rust. Keep comments short and about why, not what.

## Licence

By contributing you agree that your work is released under the [GPL-3.0-or-later](LICENSE) licence.
