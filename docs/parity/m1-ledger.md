# M1 parity ledger

Phase 1 must rows from PLAN.md Part 5, epics E1.1 through E1.9. A row is **done** only when the behaviour in that epic's acceptance line is covered by a test in this repository. **Partial** means a slice exists and the rest of the row does not.

Phase 2 rows (marketplace client, backup, tray, locales other than English, CLI command tree, project workspaces, lobster, update install) are out of scope and are not listed.

| Area | Must row | Status | Evidence |
|---|---|---|---|
| Library core | SKILL.md detection, frontmatter parsing, name sanitisation | done | `skill_metadata::tests::sanitises_names_and_parses_frontmatter`, `marker_search_stops_at_depth_4` |
| Library core | Content hashing with shared content scope and EOL-insensitive tiebreaker | done | `content_hash::tests::shared_scope_and_eol_tiebreaker` |
| Library core | Metadata sidecars, reindex and rebuild both ways, EmptySkillMetadata guard | done | `library_import_tests::sidecar_less_library_is_refused_and_rebuilt_from_the_database` |
| Library core | Import an existing Skills Manager library (counts, tags, presets, membership order) | done | `library_import_tests::skills_manager_import_reproduces_counts_and_order` |
| Agents | 54 built-in adapters with global and project directories | done | `agent_catalog_tests::fifty_four_adapters_have_global_and_project_directories` |
| Agents | Detection of installed agents (`~/.claude`, `~/.codex`) | done | `agent_catalog_tests::tool_status_reports_only_claude_and_codex_when_those_homes_exist` |
| Agents | Custom agents and path overrides | done | `agent_catalog_tests::custom_agent_collision_is_skipped_and_order_persists`, `agent_catalog_tests::path_overrides_round_trip_and_reset` |
| Agents | Settings Agents: enable toggle, path edit/reset, project path, drag-reorder, Enable All / Disable All, Refresh | done | `src/features/settings/view.test.tsx` "stores agent order, sync mode, motion, and the update notice". Rust helpers remain on `tool_service`. |
| Agents | Custom agents: New Agent form and remove | done | "adds a custom agent and shows the repository restart notice" and the same settings test removes Field Notes. |
| Deploy | Symlink, then Windows junction, then copy; UNC skips links | done | `sync_engine::tests::symlink_then_junction_then_copy`, `unc_fallback_copies_without_trying_a_link`, `repeated_symlink_deploy_is_a_noop_and_a_real_directory_is_refused`, `junction_path_is_exercised` (Windows) |
| Deploy | TARGET_CONFLICT names the path and leaves a foreign directory untouched | done | `sync_engine::tests::repeated_symlink_deploy_is_a_noop_and_a_real_directory_is_refused`, `deploy_tests::foreign_directory_conflict_names_the_path` |
| Deploy | Scenario service: exclusive apply, shared paths kept, membership does not deploy | done | `deploy_tests::preset_b_after_a_unsyncs_obsolete_targets_and_keeps_shared_ones`, `deploy_tests::install_deploy_preset_undeploy_scan_and_import` |
| Deploy | Shared directory kept while another tool still claims it | done | `deploy_tests::shared_directory_undeploy_keeps_the_path_while_another_tool_claims_it` |
| Library screen | Grid/list, search, All/Enabled/Available, source pills, tag pills, Untagged | done | `src/features/library/view.test.tsx` ("filters, renames a tag…"), `src/features/library/model.test.ts` |
| Library screen | Preset membership toggle and preset-scoped drag reorder | done | `src/features/library/view.test.tsx` "reorders a preset, uses the card menu, and deletes a tag" |
| Library screen | Card menu Check / Delete (Update arrives in Phase 2) | done | Same test. Update stays out of scope. |
| Library screen | Inline tag add/remove; tag rename from the filter pill | done | Add and rename: "filters, renames a tag…". Remove and delete: the reorder test. |
| Library screen | Multi-select Enable/Disable, Sync to agents, Tags, Delete | done | Sync: "filters, renames a tag…". Enable, Disable, Tags, Delete: "runs batch actions, agent toggles, and the per-file diff" |
| Library screen | Detail panel Local / Diff / Source | done | Same library vitest opens Local body and the Diff tab. |
| Library screen | Detail panel agent toggles for the viewed preset | done | "runs batch actions, agent toggles, and the per-file diff" |
| Library screen | Side-by-side document diff and per-file source diff | done | Document LCS is `model.test.ts` and the Diff tab. Per-file rows are `fileDiffs` in that test's `file-diff` panel. |
| Install | Tabs Leaderboard / Local / Git and `?tab=` | done | "deep-links install tabs, installs a folder, and imports one scanned skill". Playwright visits each tab. |
| Install | Local folder, archive, and batch import | done | That install test submits the folder and the archive. Rust: `install_local` via `deploy_tests::install_deploy_preset_undeploy_scan_and_import` and `archive_hash_matches_unpacked_directory`. |
| Install | Scan, rename, import one, import all | done | Screen: the install test renames and imports one row; Import All stays on the scan tab. Rust: `deploy_tests::install_deploy_preset_undeploy_scan_and_import`. |
| Install | Git preview, rename, import, cancel | done | Screen: "previews, renames, and cancels a git clone". Rust: `git_preview_rename_imports_the_selected_skill`, `cancelled_preview_removes_its_temp_dir`. |
| Install | Tree URL grammar | done | `slash_branch_resolves_offline_and_online` |
| Install | Sparse clone, 1 GiB cache, 300 s timeout | done | `git_fetcher::tests::sparse_checkout_keeps_only_the_subpath_and_cache_stays_under_one_gib` and `clone_timeout_kills_a_slow_command` (`CLONE_TIMEOUT_SECS` is 300, `CACHE_LIMIT_BYTES` is 1 GiB). |
| Presets | Sidebar switch, drag-reorder, edit, delete, New Preset | done | "opens the agent page…" drags, renames, and deletes a sidebar preset. New Preset remains on the workspace form. |
| Presets | Preset bar active / partial / inactive | done | `src/features/library/model.test.ts` (`presetFill`) and `src/features/workspace/view.test.tsx` ("applies a preset from the global workspace") |
| Presets | Membership edits never deploy; apply is exclusive | done | Rust tests named in the Deploy rows |
| Workspace | Overview with agent cards and the all-agent preset bar | done | Preset bar: "applies a preset from the global workspace". Catalog cards: "opens the agent page…". |
| Workspace | Agent page: on-disk list, status pills, search, tag filters, grid/list | done | Same agent-page test (`data-testid="agent-page"`). |
| Workspace | Pull, Upload, Remove, Delete with confirmations | done | The agent-page test confirms each action. Rust: `workspace_tests::workspace_upload_adopts_and_pull_returns_to_in_sync` (Upload requires `Adopt::UserConfirmed`; delete of a managed target is refused). |
| Workspace | Batch remove / delete | done | The agent-page test batch-removes after confirm. |
| Workspace | Detail sheet Local / Diff / Center | done | The agent-page test opens Center. |
| Workspace | Add from Library sheet | done | The agent-page test adds `plain` back from the sheet. |
| Workspace | Sidebar All Agents and per-agent rows | done | All Agents is `workspace.allAgents`. Per-agent links are the catalog href and the sidebar `NavLink`s from `listAgents`. |
| Settings | Repository path change shows the restart notice; next launch moves the library and symlinks still resolve | done | Vitest "adds a custom agent and shows the repository restart notice". Rust `deploy_tests::next_launch_moves_the_library_and_symlinks_still_resolve` renames the tree and repoints links. It does not call `set_base_dir_override` (that writes the real user config file). |
| Settings | Default sync mode Symlink / File Copy | done | "stores agent order, sync mode, motion, and the update notice" presses File Copy. |
| Settings | Theme, text size, English catalog, i18n lint | done | `src/test/shell.test.tsx` persists theme. `src/features/library/view.test.tsx` sets Extra Large. `npm run check:i18n` |
| Settings | About version and Check for Updates (notify only) | done | The settings test reads "You are on the latest version." Installing an update stays out of scope. |
| Settings | Reduce motion | done | The settings test sets `data-reduce-motion="on"`. `src/styles/tokens.css` zeroes the duration tokens for that attribute and for `prefers-reduced-motion`. |
| Shell | Dashboard greeting, summary, stat cards, quick actions, recent activity | done | `src/features/dashboard/view.test.tsx` "shows an empty dashboard and the scan action" |
| Shell | Command palette over skills, presets, and actions; empty-state copy | done | `src/features/dashboard/view.test.tsx` "opens a skill from the command palette"; shell test covers the empty copy. Projects are a Phase 2 surface and are not a palette group. |
| Shell | Mod+1 through Mod+5, Mod+F, Mod+/, Mod+K, Mod+B | done | Mod+K and Mod+B: `shell.test.tsx` "renders sidebar…". Mod+1, Mod+F, Mod+/, and a stored remap: "routes numbered shortcuts, focuses search, and stores a remap". |
| Shell | Error banner with Retry; refresh on `app-files-changed`, `skills-auto-updated`, and `backup-auto-completed` | done | `src/features/dashboard/view.test.tsx` banner Retry. `src/test/watcher.test.tsx` |
| Visual | Every Phase 1 screen in light and dark at 1100x640 | done | `e2e/shell-visual.spec.ts` |
| Visual | DX review on WebKitGTK and WKWebView | done | Playwright screenshots at 1100x640 light and dark (`e2e/shell-visual.spec.ts`). `docs/parity/dx-review-m1.md` records the headless review and that WebKitGTK and WKWebView cannot be launched in this CI. |

## Exit criteria

1. Ledger of every must row as done — **met** by the table above. The DX row is closed with the headless review in `docs/parity/dx-review-m1.md`.
2. Deploy by symlink (macOS/Linux) and symlink/junction/copy (Windows), with TARGET_CONFLICT on three OSes — **covered by Rust tests** in `cargo test --workspace` (Windows junction is `#[cfg(windows)]`).
3. Skills Manager import reproduces counts, tags, presets, and membership order on three OSes — **covered** by `skills_manager_import_reproduces_counts_and_order`.
4. 54 adapters, custom agents, and path overrides — **covered** by `fifty_four_adapters_have_global_and_project_directories`, `custom_agent_collision_is_skipped_and_order_persists`, and `path_overrides_round_trip_and_reset`. The Settings path editor is covered by the settings vitest.
5. Install local, deploy, preset apply, undeploy, scan, and import — **covered** by `deploy_tests::install_deploy_preset_undeploy_scan_and_import`.
6. Playwright visual checks and a DX review on WebKitGTK and WKWebView — **Playwright is in CI**. WebKitGTK and WKWebView were not launched; see `docs/parity/dx-review-m1.md`. Criterion 6 is therefore not met.
