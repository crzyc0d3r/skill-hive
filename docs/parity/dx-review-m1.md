# M1 design review

Phase 1 exit criterion 6 asks for a design review of every Phase 1 screen on WebKitGTK and WKWebView, in addition to the Playwright checks at 1100x640.

## What CI can review

`e2e/phase1-visual.spec.ts` runs in the frontend workflow on ubuntu-22.04 with Playwright Chromium. For each Phase 1 screen, in light and dark, at 1100 by 640, it checks:

- the screen landmark is visible
- the status bar reads Skill Hive 0.0.1
- `data-theme` matches the requested theme, and dark mode sets the `dark` class
- the page has no horizontal overflow
- a screenshot is written to `docs/parity/screenshots/{light,dark}/`

Screens covered: dashboard, library, install local, install git, install scan, install leaderboard, global workspace, agent page, settings appearance, settings general, settings agents, settings about, and the command palette. An extra check sets Extra Large text and confirms the library skill name is visible and is not truncated.

The frontend job installs Playwright Chromium with system dependencies and runs `npm run test:visual`. That is the headless review this repository can repeat.

## What cannot be done headless

WebKitGTK was not launched. The frontend job has no GTK display session and no WebKitGTK webview harness. Playwright's browser on that job is Chromium, not `libwebkit2gtk`. A screenshot from Chromium does not measure WebKitGTK font hinting, overlay scrollbars, or GTK title-bar metrics.

WKWebView was not launched. It exists only on macOS. This frontend workflow runs on Ubuntu. The macOS leg of the Rust workflow compiles and tests `hive-core`; it does not open a Tauri window. There is no WKWebView process to review.

Playwright's bundled WebKit MiniBrowser is also not WKWebView and not WebKitGTK, so it was not used as a stand-in for either engine.

No reviewer signed a native screen checklist. Criterion 6 is not met until those two engines are reviewed at 1100 by 640 in light and dark, on the routes listed in the Playwright spec.
