# M1 parity ledger

Phase 1 must rows from PLAN.md Part 5, epics E1.1 through E1.9. A row is **done** only when the behaviour in that epic's acceptance line is covered by a test in this repository. **Partial** means a slice exists and the rest of the row does not.

Phase 2 rows (marketplace client, backup, tray, locales other than English, CLI command tree, project workspaces, lobster, update install) are out of scope and are not listed.

| Area | Must row | Status | Evidence |
|---|---|---|---|
| Library core | SKILL.md detection, frontmatter parsing, name sanitisation | done | `skill_metadata::tests::sanitises_names_and_parses_frontmatter`, `marker_search_stops_at_depth_4` |
| Library core | Content hashing with shared content scope and EOL-insensitive tiebreaker | done | `content_hash::tests::shared_scope_and_eol_tiebreaker` |
| Library core | Metadata sidecars, reindex and rebuild both ways, EmptySkillMetadata guard | done | `phase1_tests::sidecar_less_library_is_refused_and_rebuilt_from_the_database` |
| Library core | Import an existing Skills Manager library (counts, tags, presets, membership order) | done | `phase1_tests::skills_manager_import_reproduces_counts_and_order` |
| Agents | 54 built-in adapters with global and project directories | done | `phase1_tests::fifty_four_adapters_have_global_and_project_directories` |
| Agents | Detection of installed agents (`~/.claude`, `~/.codex`) | done | `phase1_tests::tool_status_reports_only_claude_and_codex_when_those_homes_exist` |
| Agents | Custom agents and path overrides | done | `phase1_tests::custom_agent_collision_is_skipped_and_order_persists`, `phase1_tests::path_overrides_round_trip_and_reset` |
| Agents | Settings Agents: enable toggle, path edit/reset, project path, drag-reorder, Enable All / Disable All, Refresh | partial | Cards and New Agent render in `src/test/phase1.test.tsx` ("adds a custom agent"). Enable All, Disable All, path reset, drag-reorder, and Refresh are not wired. Rust helpers exist on `tool_service`. |
| Agents | Custom agents: New Agent form and remove | partial | Form adds a local card in the vitest above. Remove is not in the screen. `tool_service::remove_custom_tool` is not called from the UI. |
| Deploy | Symlink, then Windows junction, then copy; UNC skips links | done | `sync_engine::tests::symlink_then_junction_then_copy`, `unc_fallback_copies_without_trying_a_link`, `repeated_symlink_deploy_is_a_noop_and_a_real_directory_is_refused`, `junction_path_is_exercised` (Windows) |
| Deploy | TARGET_CONFLICT names the path and leaves a foreign directory untouched | done | `sync_engine::tests::repeated_symlink_deploy_is_a_noop_and_a_real_directory_is_refused`, `phase1_tests::foreign_directory_conflict_names_the_path` |
| Deploy | Scenario service: exclusive apply, shared paths kept, membership does not deploy | done | `phase1_tests::preset_b_after_a_unsyncs_obsolete_targets_and_keeps_shared_ones`, `install_deploy_preset_undeploy_scan_and_import` |
| Deploy | Shared directory kept while another tool still claims it | done | `phase1_tests::shared_directory_undeploy_keeps_the_path_while_another_tool_claims_it` |
| Library screen | Grid/list, search, All/Enabled/Available, source pills, tag pills, Untagged | done | `src/test/phase1.test.tsx` ("filters, renames a tag…"), `src/lib/phase1/model.test.ts` |
| Library screen | Preset membership toggle and preset-scoped drag reorder | partial | Not implemented on the card. |
| Library screen | Card menu Check / Delete (Update arrives in Phase 2) | partial | Not implemented. |
| Library screen | Inline tag add/remove; tag rename from the filter pill | partial | Add and rename are in the library vitest. Tag delete is not. |
| Library screen | Multi-select Enable/Disable, Sync to agents, Tags, Delete | partial | Sync to agents and SyncDots are in the library vitest. The other batch actions are not. |
| Library screen | Detail panel Local / Diff / Source | done | Same library vitest opens Local body and the Diff tab. |
| Library screen | Detail panel agent toggles for the viewed preset | partial | Not implemented. |
| Library screen | Side-by-side document diff and per-file source diff | partial | Document LCS is `model.test.ts` and the Diff tab. Per-file source diff is not. |
| Install | Tabs Leaderboard / Local / Git and `?tab=` | partial | `InstallView` reads `tab` and shows the marketplace empty state. No vitest asserts the deep link. Playwright visits each tab. |
| Install | Local folder, archive, and batch import | partial | `install_local` is used by the integration test. `phase1_tests::archive_hash_matches_unpacked_directory`. The screen posts a path field and does not call the installer. |
| Install | Scan, rename, import one, import all | partial | `install_deploy_preset_undeploy_scan_and_import` covers scan and import all in Rust. The screen lists scanned rows and an Import All button with no Rust command behind it. |
| Install | Git preview, rename, import, cancel | partial | `phase1_tests::git_preview_rename_imports_the_selected_skill`, `cancelled_preview_removes_its_temp_dir`, `slash_branch_resolves_offline_and_online`. The screen's preview is local component state. |
| Install | Tree URL grammar | done | `slash_branch_resolves_offline_and_online` |
| Install | Sparse clone, 1 GiB cache, 300 s timeout | partial | Not implemented in `git_fetcher.rs`. |
| Presets | Sidebar switch, drag-reorder, edit, delete, New Preset | partial | Sidebar lists presets and New Preset. Drag, edit, and delete are not implemented. |
| Presets | Preset bar active / partial / inactive | done | `src/lib/phase1/model.test.ts` (`presetFill`) and `phase1.test.tsx` ("applies a preset from the global workspace") |
| Presets | Membership edits never deploy; apply is exclusive | done | Rust tests named in the Deploy rows |
| Workspace | Overview with agent cards and the all-agent preset bar | partial | Preset bar is tested. Agent cards on the overview are the two hardcoded workspace rows, not the adapter catalog. |
| Workspace | Agent page: on-disk list, status pills, search, tag filters, grid/list | partial | `/global-workspace/:agentKey` renders the same overview. Search, tag filters, and layout toggle are absent. |
| Workspace | Pull, Upload, Remove, Delete with confirmations | partial | Rust: `phase1_tests::workspace_upload_adopts_and_pull_returns_to_in_sync` (Upload requires `Adopt::UserConfirmed`; delete of a managed target is refused). Buttons on the card are not connected. |
| Workspace | Batch remove / delete | partial | Not implemented. |
| Workspace | Detail sheet Local / Diff / Center | partial | Not implemented. |
| Workspace | Add from Library sheet | partial | Not implemented. |
| Workspace | Sidebar All Agents and per-agent rows | partial | Global Workspace entry is in the sidebar. Per-agent rows are not. |
| Settings | Repository path change shows the restart notice; next launch moves the library and symlinks still resolve | done | Vitest "adds a custom agent and shows the repository restart notice". Rust `phase1_tests::next_launch_moves_the_library_and_symlinks_still_resolve` renames the tree and repoints links. It does not call `set_base_dir_override` (that writes the real user config file). |
| Settings | Default sync mode Symlink / File Copy | partial | Both labels render. The choice is not stored. |
| Settings | Theme, text size, English catalog, i18n lint | done | `src/test/shell.test.tsx` persists theme. `phase1.test.tsx` sets Extra Large. `npm run check:i18n` |
| Settings | About version and Check for Updates (notify only) | partial | Copy renders. The button does not notify. |
| Settings | Reduce motion | partial | Phase 0 token CSS still has the reduced-motion block. Settings has no control. |
| Shell | Dashboard greeting, summary, stat cards, quick actions, recent activity | done | `phase1.test.tsx` "shows an empty dashboard and the scan action" |
| Shell | Command palette over skills, presets, and actions; empty-state copy | done | `phase1.test.tsx` "opens a skill from the command palette"; shell test covers the empty copy. Projects are a Phase 2 surface and are not a palette group. |
| Shell | Mod+1 through Mod+5, Mod+F, Mod+/, Mod+K, Mod+B | partial | Handlers are in `AppShell`. Vitest covers Mod+K and Mod+B (shell). Mod+1–5, Mod+F, and the shortcut sheet have no test. Remapping is not stored. |
| Shell | Error banner with Retry; refresh on `app-files-changed`, `skills-auto-updated`, and `backup-auto-completed` | done | `phase1.test.tsx` banner Retry. `src/test/watcher.test.tsx` |
| Visual | Every Phase 1 screen in light and dark at 1100x640 | done | `e2e/phase1-visual.spec.ts` |
| Visual | DX review on WebKitGTK and WKWebView | partial | `docs/parity/dx-review-m1.md` records that the review was not run |

## Exit criteria

1. Ledger of every must row as done — **not met**. Partial rows remain in the table above.
2. Deploy by symlink (macOS/Linux) and symlink/junction/copy (Windows), with TARGET_CONFLICT on three OSes — **covered by Rust tests** in `cargo test --workspace` (Windows junction is `#[cfg(windows)]`).
3. Skills Manager import reproduces counts, tags, presets, and membership order on three OSes — **covered** by `skills_manager_import_reproduces_counts_and_order`.
4. 54 adapters, custom agents, and path overrides — **covered** by `fifty_four_adapters_have_global_and_project_directories`, `custom_agent_collision_is_skipped_and_order_persists`, and `path_overrides_round_trip_and_reset`. The Settings path editor is still a partial screen row under criterion 1.
5. Install local, deploy, preset apply, undeploy, scan, and import — **covered** by `install_deploy_preset_undeploy_scan_and_import`.
6. Playwright visual checks and a DX review on WebKitGTK and WKWebView — **Playwright is in CI**. The DX review was not performed (`docs/parity/dx-review-m1.md`).
