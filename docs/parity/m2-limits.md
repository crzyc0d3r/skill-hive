# Phase 2 limits

These items need a person or a display. The automated checks cover the protocol and the menu rules.

## GitHub device flow with a real account

`crates/hive-core/src/backup/github.rs` runs the device-flow, personal-access-token, and private-repo calls against a scripted HTTP client. Signing in with a live GitHub account, copying the one-time code, and approving the organization prompt is a manual check. The public-repo warning string is covered by `public_repo_warning`.

## Tray icon clicks

`crates/hive-core/src/util/tray_menu.rs` builds the status line, update item, Presets apply/remove submenu, Open, Check for skill updates, Open Skills Folder, and Quit. `left_click_restores` is true on Windows and Linux and false on macOS. `src-tauri/src/tray.rs` installs that menu and hides the window when close behaviour is minimize and the tray is enabled. A hidden tray falls back to Ask.

Clicking the operating-system icon, confirming the submenu on a real menu bar, and watching the window restore still needs a desktop session on each operating system. The close dialog itself is covered by `src/components/shell/dialogs.test.tsx` and `e2e/shell-flows.spec.ts`: Ask opens the dialog, and Remember stores `minimize` or `quit`.

## tauri-driver

`.github/workflows/tauri-driver.yml` builds a debug shell and drives it on Linux (under Xvfb) and Windows. The suite opens the dashboard, library, held-back update, project link, marketplace, backup, and restore controls. It does not click the native tray icon.
