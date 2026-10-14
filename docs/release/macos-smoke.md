# macOS smoke checklist

Run this on a Mac before a signed release. Unsigned drafts cannot satisfy Gatekeeper.

| Check | Result for the unsigned 1.0.0 draft |
|---|---|
| Window opens at 1440x860 and does not scroll horizontally at 1100x640 | Covered by the shell layout and the minimum window in `src-tauri/tauri.conf.json` |
| Sidebar collapse, command palette, and light, dark, and system theme | Covered by `src/test/shell.test.tsx` and `src/test/phase1.test.tsx` |
| Tray preset and close guard | Covered by `src/test/phase2.test.tsx` |
| About shows the MIT notice and Check for Updates notifies only | Covered by `src/test/phase6.test.tsx` |
| Keychain prompt when an endpoint secret is saved | Manual on a signed build. Linux CI covers the disk fallback |
| Gatekeeper opens the `.dmg` without a warning | Waiting on `APPLE_CERTIFICATE` and the App Store Connect API key. `scripts/verify-macos-signature.sh` runs `codesign`, `spctl`, and `stapler` when those secrets are present |
| Updater Install Update then Restart Now | UI covered by `src/test/phase6.test.tsx`. A real package update waits on `TAURI_SIGNING_PRIVATE_KEY` |

Signed off for the automated rows by the Phase 6 release notes. Gatekeeper and a live update stay open until the secrets in `secrets.md` are added.
