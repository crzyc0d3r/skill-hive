# Graph Report - workspace  (2026-10-14)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 2709 nodes · 5524 edges · 172 communities (118 shown, 54 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 256 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `c67fca98`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Community 0
- Community 1
- Community 2
- Community 3
- Community 4
- Community 5
- Community 6
- Community 7
- Community 8
- Community 9
- Community 10
- Community 11
- Community 12
- Community 13
- Community 14
- Community 15
- Community 16
- Community 17
- Community 18
- Community 19
- Community 20
- Community 21
- Community 22
- Community 23
- Community 24
- Community 25
- Community 26
- Community 27
- Community 28
- Community 29
- Community 30
- Community 31
- Community 32
- Community 33
- Community 34
- Community 35
- Community 36
- Community 37
- Community 38
- Community 39
- Community 40
- Community 41
- Community 42
- Community 43
- Community 44
- Community 45
- Community 46
- Community 47
- Community 48
- Community 49
- Community 50
- Community 51
- Community 52
- Community 53
- Community 54
- Community 55
- Community 56
- Community 57
- Community 58
- Community 59
- Community 60
- Community 61
- Community 62
- Community 63
- Community 64
- Community 65
- Community 66
- Community 67
- Community 68
- Community 69
- Community 70
- Community 71
- Community 72
- Community 75
- Community 76
- Community 77
- Community 78
- Community 79
- Community 80
- Community 81
- Community 82
- Community 83
- Community 84
- Community 85
- Community 86
- Community 87
- Community 88
- Community 89
- Community 90
- Community 91
- Community 92
- Community 93
- Community 94
- Community 95
- Community 96
- Community 97
- Community 98
- Community 99
- Community 100
- Community 101
- Community 102
- Community 103
- Community 104
- Community 105
- Community 106
- Community 108
- Community 110
- Community 111
- Community 112
- Community 114
- Community 115
- Community 116
- Community 117
- Community 118
- Community 119
- Community 120
- Community 121
- Community 122
- Community 123
- Community 124
- Community 125
- Community 126
- Community 127
- Community 128
- Community 129
- Community 130
- Community 131
- Community 132
- Community 133
- Community 134
- Community 135
- Community 136
- Community 137
- Community 138
- Community 139
- Community 140
- Community 141
- Community 142
- Community 145

## God Nodes (most connected - your core abstractions)
1. `SkillStore` - 70 edges
2. `SkillStore` - 68 edges
3. `react` - 44 edges
4. `cn` - 36 edges
5. `radix-ui` - 28 edges
6. `t()` - 27 edges
7. `main()` - 24 edges
8. `Document` - 24 edges
9. `SkillsCmd` - 22 edges
10. `deploy_skill()` - 22 edges

## Surprising Connections (you probably didn't know these)
- `install()` --calls--> `left_click_restores()`  [INFERRED]
  src-tauri/src/tray.rs → crates/hive-core/src/util/tray_menu.rs
- `membership()` --references--> `ScenarioSkillMetaFile`  [EXTRACTED]
  automations/skill-hive-p0/sources/reference/skill_store.rs → crates/hive-core/src/library/sync_metadata.rs
- `cli_command()` --references--> `Command`  [EXTRACTED]
  automations/skill-hive-p0/sources/reference/cli_bridge.rs → crates/hive-cli/src/lib.rs
- `LIBRARY_LEASE` --references--> `File`  [EXTRACTED]
  automations/skill-hive-p0/sources/reference/central_repo.rs → crates/hive-core/src/deploy/sync_engine.rs
- `install()` --calls--> `build_tray_menu_from_data()`  [INFERRED]
  src-tauri/src/tray.rs → crates/hive-core/src/util/tray_menu.rs

## Import Cycles
- None detected.

## Communities (172 total, 54 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.07
Nodes (40): corrupted_key_file_returns_error(), decrypt(), different_nonces_each_time(), ENC_PREFIX, encrypt(), key_file_has_owner_only_permissions(), load_or_create_key(), load_or_create_key_creates_and_reloads() (+32 more)

### Community 1 - "Community 1"
Cohesion: 0.09
Nodes (51): profile_name(), agent_description_budget(), validate_agent_file(), estimate_tokens(), command_rules_and_agent_rows(), agent_budget_finding(), body_rules(), claude_enums() (+43 more)

### Community 2 - "Community 2"
Cohesion: 0.05
Nodes (32): canonical_root(), compare_bodies(), dedupe_agents_sharing_root(), dedupes_agents_sharing_a_root(), disable_by_moving(), disable_moves_directory_to_disabled_sibling(), disabled_sibling(), dot_color() (+24 more)

### Community 3 - "Community 3"
Cohesion: 0.07
Nodes (34): home_replacement(), other_rules(), redacts_email_and_token(), redacts_git_credentials(), replaces_generic_users_path(), sanitize(), should_log_first_or_slow(), AutoBackupPolicy (+26 more)

### Community 4 - "Community 4"
Cohesion: 0.15
Nodes (37): Bundle, cmd_accept(), cmd_approve(), cmd_baseline(), cmd_brief(), cmd_complete(), cmd_decide(), cmd_dispatch() (+29 more)

### Community 5 - "Community 5"
Cohesion: 0.04
Nodes (5): DiscoveredSkillRecord, PendingConflictRow, ProjectRecord, SkillStore, SkillTargetRecord

### Community 6 - "Community 6"
Cohesion: 0.08
Nodes (34): classify_target(), conflict(), deploy_skill(), DeployOutcome, fallback_chain(), is_current(), is_junction(), is_unc_path() (+26 more)

### Community 7 - "Community 7"
Cohesion: 0.07
Nodes (38): Document, move_skill_tool(), block_style(), Document, editing_description_touches_only_that_key(), Entry, is_block_indicator(), parse_document() (+30 more)

### Community 8 - "Community 8"
Cohesion: 0.13
Nodes (26): versions_dedup_pin_restore_and_history_schema(), AUTOSAVE_KEEP, copy_subject(), DraftVersion, export_history_json(), FileHash, hash_files(), hash_manifest() (+18 more)

### Community 9 - "Community 9"
Cohesion: 0.07
Nodes (9): body_gauge_references_and_silent_skip_reasons(), every_rule_id_has_a_function_path_and_the_suite_agrees_with_quick_validate(), python_bin(), quick_validate(), Qv, rename_refuses_unsafe_names_and_updates_evals(), skill(), write_skill() (+1 more)

### Community 10 - "Community 10"
Cohesion: 0.09
Nodes (36): apply_skill_fix(), BuiltInTemplate, Blank, DocumentProducer, MultiVariant, ReferenceHeavy, ScriptBacked, Workflow (+28 more)

### Community 11 - "Community 11"
Cohesion: 0.08
Nodes (21): collect(), hash_skill_dir(), normalise_newlines(), import_all(), import_scanned(), scan_roots(), ScannedSkill, Adopt (+13 more)

### Community 12 - "Community 12"
Cohesion: 0.09
Nodes (25): is_safe_zip_name(), safe_extract(), cache_expires(), cancelled_install_writes_nothing(), http_get(), install_zip_bytes(), leaderboard_falls_back_when_next_data_is_absent(), MarketSkill (+17 more)

### Community 13 - "Community 13"
Cohesion: 0.06
Nodes (25): AppError, classify_git_error_case_insensitive(), classify_git_error_detects_canceled_american_spelling(), classify_git_error_detects_cancelled(), classify_git_error_detects_connection_refused(), classify_git_error_detects_could_not_resolve_host(), classify_git_error_regular_git_error(), display_shows_message() (+17 more)

### Community 14 - "Community 14"
Cohesion: 0.05
Nodes (3): open_store(), sidecar_less_library_is_refused_and_rebuilt_from_the_database(), skills_manager_import_reproduces_counts_and_order()

### Community 15 - "Community 15"
Cohesion: 0.07
Nodes (22): AppError, classify_git_error_case_insensitive(), classify_git_error_detects_canceled_american_spelling(), classify_git_error_detects_cancelled(), classify_git_error_detects_connection_refused(), classify_git_error_detects_could_not_resolve_host(), classify_git_error_regular_git_error(), display_shows_message() (+14 more)

### Community 16 - "Community 16"
Cohesion: 0.08
Nodes (19): Input(), Separator(), SidebarContext, SidebarContextProps, SidebarInput(), SidebarMenuButton(), sidebarMenuButtonVariants, SidebarMenuSkeleton() (+11 more)

### Community 18 - "Community 18"
Cohesion: 0.13
Nodes (23): all_tool_adapters(), antigravity_uses_current_default_skills_path(), apply_builtin_path_overrides(), claude_code_does_not_scan_plugin_marketplaces_by_default(), custom_omp_agent_collision_keeps_builtin_adapter(), custom_tool_adapter(), custom_tool_paths(), custom_tool_project_paths() (+15 more)

### Community 19 - "Community 19"
Cohesion: 0.13
Nodes (29): addSkillToAgents(), applyRemoval(), backupNow(), BackupSnapshot, CloseChoice, DotColor, effectiveClose(), filterMarket() (+21 more)

### Community 20 - "Community 20"
Cohesion: 0.11
Nodes (21): APP_FS_CHANGED_EVENT, collect_watch_paths(), dir_has_entries(), EPOCH, insert_non_linked_project(), is_in_git_dir(), linked_workspace_watch_paths_only_include_selected_roots(), MUTE_STATE (+13 more)

### Community 21 - "Community 21"
Cohesion: 0.10
Nodes (23): react-dom, react-router-dom, @tanstack/react-query, @tauri-apps/api, @testing-library/jest-dom, @testing-library/react, @testing-library/user-event, vitest (+15 more)

### Community 22 - "Community 22"
Cohesion: 0.10
Nodes (24): clear_regenerable_target(), CONFIG_FILE_NAME, copy_dir_recursive(), copy_dir_recursive_copies_read_only_source_files(), copy_dir_recursive_keeps_links_as_links(), copy_symlink(), default_base_dir(), external_base_dir() (+16 more)

### Community 23 - "Community 23"
Cohesion: 0.12
Nodes (27): cache_evictions(), CACHE_LIMIT_BYTES, CacheSlot, cancel_git_preview(), clone_timeout_kills_a_slow_command(), CLONE_TIMEOUT_SECS, ClonePlan, confirm_git_install() (+19 more)

### Community 24 - "Community 24"
Cohesion: 0.11
Nodes (18): archive_and_directory_hash_equal(), batch_import_folder(), copy_dir(), install_archive(), install_local(), install_tree(), InstalledSkill, MAX_ZIP_ENTRIES (+10 more)

### Community 25 - "Community 25"
Cohesion: 0.10
Nodes (24): clear_regenerable_target(), CONFIG_FILE_NAME, copy_dir_recursive(), copy_dir_recursive_copies_read_only_source_files(), copy_dir_recursive_keeps_links_as_links(), copy_symlink(), default_base_dir(), external_base_dir() (+16 more)

### Community 26 - "Community 26"
Cohesion: 0.08
Nodes (6): cn, react, Alert(), alertVariants, ScrollArea(), ScrollBar()

### Community 27 - "Community 27"
Cohesion: 0.12
Nodes (21): add_custom_tool(), BUILTIN_ADAPTER_COUNT, CustomAdd, Added, SkippedCollision, disable_all(), disabled_keys(), enable_all() (+13 more)

### Community 28 - "Community 28"
Cohesion: 0.12
Nodes (16): APP_FS_CHANGED_EVENT, collect_watch_paths(), dir_has_entries(), EPOCH, insert_non_linked_project(), linked_workspace_watch_paths_only_include_selected_roots(), MUTE_STATE, MuteState (+8 more)

### Community 29 - "Community 29"
Cohesion: 0.08
Nodes (25): license, name, private, type, version, eslint, @eslint/js, eslint-plugin-react-hooks (+17 more)

### Community 30 - "Community 30"
Cohesion: 0.07
Nodes (27): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+19 more)

### Community 31 - "Community 31"
Cohesion: 0.16
Nodes (21): Kbd(), ACTIONS, AppShell(), ShellOutletContext, SHORTCUTS, StatusBanner(), isTextSize(), t() (+13 more)

### Community 32 - "Community 32"
Cohesion: 0.15
Nodes (12): blocking_acquire_waits_then_times_out_and_recovers(), FOREGROUND_WAIT, lock_file_lives_outside_skills_dir(), LOCK_FILE_NAME, open_lock_file(), POLL_INTERVAL, RepoLock, stamp_is_readable_by_another_handle_while_held() (+4 more)

### Community 33 - "Community 33"
Cohesion: 0.16
Nodes (18): add_membership(), apply_preset(), claimed_paths(), DeployIntent, Add, Remove, desired_targets(), DesiredTarget (+10 more)

### Community 34 - "Community 34"
Cohesion: 0.21
Nodes (21): add_column_if_missing(), count_setting(), has_column(), LATEST_VERSION, migrate_step(), migrate_v0_to_v1(), migrate_v1_to_v2(), migrate_v2_to_v3() (+13 more)

### Community 35 - "Community 35"
Cohesion: 0.12
Nodes (8): execute(), execute_isolated(), execute_locked(), finish(), lock(), now_ms(), Output, usage_error_is_exit_2_with_invalid_argument()

### Community 36 - "Community 36"
Cohesion: 0.14
Nodes (20): status_name(), apply_replacement(), approval_token(), AutoUpdateSchedule, check_replacement(), files_differ(), ignored_removal(), line_ending_only_difference_reports_current() (+12 more)

### Community 37 - "Community 37"
Cohesion: 0.21
Nodes (22): add_column_if_missing(), count_setting(), has_column(), LATEST_VERSION, migrate_step(), migrate_v0_to_v1(), migrate_v1_to_v2(), migrate_v2_to_v3() (+14 more)

### Community 38 - "Community 38"
Cohesion: 0.14
Nodes (9): AgentRow, delete_tag_removes_from_all_skills(), log_audit_appends_and_lists_newest_first(), log_audit_respects_limit(), rename_tag_to_itself_is_noop_not_delete(), rename_tag_updates_all_and_merges_into_existing(), ScenarioSkillToolToggleRecord, SENSITIVE_KEYS (+1 more)

### Community 40 - "Community 40"
Cohesion: 0.25
Nodes (20): agents(), Cli, compare_skill(), Ctx, dispatch(), Fail, App, Other (+12 more)

### Community 41 - "Community 41"
Cohesion: 0.09
Nodes (23): SkillsCmd, Adopt, Check, Deploy, Disable, Enable, Export, Install (+15 more)

### Community 42 - "Community 42"
Cohesion: 0.09
Nodes (23): devDependencies, eslint, @eslint/js, eslint-plugin-react-hooks, eslint-plugin-react-refresh, globals, jsdom, @playwright/test (+15 more)

### Community 43 - "Community 43"
Cohesion: 0.19
Nodes (18): emit_module(), port_central_repo(), port_cli_bridge(), port_error(), port_file_watcher(), port_log_sanitize(), port_path_guard(), port_repo_lock() (+10 more)

### Community 44 - "Community 44"
Cohesion: 0.11
Nodes (7): FALLBACK_NAME, is_path_safe(), MAX_NAME_LEN, normalize(), resolve_non_existing(), sanitize_name(), sanitize_truncates_long_names()

### Community 45 - "Community 45"
Cohesion: 0.13
Nodes (10): class-variance-authority, radix-ui, Badge(), badgeVariants, TabsList(), tabsListVariants, ToggleGroupContext, ToggleGroupItem() (+2 more)

### Community 46 - "Community 46"
Cohesion: 0.21
Nodes (17): DiffRow, fileDiffs(), filterSkills(), lcsDiff(), LibraryFilter, moveById(), moveItem(), PresetFill (+9 more)

### Community 47 - "Community 47"
Cohesion: 0.09
Nodes (21): compilerOptions, allowImportingTsExtensions, baseUrl, isolatedModules, jsx, lib, module, moduleDetection (+13 more)

### Community 48 - "Community 48"
Cohesion: 0.10
Nodes (6): AuditDraft, AuditEntry, MAX_ENTRIES, AuditDraft, AuditEntry, MAX_ENTRIES

### Community 49 - "Community 49"
Cohesion: 0.20
Nodes (14): begin_device_flow(), device_flow_and_pat_against_a_fixture(), DeviceCodes, DevicePoll, Approved, Denied, Pending, find_or_create_private_repo() (+6 more)

### Community 50 - "Community 50"
Cohesion: 0.24
Nodes (19): all_tool_adapters(), antigravity_uses_current_default_skills_path(), apply_builtin_path_overrides(), claude_code_does_not_scan_plugin_marketplaces_by_default(), custom_omp_agent_collision_keeps_builtin_adapter(), custom_tool_adapter(), custom_tool_paths(), custom_tool_project_paths() (+11 more)

### Community 51 - "Community 51"
Cohesion: 0.16
Nodes (10): cmdk, Command(), CommandDialog(), Dialog(), DialogContent(), DialogDescription(), DialogHeader(), DialogOverlay() (+2 more)

### Community 53 - "Community 53"
Cohesion: 0.22
Nodes (20): base_dir(), base_dir_override_active(), cache_dir(), clear_repoint_from(), configured_base_dir(), db_path(), ensure_central_repo(), live_base_from() (+12 more)

### Community 54 - "Community 54"
Cohesion: 0.10
Nodes (20): dependencies, class-variance-authority, cmdk, cn, @fontsource/ibm-plex-sans, @fontsource/jetbrains-mono, @hookform/resolvers, lucide-react (+12 more)

### Community 55 - "Community 55"
Cohesion: 0.26
Nodes (17): a_bridge_that_cannot_be_invalidated_blocks_the_publish(), a_failed_republish_leaves_the_previous_bridge_unusable(), BRIDGE_BIN_NAME, bridge_dir(), bridge_path(), bundled_cli(), cli_command(), ensure_bridge() (+9 more)

### Community 56 - "Community 56"
Cohesion: 0.16
Nodes (10): cmd_validate(), console(), graphify_cmd(), in_scope(), one_line(), render_brief(), render_tables(), topo_levels() (+2 more)

### Community 57 - "Community 57"
Cohesion: 0.23
Nodes (19): base_dir(), base_dir_override_active(), cache_dir(), clear_repoint_from(), configured_base_dir(), db_path(), ensure_central_repo(), live_base_from() (+11 more)

### Community 58 - "Community 58"
Cohesion: 0.22
Nodes (15): copy_path(), has_real_directories(), import_skills_manager_library(), ImportReport, json_dir_has_files(), list_json(), read_memberships(), read_scenarios() (+7 more)

### Community 59 - "Community 59"
Cohesion: 0.26
Nodes (17): a_bridge_that_cannot_be_invalidated_blocks_the_publish(), a_failed_republish_leaves_the_previous_bridge_unusable(), BRIDGE_BIN_NAME, bridge_dir(), bridge_path(), bundled_cli(), cli_command(), ensure_bridge() (+9 more)

### Community 60 - "Community 60"
Cohesion: 0.19
Nodes (16): DashboardPage(), InstallPage(), LibraryPage(), LobsterPage(), APP_ROUTES, AppRoute, NAV_ITEMS, screenFor() (+8 more)

### Community 61 - "Community 61"
Cohesion: 0.31
Nodes (12): cname(), communities(), cross_edges(), focus(), god_nodes(), is_internal(), load(), load_labels() (+4 more)

### Community 62 - "Community 62"
Cohesion: 0.11
Nodes (17): aliases, components, hooks, lib, ui, utils, iconLibrary, rsc (+9 more)

### Community 63 - "Community 63"
Cohesion: 0.22
Nodes (11): AgentExport, AgentExportRequest, apply_agent_fix(), export_agent(), render_agent_file(), render_agents_cli_json(), render_key(), scalar_json() (+3 more)

### Community 64 - "Community 64"
Cohesion: 0.15
Nodes (11): applyTextSize(), Locale, LOCALES, setLocale(), TABLES, TEXT_SIZES, TextSize, proxyOk() (+3 more)

### Community 65 - "Community 65"
Cohesion: 0.18
Nodes (17): a_bin_dir_outside_the_default_home_is_not_debris(), a_move_that_finished_unrecorded_still_repoints(), app_file_names_count_as_debris_only_at_the_target_root(), config_migrating(), directory_has_entries(), live_base_is_the_move_source_until_the_move_happens(), migrate_repo_if_needed(), migration_clears_regenerable_leftovers_and_records_repoint() (+9 more)

### Community 66 - "Community 66"
Cohesion: 0.18
Nodes (17): a_bin_dir_outside_the_default_home_is_not_debris(), a_move_that_finished_unrecorded_still_repoints(), app_file_names_count_as_debris_only_at_the_target_root(), config_migrating(), directory_has_entries(), live_base_is_the_move_source_until_the_move_happens(), migrate_repo_if_needed(), migration_clears_regenerable_leftovers_and_records_repoint() (+9 more)

### Community 67 - "Community 67"
Cohesion: 0.12
Nodes (17): scripts, build, check:headers, check:i18n, check:palette, cli, cli:build, cli:install (+9 more)

### Community 68 - "Community 68"
Cohesion: 0.16
Nodes (8): AlertDialogAction(), AlertDialogCancel(), AlertDialogContent(), AlertDialogOverlay(), AlertDialogPortal(), Button(), buttonVariants, DialogFooter()

### Community 69 - "Community 69"
Cohesion: 0.12
Nodes (16): compilerOptions, allowImportingTsExtensions, isolatedModules, lib, module, moduleDetection, moduleResolution, noEmit (+8 more)

### Community 70 - "Community 70"
Cohesion: 0.17
Nodes (4): install_panic_hook(), last_panic_path(), LOG_DIR, LOG_DIR

### Community 71 - "Community 71"
Cohesion: 0.17
Nodes (4): log_audit_appends_and_lists_newest_first(), log_audit_respects_limit(), ScenarioSkillToolToggleRecord, SENSITIVE_KEYS

### Community 72 - "Community 72"
Cohesion: 0.23
Nodes (9): check(), command_ids(), SCHEMAS, validate(), contract_suite_exercises_every_subcommand(), data_of(), deploy_into_an_occupied_target_returns_target_conflict(), record() (+1 more)

### Community 75 - "Community 75"
Cohesion: 0.14
Nodes (5): map_skill_row(), sample_skill(), ScenarioRecord, SkillRecord, skips_memberships_referencing_missing_skill_or_scenario()

### Community 76 - "Community 76"
Cohesion: 0.24
Nodes (10): append_under(), fail(), main(), ok(), packaged_skill_mds(), walk(), parse_frontmatter(), parse_scalar() (+2 more)

### Community 77 - "Community 77"
Cohesion: 0.13
Nodes (15): PresetsCmd, AddSkill, Apply, Create, Current, Deactivate, Delete, Deploy (+7 more)

### Community 78 - "Community 78"
Cohesion: 0.20
Nodes (8): an_environment_argument_string_keeps_every_flag(), an_environment_port_is_forwarded_without_driver_arguments(), box_err(), driver_log(), driver_switches_become_webview_arguments(), run(), webview_driver_overrides(), WebviewDriverOverrides

### Community 79 - "Community 79"
Cohesion: 0.19
Nodes (11): react-hook-form, FormControl(), FormDescription(), FormFieldContext, FormFieldContextValue, FormItemContext, FormItemContextValue, FormLabel() (+3 more)

### Community 80 - "Community 80"
Cohesion: 0.27
Nodes (12): call(), listAgents(), listPresets(), listScanned(), listSkills(), readSetting(), ScanRow, writeSetting() (+4 more)

### Community 82 - "Community 82"
Cohesion: 0.15
Nodes (5): map_skill_row(), sample_skill(), ScenarioRecord, SkillRecord, skips_memberships_referencing_missing_skill_or_scenario()

### Community 83 - "Community 83"
Cohesion: 0.25
Nodes (11): EmptySkillMetadata, find_skill_dirs(), marker_search_stops_at_depth_4(), parse_skill_md(), read_skill_document(), sanitises_names_and_parses_frontmatter(), sanitize_skill_name(), SkillDocument (+3 more)

### Community 84 - "Community 84"
Cohesion: 0.20
Nodes (13): recharts, ChartConfig, ChartContainer(), ChartContext, ChartContextProps, ChartLegendContent(), ChartStyle(), ChartTooltipContent() (+5 more)

### Community 86 - "Community 86"
Cohesion: 0.22
Nodes (4): ToolAdapter, ToolCategory, Coding, Lobster

### Community 87 - "Community 87"
Cohesion: 0.20
Nodes (9): decide_emit(), EmitAction, Defer, Emit, Skip, is_in_git_dir(), should_emit(), start_file_watcher() (+1 more)

### Community 88 - "Community 88"
Cohesion: 0.29
Nodes (7): file_watcher_emits_app_files_changed(), hive_db_migrates_to_v8_and_layout_is_created(), blocking_acquire_waits_then_times_out_and_recovers(), lock_file_lives_outside_skills_dir(), open_lock_file(), RepoLock, stamp_is_readable_by_another_handle_while_held()

### Community 89 - "Community 89"
Cohesion: 0.20
Nodes (8): baseArgs, canRun(), cargo, extraArgs, resolveCargo(), result, base, child

### Community 90 - "Community 90"
Cohesion: 0.26
Nodes (8): Sheet(), SheetContent(), SheetDescription(), SheetHeader(), SheetOverlay(), SheetPortal(), SheetTitle(), Sidebar()

### Community 91 - "Community 91"
Cohesion: 0.18
Nodes (11): GitCmd, Clone, Commit, Init, PruneSyncRefs, Pull, Push, Restore (+3 more)

### Community 92 - "Community 92"
Cohesion: 0.22
Nodes (4): Notifier, RecordingNotifier, StderrNotifier, TauriNotifier

### Community 93 - "Community 93"
Cohesion: 0.27
Nodes (9): base, call(), click(), driver, find(), promoteDevToolsPort(), promotePortFile(), walkForPort() (+1 more)

### Community 94 - "Community 94"
Cohesion: 0.22
Nodes (3): SelectContent(), SelectScrollDownButton(), SelectScrollUpButton()

### Community 95 - "Community 95"
Cohesion: 0.27
Nodes (10): config_file_path(), config_state_corrupt_json_is_invalid_not_fresh_install(), config_state_missing_file_is_missing(), config_state_valid_json_is_valid(), ConfigState, Invalid, Missing, Valid (+2 more)

### Community 96 - "Community 96"
Cohesion: 0.27
Nodes (10): classify_event_paths(), classify_mute(), mute_self_writes(), muted_at(), MuteVerdict, Foreign, Live, SelfWrite (+2 more)

### Community 97 - "Community 97"
Cohesion: 0.20
Nodes (4): membership(), membership(), ScenarioMetaFile, ScenarioSkillMetaFile

### Community 98 - "Community 98"
Cohesion: 0.20
Nodes (10): Command, Agents, Git, Presets, Repo, Skills, RepoCmd, ResetPath (+2 more)

### Community 99 - "Community 99"
Cohesion: 0.27
Nodes (10): classify_event_paths(), classify_mute(), mute_self_writes(), muted_at(), MuteVerdict, Foreign, Live, SelfWrite (+2 more)

### Community 100 - "Community 100"
Cohesion: 0.20
Nodes (9): keys, locales, missing, problems, root, tables, unused, used (+1 more)

### Community 101 - "Community 101"
Cohesion: 0.39
Nodes (7): BASE_DIR_OVERRIDE, HOME_DIR_OVERRIDE, LIBRARY_LEASE, SKILLS_DIR_OVERRIDE, STARTUP_ERROR_LOG, STARTUP_WARNINGS, TEST_BASE_DIR_GUARD

### Community 102 - "Community 102"
Cohesion: 0.39
Nodes (4): delete_tag_removes_from_all_skills(), rename_tag_to_itself_is_noop_not_delete(), rename_tag_updates_all_and_merges_into_existing(), skill()

### Community 103 - "Community 103"
Cohesion: 0.39
Nodes (7): BASE_DIR_OVERRIDE, HOME_DIR_OVERRIDE, LIBRARY_LEASE, SKILLS_DIR_OVERRIDE, STARTUP_ERROR_LOG, STARTUP_WARNINGS, TEST_BASE_DIR_GUARD

### Community 104 - "Community 104"
Cohesion: 0.31
Nodes (9): config_state_corrupt_json_is_invalid_not_fresh_install(), config_state_missing_file_is_missing(), config_state_valid_json_is_valid(), ConfigState, Invalid, Missing, Valid, load_config_state() (+1 more)

### Community 105 - "Community 105"
Cohesion: 0.33
Nodes (6): build_tray_menu_from_data(), left_click_restores(), tray_menu_lists_status_presets_and_quit(), TrayEntry, install(), label()

### Community 106 - "Community 106"
Cohesion: 0.25
Nodes (7): close_effect(), CloseChoice, Ask, Minimize, Quit, effective_close(), on_window()

### Community 108 - "Community 108"
Cohesion: 0.25
Nodes (8): AgentDefCmd, Validate, AgentsCmd, AddCustom, Def, Disable, Enable, List

### Community 110 - "Community 110"
Cohesion: 0.33
Nodes (6): baseArgs, canRun(), cargo, extraArgs, resolveCargo(), result

### Community 112 - "Community 112"
Cohesion: 0.38
Nodes (6): check, commandNames(), differs(), known, render(), selfTest

### Community 114 - "Community 114"
Cohesion: 0.33
Nodes (5): config_file_path(), downgrade_library_lease(), lease_slot(), take_library_lease(), test_base_dir_lock()

### Community 115 - "Community 115"
Cohesion: 0.47
Nodes (3): @tailwindcss/vite, vite, @vitejs/plugin-react

### Community 116 - "Community 116"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 117 - "Community 117"
Cohesion: 0.47
Nodes (3): get_settings(), list_base_entries(), set_settings()

### Community 118 - "Community 118"
Cohesion: 0.33
Nodes (5): compilerOptions, baseUrl, paths, files, references

### Community 119 - "Community 119"
Cohesion: 0.40
Nodes (4): downgrade_library_lease(), lease_slot(), take_library_lease(), test_base_dir_lock()

### Community 120 - "Community 120"
Cohesion: 0.40
Nodes (4): adapter_installed(), builtin_catalog(), global_and_project_dirs(), fifty_four_adapters_have_global_and_project_directories()

### Community 121 - "Community 121"
Cohesion: 0.40
Nodes (5): decide_emit(), EmitAction, Defer, Emit, Skip

### Community 123 - "Community 123"
Cohesion: 0.40
Nodes (4): files, header, roots, walk()

### Community 124 - "Community 124"
Cohesion: 0.60
Nodes (4): channel(), luminance(), pairs, ratio()

### Community 127 - "Community 127"
Cohesion: 0.50
Nodes (3): printWidth, singleQuote, trailingComma

### Community 129 - "Community 129"
Cohesion: 0.67
Nodes (3): MigrationOutcome, Proceed, UseSource

### Community 135 - "Community 135"
Cohesion: 0.67
Nodes (3): MigrationOutcome, Proceed, UseSource

### Community 136 - "Community 136"
Cohesion: 1.00
Nodes (3): sidecar_dir(), sidecar_read_dir(), skills_dir()

### Community 137 - "Community 137"
Cohesion: 0.67
Nodes (3): hive-cli, hive-core, skill-hive

## Knowledge Gaps
- **182 isolated node(s):** `ClaudeCode`, `Portable`, `Error`, `Info`, `Warning` (+177 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 977 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **54 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `react` connect `Community 26` to `Community 16`, `Community 19`, `Community 21`, `Community 29`, `Community 31`, `Community 45`, `Community 46`, `Community 51`, `Community 52`, `Community 60`, `Community 64`, `Community 68`, `Community 73`, `Community 74`, `Community 79`, `Community 80`, `Community 84`, `Community 90`, `Community 94`, `Community 107`, `Community 109`, `Community 113`?**
  _High betweenness centrality (0.023) - this node is a cross-community bridge._
- **What connects `ClaudeCode`, `Portable`, `Error` to the rest of the system?**
  _182 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.07407407407407407 - nodes in this community are weakly interconnected._
- **Why does `SkillStore` connect `Community 5` to `Community 97`, `Community 82`, `Community 38`?**
  _High betweenness centrality (0.019) - this node is a cross-community bridge._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.08735150244584207 - nodes in this community are weakly interconnected._
- **Should `Community 2` be split into smaller, more focused modules?**
  _Cohesion score 0.05152394775036284 - nodes in this community are weakly interconnected._
- **Should `Community 3` be split into smaller, more focused modules?**
  _Cohesion score 0.06787330316742081 - nodes in this community are weakly interconnected._