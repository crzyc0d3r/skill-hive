# Wiring audit

Status is the behavior of each interactive control in `src/features/**` and `src/App.tsx` against `PLAN.md`.

| Status | Meaning |
| --- | --- |
| real IPC | The control calls a Tauri command that reads or writes `hive.db` or the skills directory. |
| local-state-only | The control used to change React state and stop there. |
| unwired | The control had no handler. |
| demo-only | The control only did something when `localStorage["skill-hive-demo"] === "1"`. |

Demo mode still short-circuits catalog reads when that key is set. A normal `tauri dev` run does not set it.

| Screen | Control | PLAN.md requirement | Before | After |
| --- | --- | --- | --- | --- |
| Install | Local Install | Copy a chosen skill folder into the central skills directory and insert a skill row | unwired | real IPC `install_local_skill` plus folder picker |
| Install | Install archive | Install a `.zip` or `.skill` archive | unwired | real IPC `install_archive_skill` plus file picker |
| Install | Clone and Preview | Preview skills in a git URL or local repo, including a bare repo | unwired | real IPC `preview_git_install` |
| Install | Import Selected | Copy the previewed skill and mark its source `git` | unwired | real IPC `confirm_git_install_skill` |
| Install | Cancel clone | Delete the temporary preview checkout | unwired | real IPC `cancel_install` |
| Install | Import one scanned skill | Import a skill found in a known agent directory | unwired | real IPC `import_existing_skill` |
| Install | Import All | Import every discovered skill | unwired | real IPC `import_all_discovered` |
| Install | Scan list | Find `SKILL.md` files under known agent directories | demo-only fallback | real IPC `scan_local_skills` |
| Install | Leaderboard search, owner filter, Install, Cancel, Load more | Search and install from skills.sh | local-state-only fixture | real IPC `fetch_leaderboard`, `install_from_skillssh`, and `cancel_market_install`; cancel sets a flag and kills the spawned git process |
| Library | Add, remove, rename, and delete tags | Persist tags in `hive.db` | local-state-only | real IPC `set_skill_tags`, `rename_tag`, `delete_tag` |
| Library | Enable and disable | Persist `skills.enabled` | local-state-only | real IPC `set_skills_enabled` |
| Library | Delete and Delete selected | Remove the skill directory, sidecars, and the database row | local-state-only | real IPC `delete_managed_skills` |
| Library | Sync to agents and per-agent toggle | Deploy or remove the skill in the agent directory | local-state-only | real IPC `sync_skill_to_tool` and `unsync_skill_from_tool` |
| Library | Preset membership and reorder | Update scenario membership | local-state-only | real IPC `add_skill_to_preset`, `remove_skill_from_preset`, `reorder_preset_skills` |
| Library | Check all and Update | Record an update check | local dialog only | real IPC `check_all_skill_updates` plus the existing removal dialog |
| Library | Git status | Show backup repository status | unwired | real IPC `git_backup_status` |
| Sidebar | Preset reorder, rename, delete, New Preset | Persist scenarios and rebuild the tray preset menu | local-state-only | real IPC `reorder_presets`, `update_preset`, `delete_preset`, `create_preset`, and `rebuild_tray_menu` |
| Sidebar | Theme | Persist the theme setting | real IPC `set_settings` | real IPC `set_settings` |
| Sidebar | Agent list | Show installed, enabled agents | listed every adapter | filtered to installed and enabled |
| Workspace | Apply preset and New Preset | Apply or create a scenario | unwired | real IPC `apply_preset` and `create_preset` |
| Workspace | Upload, pull, remove, delete, add from library | Sync or delete the skill for that agent | local-state-only | real IPC `sync_skill_to_tool`, `unsync_skill_from_tool`, `delete_managed_skills` |
| Project | Link project | Create the project directory and a `projects` row | local-state-only | real IPC `add_project` |
| Project | Add to agents, disable, batch enable | Sync and enable the project skill | local-state-only | real IPC `sync_skill_to_tool` and `set_skills_enabled` |
| Project | Pull center | Write the center body into the project skill | local-state-only | real IPC `pull_project_center` writes `skills/projects/{name}/SKILL.md` |
| Backup | Back Up Now, Sync, Retry | Commit the skills directory and tag `snap-*` | local-state-only | real IPC `git_backup_sync` |
| Backup | Restore | `git reset` to the snapshot and keep a safety tag | local-state-only | real IPC `git_backup_restore_version` |
| Backup | GitHub connect | Store a remote URL | unwired | real IPC `git_backup_set_remote` |
| Backup | Disconnect | Remove `origin` | unwired | real IPC `git_backup_remove_remote` |
| Backup | Device name | Persist `backup_device_name` | local-state-only | real IPC `backup_set_device_name` |
| Backup | Revoke, set aside, PAT | Keep the PAT in the OS keychain and set local changes aside | unwired | real IPC `store_backup_pat`, `revoke_backup_token`, and `set_aside_backup` |
| Backup | Setup and recovery | Read git status | unwired | real IPC `git_backup_status` |
| Backup | libgit2 engine | Alternate backup engine | disabled | still disabled; PLAN marks it for a later release |
| Settings | Save path and reset repository path | Persist the central repository override | unwired | real IPC `set_central_repo_path` |
| Settings | Symlink and File Copy | Persist `sync_mode` | local-state-only | real IPC `set_settings` |
| Settings | Close behavior, tray, proxy | Persist those settings | local-state-only | real IPC `set_settings` |
| Settings | Enable all, disable all, reorder, path, project path, remove | Persist agent configuration | local-state-only | real IPC `set_all_tools_enabled`, `set_tool_order_cmd`, `set_custom_tool_path`, `set_custom_tool_project_path`, `remove_custom_tool` |
| Settings | Reset path | Restore the adapter's default skills directory | hardcoded `~/.claude/skills` for every agent | real IPC `reset_custom_tool_path` |
| Settings | New Agent | Insert a custom tool | local-state-only | real IPC `add_custom_tool` plus a folder picker |
| Settings | Check for Updates | Download and install through tauri-plugin-updater | local decision only | local decision plus `download_and_install_update`; `check_app_update` still returns notify-only |
| Settings | Export diagnostics | Write a sanitized zip to a chosen path | local-state-only | real IPC `export_diagnostics` |
| Skill editor | Save, snapshot, publish, fix, restore | Write, version, publish, and fix the draft | local-state-only | real IPC `save_skill_draft`, `snapshot_skill_draft`, `publish_skill_draft`, `fix_skill_draft`, `restore_skill_draft` |
| Agent editor | Save and export | Write and export `AGENT.md` | local-state-only | real IPC `save_agent_draft` and `export_agent_draft` |
| Endpoints | Secret | Store the provider secret in the OS keychain | local-state-only | real IPC `set_provider_secret` through the keyring crate |
| Generate | Save draft | Write generated `SKILL.md` | local-state-only | real IPC `save_generated_draft` |
| Suites | Apply optimized description | Rewrite the description and write `evals.json` | local-state-only | real IPC `apply_optimized_description` |
| Compare | Verdict | Store the verdict in `comparisons` | local-state-only | real IPC `record_compare_verdict` |
| History | Baseline | Write `history.json` with one current best | unwired | real IPC `write_history_baseline` |
| Backup | First-run restore | Clone a remote library into the skills directory | not rendered | real form plus `git_backup_restore_remote` |
| Tray | Presets submenu | Apply and remove presets from the tray | not rendered | tray menu built from scenarios; Apply and Remove call hive-core |
| Lobster | Scan | Rescan known agent directories | unwired | real IPC `scan_local_skills` |
| Dashboard | Stats and activity | Render the managed library | demo or empty | real IPC `get_managed_skills` |

## Not completed

- GitHub device login cannot finish in this build. Connect stores the remote URL and says that an OAuth client id is required. There is no client id in the repository.
- The libgit2 backup engine stays disabled. `PLAN.md` keeps the system-git engine as the one that ships.
- A live skills.sh page is still required for a real marketplace listing. The click suite sets `SKILL_HIVE_MARKET_FIXTURE` so the row is local, pauses the install, and asserts Cancel stops it before the skill row appears. A non-directory source still spawns `git clone` and Cancel kills that process.
- `check_app_update` still returns `notify-only`. Check for Updates also calls `download_and_install_update`. That install finishes only when the signed updater endpoint publishes a release. The command records `installed:`, `current`, `unavailable:`, or `download-failed:` in `updater_last_result`.
- The tray icon itself is not clicked from the webview. New Preset rebuilds the menu and writes `tray-menu.json`. Apply and Remove on that menu call `apply_preset` and unsync the scenario skills.
- `clear_endpoint_secret` and `skip_first_run_restore` are registered. The wizard submits a remote URL; it does not render a separate skip button.
- Demo mode (`localStorage` `skill-hive-demo` = `1`) still skips these commands so the browser visual suite stays local.

## Verification

- `npm test` covers the screens with a stateful IPC mock.
- `scripts/tauri-driver-suite.mjs` installs a local folder, restarts, imports a seeded `~/.claude/skills` skill, deletes, syncs to Codex, adds an agent, imports a local bare repo, and restores a backup. It asserts `hive.db` and the skills directory under `SKILL_HIVE_BASE`. Linux isolates `HOME`. Windows keeps the real user profile so the log plugin can resolve its directory, and seeds the agent folders there.
- `.github/workflows/tauri-driver.yml` runs that click suite on Ubuntu and Windows, including the authoring artifacts. tauri-driver 2.1.0 does not listen on macOS, so the macOS job runs `cargo test -p skill-hive --lib effects`. That covers the earlier wiring effects and `authoring_effects_save_publish_secrets_history_and_diagnostics`.
- Demo mode (`localStorage` `skill-hive-demo` = `1`) keeps the marketplace fixture and in-memory backup snapshots for the browser visual suite. A normal `tauri dev` run does not set that flag, so those controls call hive-core.
