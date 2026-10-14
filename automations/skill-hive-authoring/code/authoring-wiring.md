# Authoring controls

`record_effect` is gone. Each control below calls a dedicated Tauri command, and that command calls hive-core.

Secrets go through the `keyring` crate (`skill-hive-provider` for endpoint keys, `skill-hive-git-backup` for the backup PAT). On Linux without a Secret Service the same helpers write a labelled AES-256-GCM file. The secret string is not written in plaintext. The settings row stores the backend name (`keyring` or `labelledfallback`), not the secret.

| Control | Before | After |
| --- | --- | --- |
| Skill editor Save | `record_effect` | `save_skill_draft` writes the draft and `VersionStore::save_autosave` |
| Skill editor Snapshot | `record_effect` | `snapshot_skill_draft` calls `VersionStore::snapshot` |
| Skill editor Restore | `record_effect` | `restore_skill_draft` calls `VersionStore::restore` with the id the snapshot command returned |
| Skill editor Publish | `record_effect` | `publish_skill_draft` calls `publish_draft` |
| Skill editor Fix | `record_effect` | `fix_skill_draft` calls `apply_skill_fix` |
| Agent editor Save | `record_effect` | `save_agent_draft` writes `AGENT.md` |
| Agent editor Export | `record_effect` | `export_agent_draft` copies `export/AGENT.md` |
| Generate Save | `record_effect` | `save_generated_draft` writes `SKILL.md` |
| Suites Apply | `record_effect` | `apply_optimized_description` calls `apply_optimized` and `save_suite` |
| Compare Record verdict | `record_effect` | `record_compare_verdict` calls `store_verdict` |
| History Baseline | `record_effect` | `write_history_baseline` calls `write_history` |
| Endpoints Save key | `record_effect` | `set_provider_secret` through the keyring crate |
| Backup PAT | `record_effect` | `store_backup_pat` through the keyring crate |
| Backup Revoke | `record_effect` | `revoke_backup_token` deletes the keychain item |
| Backup Set aside | `record_effect` | `set_aside_backup` copies the skills tree and records `backup_aside` |
| Project Pull center | `record_effect` | `pull_project_center` writes `skills/projects/{name}/SKILL.md` |
| Settings Export Logs | `record_effect` | `export_diagnostics` calls `export_logs_zip` at the chosen path |
| Settings Check for Updates | `decideUpdate` plus notify-only `check_app_update` | `download_and_install_update` uses `tauri-plugin-updater` |
| Leaderboard Cancel | local button only | `begin_market_install` runs the clone on a background thread and returns immediately; `cancel_market_install` sets a flag and kills that process |
| First-run restore | not rendered | form submits `git_backup_restore_remote`, which clones into the skills directory |
| Tray Presets | not rendered | submenu from scenarios; Apply and Remove call hive-core; New Preset calls `rebuild_tray_menu` |

## Still not the full PLAN behavior

- GitHub device login still cannot finish. This build has no OAuth client id. Connect stores the remote URL.
- The libgit2 backup engine stays disabled. Backup uses system git, which is the engine PLAN ships.
- A live skills.sh leaderboard still needs the network. The click suite points `SKILL_HIVE_MARKET_FIXTURE` at a local skill and pauses the install so Cancel can be asserted without GitHub. The Install click returns as soon as the background thread starts, then Cancel lands during that pause. A non-directory source still spawns `git clone`, and Cancel kills that process.
- `check_app_update` still returns `notify-only`. Check for Updates also runs `download_and_install_update`. The install completes only when the signed endpoint has a release. The command records `installed:`, `current`, `unavailable:`, or `download-failed:` in `updater_last_result`.
- The OS tray icon is not clicked from the webview. The suite asserts `tray-menu.json` after New Preset. The menu items call `apply_preset_by_id` and `remove_preset_by_id`.
- `clear_endpoint_secret` and `skip_first_run_restore` are registered. The wizard submits a remote; it does not show a separate skip button.
- Demo mode (`localStorage` `skill-hive-demo` = `1`) still skips these commands so the browser visual suite stays local.
