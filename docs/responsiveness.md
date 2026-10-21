# Responsiveness

Pat's Windows build felt jammed because the library command read every skill file into one IPC payload, and almost every Tauri command ran that disk, git, network, and database work on the caller. These timings are from `cargo test -p skill-hive --lib profile_library_load` on Linux with 200 skills (each has `SKILL.md`, `notes.md`, and `refs/a.md`) and 4 projects.

## Before

The list command embedded every file body.

```
PROFILE seed_ms=66.4 list_ms=7.0 skills=200 payload_bytes=2612160 projects_ms=0.1 projects=4 presets_ms=0.0 presets=4
```

`list_ms=7.0` is only the Rust build. The 2,612,160-byte JSON still had to cross IPC and be parsed on the webview. On a warm Linux disk that list step was 7.0 ms. Windows WebView2 pays more for the same payload, and the command blocked the UI thread while it read the files. Seeding the 200 directories took 66.4 ms. Projects took 0.1 ms. Presets took 0.0 ms.

Startup also ran the daily auto-update and auto-backup on the Tauri setup thread, after the window existed and before the webview could settle.

A separate frontend render timing for the old 200-card list was not captured. The library view capped the painted rows at 100, so skills 101–200 were not reachable.

## After

The list command returns metadata only. File bodies load when a skill is opened.

```
PROFILE seed_ms=67.0 list_ms=0.7 skills=200 payload_bytes=0 projects_ms=0.0 projects=4 presets_ms=0.0 presets=4
```

`payload_bytes` counts embedded skill bodies and file text, not names or tags. That embedded text dropped by 2612160 bytes, to `payload_bytes=0`. `list_ms=0.7`. Projects `0.0` ms. Presets `0.0` ms. Same 200 skills and 4 projects.

Commands that touch the database, the filesystem, git, the network, or a process are `async` and run the blocking work in `tauri::async_runtime::spawn_blocking` via `crate::offload::offload`. `scripts/check-async-commands.mjs` fails the frontend CI job if a synchronous command body still contains those markers. Window exit, restart, and hide stay synchronous because they have to run on the UI thread.

The one-shot auto-update and auto-backup moved onto that same blocking pool during setup. Opening the database still happens on the setup thread so commands cannot run before the store exists.

Search and filter fields wait 150 ms. The library list paints a window of rows when the scroll parent is at least 40 px tall, so a 200-skill library stays reachable without mounting every card. Repeated catalog invokes share one in-flight promise.

Long jobs emit `job-progress` (library load, scan, install, import, backup sync, update apply, effectiveness run) with a progress bar and Cancel. Cancel sets a flag the job checks between steps.

## Loading states

Every routed screen renders a loading frame with skeleton blocks when `?state=loading` is set, or when the live skill query is still pending outside demo mode. Query failures render the same frame with Retry. Action buttons set `aria-busy` and show a spinner while they are disabled.

Screenshots, light and dark, at 1100×640 and 1440×860:

- `docs/parity/screenshots/light/loading-dashboard.png`
- `docs/parity/screenshots/dark/loading-dashboard.png`
- `docs/parity/screenshots/light/loading-library.png`
- `docs/parity/screenshots/dark/loading-library.png`
- `docs/parity/screenshots/light/loading-agents.png`
- `docs/parity/screenshots/dark/loading-agents.png`
- `docs/parity/screenshots/light/loading-install.png`
- `docs/parity/screenshots/dark/loading-install.png`
- `docs/parity/screenshots/light/loading-backup.png`
- `docs/parity/screenshots/dark/loading-backup.png`
- `docs/parity/screenshots/light/loading-project.png`
- `docs/parity/screenshots/dark/loading-project.png`
- `docs/parity/screenshots/light/loading-settings.png`
- `docs/parity/screenshots/dark/loading-settings.png`
- `docs/parity/screenshots/light/loading-endpoints.png`
- `docs/parity/screenshots/dark/loading-endpoints.png`
- `docs/parity/screenshots/light/loading-workspace.png`
- `docs/parity/screenshots/dark/loading-workspace.png`
- `docs/parity/screenshots/light/loading-effectiveness.png`
- `docs/parity/screenshots/dark/loading-effectiveness.png`
- `docs/parity/screenshots/light/loading-suite.png`
- `docs/parity/screenshots/dark/loading-suite.png`
- `docs/parity/screenshots/light/loading-results.png`
- `docs/parity/screenshots/dark/loading-results.png`
- `docs/parity/screenshots/light/loading-compare.png`
- `docs/parity/screenshots/dark/loading-compare.png`
- `docs/parity/screenshots/light/loading-history.png`
- `docs/parity/screenshots/dark/loading-history.png`
- `docs/parity/screenshots/light/loading-job.png`
- `docs/parity/screenshots/dark/loading-job.png`

The 1440×860 frames use the same names with a `-1440` suffix, except the job bar, which is captured at 1100×640.
