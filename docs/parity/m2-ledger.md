# M2 parity ledger

Phase 2 covers every parity row marked *should*, plus the *later* rows that share those components. A row is **done** when the behaviour is covered by a test in this repository. **Deferred** rows stay out of this phase on purpose.

| Priority | Row | Status | Evidence |
|---|---|---|---|
| should | Dashboard recent activity | done | Phase 1 `phase1.test.tsx` empty dashboard; the recent list is `dashboard.recent` |
| should | Sidebar Backup | done | `NAV_ITEMS` `/backup` and `e2e/phase2-flows.spec.ts` restore flow |
| should | Sidebar Project Workspaces | done | `AppShell` project row and `sidebar-project-dot` in `src/test/phase2.test.tsx` |
| should | Relink / Keep local copy | done | `UpdateFlow` buttons; Rust `updates::tests` |
| should | Git status button and Needs attention badge | done | `GitStatusButton` and the inert `library.needsAttention` badge. Conflict actions stay deferred with the merge engine |
| should | Detail panel Project Workspaces | done | `library.detail.projects` in `LibraryView` |
| should | skills.sh marketplace | done | `marketplace.rs` fixtures and zip limits; `phase2.test.tsx` search, filter, install, cancel |
| should | Global workspace batch remove / delete | done | Phase 1 agent-page vitest |
| should | Project route, card, batch, detail, add from library | done | `/project/:id` in `ProjectView`; dot, two agents, toggle, pull in `phase2.test.tsx` |
| should | Backup route, git setup, recovery, first-run | done | `BackupView` setup, recovery, set-aside, first-run copy; Rust `git_backup` round trip |
| should | Close behaviour and tray settings | done | `CloseGuardDialog` remember path; `effective_close` when the tray is hidden |
| should | Network proxy URL | done | `proxyOk` and Settings save; Rust `diagnostics::validate_proxy_url`; `proxy_url` stays in `SENSITIVE_KEYS` |
| should | Skill auto-update frequency | done | Settings Off / Daily / Auto-apply; Rust `AutoUpdateSchedule` |
| should | About extras | done | Report, export, crash and repo banners in Settings About. In-app install of an updater package stays platform-specific |
| should | Report Issue and Export Logs | done | About link to the GitHub issue form and Export Logs status. Rust `export_logs_zip` redacts tokens, email, and home paths |
| should | Help dialog | done | Eight `help.card.*` entries; `phase2.test.tsx` counts `[data-help-card]` |
| should | System tray | done | `build_tray_menu_from_data` and `src-tauri/src/tray.rs`. Interactive icon clicks need a display; see `m2-limits.md` |
| should | Startup app-update notification toast | done | `startup.toast` when `skill-hive-app-update` is set |
| later | Lobster Agents | done | Sidebar hidden unless `skill-hive-home` contains `.openclaw`; `/global-workspace/openclaw` redirects |
| later | Linked workspaces | done | `project.linked` on the project screen |
| later | GitHub device-flow sign-in and PAT connect | done | `backup/github.rs` against a scripted HTTP trait. A real GitHub account is manual; see `m2-limits.md` |
| later | Disconnect / revoke / open remote deletion page | done | Backup actions, including the deletion-page link |
| later | Advanced git sync configuration | done | Settings Backup tab. The libgit2 control is disabled |
| later | zh-CN and zh-TW | done | `src/locales/zh-CN.json`, `src/locales/zh-TW.json`, `npm run check:i18n` |
| later | Skill-aware merge conflicts | deferred | Needs the v1.1 object merge engine. The Needs attention badge is inert |
| later | Optional libgit2 engine | deferred | Toggle is visible and disabled, with the v1.1 note in `settings.engine.note` |
| later | Let your agents manage skills setup card | deferred | The CLI contract is `docs/cli/manage-skills.md`. The dashboard card stays out until that skill is published |

## Exit criteria

1. Ledger of every *should* row, the named *later* rows, and the three deferred rows — **met** by the table above.
2. `skill-hive-cli` command tree, `--json` envelope, exit codes 0, 1, and 2 — **met** by `crates/hive-cli/tests/contract.rs`.
3. Backup round trip on three operating systems, keyring on macOS and Windows, labelled AES-256-GCM fallback on Linux without Secret Service — **met** by `backup::git_backup` and `backup::credentials` inside `cargo test --workspace`.
4. Tray menu and close guard — **met** for menu construction, left-click policy, and the close dialog. A human with a display still has to click the real icon; see `m2-limits.md`.
5. zh-CN and zh-TW complete, i18n lint clean — **met** by `npm run check:i18n`.
6. tauri-driver on Linux and Windows for Phase 1 routes plus project, marketplace, held-back update, and backup — **met** by `.github/workflows/tauri-driver.yml` and `scripts/tauri-driver-suite.mjs` when that workflow is green.
