# Graph Report - workspace  (2026-10-14)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 2852 nodes · 5919 edges · 175 communities (112 shown, 63 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 258 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `c6b4d60e`
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
- Community 73
- Community 74
- Community 75
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
- Community 111
- Community 112
- Community 113
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
- Community 143
- Community 144
- Community 147

## God Nodes (most connected - your core abstractions)
1. `SkillStore` - 70 edges
2. `SkillStore` - 68 edges
3. `react` - 49 edges
4. `cn` - 36 edges
5. `t()` - 33 edges
6. `AppShell()` - 28 edges
7. `radix-ui` - 28 edges
8. `main()` - 24 edges
9. `Document` - 24 edges
10. `SkillEditor()` - 23 edges

## Surprising Connections (you probably didn't know these)
- `on_window()` --calls--> `close_effect()`  [INFERRED]
  src-tauri/src/tray.rs → crates/hive-core/src/util/tray_menu.rs
- `on_window()` --calls--> `effective_close()`  [INFERRED]
  src-tauri/src/tray.rs → crates/hive-core/src/util/tray_menu.rs
- `install()` --calls--> `left_click_restores()`  [INFERRED]
  src-tauri/src/tray.rs → crates/hive-core/src/util/tray_menu.rs
- `cli_command()` --references--> `Command`  [EXTRACTED]
  automations/skill-hive-p0/sources/reference/cli_bridge.rs → crates/hive-cli/src/lib.rs
- `membership()` --references--> `ScenarioSkillMetaFile`  [EXTRACTED]
  automations/skill-hive-p0/sources/reference/skill_store.rs → crates/hive-core/src/library/sync_metadata.rs

## Import Cycles
- None detected.

## Communities (175 total, 63 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.05
Nodes (88): AgentDraft, applyAgentFix(), applySkillFix(), bodyStats(), CLAUDE_KEYS, deployFixture(), Deployment, descriptionBudget() (+80 more)

### Community 1 - "Community 1"
Cohesion: 0.05
Nodes (85): a_bin_dir_outside_the_default_home_is_not_debris(), a_move_that_finished_unrecorded_still_repoints(), app_file_names_count_as_debris_only_at_the_target_root(), base_dir(), BASE_DIR_OVERRIDE, base_dir_override_active(), cache_dir(), clear_regenerable_target() (+77 more)

### Community 2 - "Community 2"
Cohesion: 0.06
Nodes (45): status_name(), canonical_root(), compare_bodies(), dedupe_agents_sharing_root(), dedupes_agents_sharing_a_root(), disable_by_moving(), disable_moves_directory_to_disabled_sibling(), disabled_sibling() (+37 more)

### Community 3 - "Community 3"
Cohesion: 0.09
Nodes (48): agent_description_budget(), estimate_tokens(), command_rules_and_agent_rows(), agent_budget_finding(), body_rules(), claude_enums(), CLAUDE_KEYS, claude_only_syntax() (+40 more)

### Community 4 - "Community 4"
Cohesion: 0.07
Nodes (10): profile_name(), body_gauge_references_and_silent_skip_reasons(), every_rule_id_has_a_function_path_and_the_suite_agrees_with_quick_validate(), python_bin(), quick_validate(), Qv, rename_refuses_unsafe_names_and_updates_evals(), skill() (+2 more)

### Community 5 - "Community 5"
Cohesion: 0.07
Nodes (38): Document, move_skill_tool(), block_style(), Document, editing_description_touches_only_that_key(), Entry, is_block_indicator(), parse_document() (+30 more)

### Community 6 - "Community 6"
Cohesion: 0.13
Nodes (26): versions_dedup_pin_restore_and_history_schema(), AUTOSAVE_KEEP, copy_subject(), DraftVersion, export_history_json(), FileHash, hash_files(), hash_manifest() (+18 more)

### Community 7 - "Community 7"
Cohesion: 0.05
Nodes (14): FALLBACK_NAME, is_path_safe(), MAX_NAME_LEN, normalize(), resolve_non_existing(), sanitize_name(), sanitize_truncates_long_names(), FALLBACK_NAME (+6 more)

### Community 8 - "Community 8"
Cohesion: 0.05
Nodes (3): ProjectRecord, SkillStore, SkillTargetRecord

### Community 9 - "Community 9"
Cohesion: 0.19
Nodes (32): Bundle, cmd_accept(), cmd_approve(), cmd_baseline(), cmd_brief(), cmd_complete(), cmd_decide(), cmd_dispatch() (+24 more)

### Community 10 - "Community 10"
Cohesion: 0.10
Nodes (36): apply_skill_fix(), BuiltInTemplate, Blank, DocumentProducer, MultiVariant, ReferenceHeavy, ScriptBacked, Workflow (+28 more)

### Community 11 - "Community 11"
Cohesion: 0.06
Nodes (25): AppError, classify_git_error_case_insensitive(), classify_git_error_detects_canceled_american_spelling(), classify_git_error_detects_cancelled(), classify_git_error_detects_connection_refused(), classify_git_error_detects_could_not_resolve_host(), classify_git_error_regular_git_error(), display_shows_message() (+17 more)

### Community 12 - "Community 12"
Cohesion: 0.09
Nodes (24): classify_target(), conflict(), deploy_skill(), DeployOutcome, is_current(), is_junction(), is_unc_path(), junction_path_is_exercised() (+16 more)

### Community 13 - "Community 13"
Cohesion: 0.08
Nodes (19): Input(), Separator(), SidebarContext, SidebarContextProps, SidebarInput(), SidebarMenuButton(), sidebarMenuButtonVariants, SidebarMenuSkeleton() (+11 more)

### Community 14 - "Community 14"
Cohesion: 0.06
Nodes (4): git_preview_rename_imports_the_selected_skill(), open_store(), sidecar_less_library_is_refused_and_rebuilt_from_the_database(), skills_manager_import_reproduces_counts_and_order()

### Community 15 - "Community 15"
Cohesion: 0.07
Nodes (22): AppError, classify_git_error_case_insensitive(), classify_git_error_detects_canceled_american_spelling(), classify_git_error_detects_cancelled(), classify_git_error_detects_connection_refused(), classify_git_error_detects_could_not_resolve_host(), classify_git_error_regular_git_error(), display_shows_message() (+14 more)

### Community 16 - "Community 16"
Cohesion: 0.09
Nodes (25): is_safe_zip_name(), safe_extract(), cache_expires(), cancelled_install_writes_nothing(), http_get(), install_zip_bytes(), leaderboard_falls_back_when_next_data_is_absent(), MarketSkill (+17 more)

### Community 18 - "Community 18"
Cohesion: 0.07
Nodes (33): license, name, private, type, version, codemirror, @codemirror/lang-json, @codemirror/lang-markdown (+25 more)

### Community 19 - "Community 19"
Cohesion: 0.10
Nodes (25): adapter_installed(), add_custom_tool(), BUILTIN_ADAPTER_COUNT, builtin_catalog(), CustomAdd, Added, SkippedCollision, disable_all() (+17 more)

### Community 20 - "Community 20"
Cohesion: 0.12
Nodes (30): addSkillToAgents(), applyRemoval(), backupNow(), BackupSnapshot, CloseChoice, DotColor, effectiveClose(), filterMarket() (+22 more)

### Community 21 - "Community 21"
Cohesion: 0.10
Nodes (24): corrupted_key_file_returns_error(), decrypt(), different_nonces_each_time(), ENC_PREFIX, encrypt(), key_file_has_owner_only_permissions(), load_or_create_key(), load_or_create_key_creates_and_reloads() (+16 more)

### Community 22 - "Community 22"
Cohesion: 0.13
Nodes (23): all_tool_adapters(), antigravity_uses_current_default_skills_path(), apply_builtin_path_overrides(), claude_code_does_not_scan_plugin_marketplaces_by_default(), custom_omp_agent_collision_keeps_builtin_adapter(), custom_tool_adapter(), custom_tool_paths(), custom_tool_project_paths() (+15 more)

### Community 23 - "Community 23"
Cohesion: 0.10
Nodes (24): react-dom, react-router-dom, @tanstack/react-query, @tauri-apps/api, @testing-library/jest-dom, @testing-library/react, @testing-library/user-event, vitest (+16 more)

### Community 24 - "Community 24"
Cohesion: 0.14
Nodes (23): all_tool_adapters(), antigravity_uses_current_default_skills_path(), apply_builtin_path_overrides(), claude_code_does_not_scan_plugin_marketplaces_by_default(), custom_omp_agent_collision_keeps_builtin_adapter(), custom_tool_adapter(), custom_tool_paths(), custom_tool_project_paths() (+15 more)

### Community 25 - "Community 25"
Cohesion: 0.09
Nodes (7): cn, radix-ui, react, Badge(), badgeVariants, ScrollArea(), ScrollBar()

### Community 26 - "Community 26"
Cohesion: 0.11
Nodes (17): archive_and_directory_hash_equal(), batch_import_folder(), install_archive(), install_local(), install_tree(), InstalledSkill, MAX_ZIP_ENTRIES, MAX_ZIP_UNCOMPRESSED_BYTES (+9 more)

### Community 27 - "Community 27"
Cohesion: 0.10
Nodes (24): clear_regenerable_target(), CONFIG_FILE_NAME, copy_dir_recursive(), copy_dir_recursive_copies_read_only_source_files(), copy_dir_recursive_keeps_links_as_links(), copy_symlink(), default_base_dir(), external_base_dir() (+16 more)

### Community 28 - "Community 28"
Cohesion: 0.12
Nodes (26): cache_evictions(), CACHE_LIMIT_BYTES, CacheSlot, cancel_git_preview(), clone_timeout_kills_a_slow_command(), CLONE_TIMEOUT_SECS, ClonePlan, confirm_git_install() (+18 more)

### Community 29 - "Community 29"
Cohesion: 0.09
Nodes (9): execute(), execute_isolated(), execute_locked(), finish(), lock(), now_ms(), Output, write_tree() (+1 more)

### Community 30 - "Community 30"
Cohesion: 0.12
Nodes (19): remove_deployed_link(), import_all(), import_scanned(), scan_roots(), ScannedSkill, Adopt, UserConfirmed, center_changed_after_edit() (+11 more)

### Community 31 - "Community 31"
Cohesion: 0.12
Nodes (16): APP_FS_CHANGED_EVENT, collect_watch_paths(), dir_has_entries(), EPOCH, insert_non_linked_project(), linked_workspace_watch_paths_only_include_selected_roots(), MUTE_STATE, MuteState (+8 more)

### Community 32 - "Community 32"
Cohesion: 0.12
Nodes (16): APP_FS_CHANGED_EVENT, collect_watch_paths(), dir_has_entries(), EPOCH, insert_non_linked_project(), linked_workspace_watch_paths_only_include_selected_roots(), MUTE_STATE, MuteState (+8 more)

### Community 33 - "Community 33"
Cohesion: 0.15
Nodes (23): add_membership(), apply_preset(), claimed_paths(), DeployIntent, Add, Remove, desired_targets(), DesiredTarget (+15 more)

### Community 34 - "Community 34"
Cohesion: 0.07
Nodes (27): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+19 more)

### Community 35 - "Community 35"
Cohesion: 0.15
Nodes (22): Kbd(), ACTIONS, AppShell(), runAction(), ShellOutletContext, SHORTCUTS, demoSettingKey(), getSettings() (+14 more)

### Community 36 - "Community 36"
Cohesion: 0.08
Nodes (26): dependencies, class-variance-authority, cmdk, cn, codemirror, @codemirror/lang-json, @codemirror/lang-markdown, @codemirror/lang-python (+18 more)

### Community 37 - "Community 37"
Cohesion: 0.15
Nodes (12): blocking_acquire_waits_then_times_out_and_recovers(), FOREGROUND_WAIT, lock_file_lives_outside_skills_dir(), LOCK_FILE_NAME, open_lock_file(), POLL_INTERVAL, RepoLock, stamp_is_readable_by_another_handle_while_held() (+4 more)

### Community 38 - "Community 38"
Cohesion: 0.21
Nodes (21): add_column_if_missing(), count_setting(), has_column(), LATEST_VERSION, migrate_step(), migrate_v0_to_v1(), migrate_v1_to_v2(), migrate_v2_to_v3() (+13 more)

### Community 39 - "Community 39"
Cohesion: 0.12
Nodes (7): install_panic_hook(), last_panic_path(), LOG_DIR, collect(), hash_skill_dir(), normalise_newlines(), LOG_DIR

### Community 40 - "Community 40"
Cohesion: 0.14
Nodes (8): delete_tag_removes_from_all_skills(), log_audit_appends_and_lists_newest_first(), log_audit_respects_limit(), rename_tag_to_itself_is_noop_not_delete(), rename_tag_updates_all_and_merges_into_existing(), ScenarioSkillToolToggleRecord, SENSITIVE_KEYS, skill()

### Community 41 - "Community 41"
Cohesion: 0.13
Nodes (14): cmd_validate(), compute_ready(), console(), graphify_cmd(), in_scope(), nodes_hash(), one_line(), render_brief() (+6 more)

### Community 42 - "Community 42"
Cohesion: 0.21
Nodes (22): add_column_if_missing(), count_setting(), has_column(), LATEST_VERSION, migrate_step(), migrate_v0_to_v1(), migrate_v1_to_v2(), migrate_v2_to_v3() (+14 more)

### Community 43 - "Community 43"
Cohesion: 0.12
Nodes (16): StatusBanner(), applyTextSize(), isTextSize(), Locale, LOCALES, setLocale(), t(), TABLES (+8 more)

### Community 44 - "Community 44"
Cohesion: 0.09
Nodes (23): SkillsCmd, Adopt, Check, Deploy, Disable, Enable, Export, Install (+15 more)

### Community 45 - "Community 45"
Cohesion: 0.09
Nodes (23): devDependencies, eslint, @eslint/js, eslint-plugin-react-hooks, eslint-plugin-react-refresh, globals, jsdom, @playwright/test (+15 more)

### Community 47 - "Community 47"
Cohesion: 0.19
Nodes (18): emit_module(), port_central_repo(), port_cli_bridge(), port_error(), port_file_watcher(), port_log_sanitize(), port_path_guard(), port_repo_lock() (+10 more)

### Community 48 - "Community 48"
Cohesion: 0.21
Nodes (17): DiffRow, fileDiffs(), filterSkills(), lcsDiff(), LibraryFilter, moveById(), moveItem(), PresetFill (+9 more)

### Community 49 - "Community 49"
Cohesion: 0.09
Nodes (21): compilerOptions, allowImportingTsExtensions, baseUrl, isolatedModules, jsx, lib, module, moduleDetection (+13 more)

### Community 50 - "Community 50"
Cohesion: 0.20
Nodes (14): begin_device_flow(), device_flow_and_pat_against_a_fixture(), DeviceCodes, DevicePoll, Approved, Denied, Pending, find_or_create_private_repo() (+6 more)

### Community 51 - "Community 51"
Cohesion: 0.28
Nodes (18): agents(), compare_skill(), Ctx, dispatch(), Fail, App, Other, Usage (+10 more)

### Community 52 - "Community 52"
Cohesion: 0.18
Nodes (13): AgentExport, AgentExportRequest, apply_agent_fix(), export_agent(), render_agent_file(), render_agents_cli_json(), render_key(), scalar_json() (+5 more)

### Community 53 - "Community 53"
Cohesion: 0.16
Nodes (10): cmdk, Command(), CommandDialog(), Dialog(), DialogContent(), DialogDescription(), DialogHeader(), DialogOverlay() (+2 more)

### Community 54 - "Community 54"
Cohesion: 0.18
Nodes (18): DashboardPage(), InstallPage(), LibraryPage(), LobsterPage(), APP_ROUTES, AppRoute, NAV_ITEMS, screenFor() (+10 more)

### Community 55 - "Community 55"
Cohesion: 0.17
Nodes (11): an_environment_argument_string_keeps_every_flag(), an_environment_port_is_forwarded_without_driver_arguments(), box_err(), driver_log(), driver_switches_become_webview_arguments(), get_settings(), list_base_entries(), run() (+3 more)

### Community 56 - "Community 56"
Cohesion: 0.26
Nodes (17): a_bridge_that_cannot_be_invalidated_blocks_the_publish(), a_failed_republish_leaves_the_previous_bridge_unusable(), BRIDGE_BIN_NAME, bridge_dir(), bridge_path(), bundled_cli(), cli_command(), ensure_bridge() (+9 more)

### Community 58 - "Community 58"
Cohesion: 0.23
Nodes (19): base_dir(), base_dir_override_active(), cache_dir(), clear_repoint_from(), configured_base_dir(), db_path(), ensure_central_repo(), live_base_from() (+11 more)

### Community 59 - "Community 59"
Cohesion: 0.22
Nodes (15): copy_path(), has_real_directories(), import_skills_manager_library(), ImportReport, json_dir_has_files(), list_json(), read_memberships(), read_scenarios() (+7 more)

### Community 60 - "Community 60"
Cohesion: 0.26
Nodes (17): a_bridge_that_cannot_be_invalidated_blocks_the_publish(), a_failed_republish_leaves_the_previous_bridge_unusable(), BRIDGE_BIN_NAME, bridge_dir(), bridge_path(), bundled_cli(), cli_command(), ensure_bridge() (+9 more)

### Community 61 - "Community 61"
Cohesion: 0.14
Nodes (9): class-variance-authority, Alert(), alertVariants, TabsList(), tabsListVariants, ToggleGroupContext, ToggleGroupItem(), Toggle() (+1 more)

### Community 62 - "Community 62"
Cohesion: 0.21
Nodes (13): home_replacement(), other_rules(), redacts_email_and_token(), redacts_git_credentials(), replaces_generic_users_path(), sanitize(), home_replacement(), other_rules() (+5 more)

### Community 63 - "Community 63"
Cohesion: 0.11
Nodes (17): aliases, components, hooks, lib, ui, utils, iconLibrary, rsc (+9 more)

### Community 64 - "Community 64"
Cohesion: 0.28
Nodes (16): backup_round_trip_against_local_bare_remote(), BackupStatus, clone_repo(), commit_all(), git(), init_bare(), init_repo(), prune_sync_refs() (+8 more)

### Community 65 - "Community 65"
Cohesion: 0.16
Nodes (18): a_bin_dir_outside_the_default_home_is_not_debris(), a_move_that_finished_unrecorded_still_repoints(), app_file_names_count_as_debris_only_at_the_target_root(), config_migrating(), directory_has_entries(), live_base_is_the_move_source_until_the_move_happens(), migrate_repo_if_needed(), migration_does_not_follow_a_link_disguised_as_a_skeleton_dir() (+10 more)

### Community 66 - "Community 66"
Cohesion: 0.32
Nodes (12): cname(), communities(), cross_edges(), focus(), god_nodes(), is_internal(), load(), load_labels() (+4 more)

### Community 67 - "Community 67"
Cohesion: 0.21
Nodes (11): append_under(), render_verify_brief(), fail(), main(), ok(), packaged_skill_mds(), walk(), parse_frontmatter() (+3 more)

### Community 68 - "Community 68"
Cohesion: 0.12
Nodes (17): scripts, build, check:headers, check:i18n, check:palette, cli, cli:build, cli:install (+9 more)

### Community 69 - "Community 69"
Cohesion: 0.16
Nodes (8): AlertDialogAction(), AlertDialogCancel(), AlertDialogContent(), AlertDialogOverlay(), AlertDialogPortal(), Button(), buttonVariants, DialogFooter()

### Community 70 - "Community 70"
Cohesion: 0.12
Nodes (16): compilerOptions, allowImportingTsExtensions, isolatedModules, lib, module, moduleDetection, moduleResolution, noEmit (+8 more)

### Community 72 - "Community 72"
Cohesion: 0.23
Nodes (9): check(), command_ids(), SCHEMAS, validate(), contract_suite_exercises_every_subcommand(), data_of(), deploy_into_an_occupied_target_returns_target_conflict(), record() (+1 more)

### Community 73 - "Community 73"
Cohesion: 0.26
Nodes (15): backend_matches_platform_policy(), CredentialKind, Keyring, LabelledFallback, credentials_round_trip_on_the_platform_backend(), delete_credential(), fallback_path(), keyring_get() (+7 more)

### Community 74 - "Community 74"
Cohesion: 0.17
Nodes (5): AgentRow, log_audit_appends_and_lists_newest_first(), log_audit_respects_limit(), ScenarioSkillToolToggleRecord, SENSITIVE_KEYS

### Community 78 - "Community 78"
Cohesion: 0.13
Nodes (15): PresetsCmd, AddSkill, Apply, Create, Current, Deactivate, Delete, Deploy (+7 more)

### Community 79 - "Community 79"
Cohesion: 0.19
Nodes (12): build_tray_menu_from_data(), close_effect(), CloseChoice, Ask, Minimize, Quit, effective_close(), left_click_restores() (+4 more)

### Community 80 - "Community 80"
Cohesion: 0.27
Nodes (12): call(), listAgents(), listPresets(), listScanned(), listSkills(), readSetting(), ScanRow, writeSetting() (+4 more)

### Community 81 - "Community 81"
Cohesion: 0.15
Nodes (5): map_skill_row(), sample_skill(), ScenarioRecord, SkillRecord, skips_memberships_referencing_missing_skill_or_scenario()

### Community 82 - "Community 82"
Cohesion: 0.25
Nodes (11): EmptySkillMetadata, find_skill_dirs(), marker_search_stops_at_depth_4(), parse_skill_md(), read_skill_document(), sanitises_names_and_parses_frontmatter(), sanitize_skill_name(), SkillDocument (+3 more)

### Community 83 - "Community 83"
Cohesion: 0.20
Nodes (13): recharts, ChartConfig, ChartContainer(), ChartContext, ChartContextProps, ChartLegendContent(), ChartStyle(), ChartTooltipContent() (+5 more)

### Community 84 - "Community 84"
Cohesion: 0.21
Nodes (10): FormControl(), FormDescription(), FormFieldContext, FormFieldContextValue, FormItemContext, FormItemContextValue, FormLabel(), FormMessage() (+2 more)

### Community 85 - "Community 85"
Cohesion: 0.20
Nodes (9): decide_emit(), EmitAction, Defer, Emit, Skip, is_in_git_dir(), should_emit(), start_file_watcher() (+1 more)

### Community 87 - "Community 87"
Cohesion: 0.18
Nodes (4): map_skill_row(), sample_skill(), SkillRecord, skips_memberships_referencing_missing_skill_or_scenario()

### Community 88 - "Community 88"
Cohesion: 0.20
Nodes (11): file_watcher_emits_app_files_changed(), decide_emit(), EmitAction, Defer, Emit, Skip, is_in_git_dir(), notify_central_change() (+3 more)

### Community 89 - "Community 89"
Cohesion: 0.20
Nodes (8): baseArgs, canRun(), cargo, extraArgs, resolveCargo(), result, base, child

### Community 90 - "Community 90"
Cohesion: 0.26
Nodes (8): Sheet(), SheetContent(), SheetDescription(), SheetHeader(), SheetOverlay(), SheetPortal(), SheetTitle(), Sidebar()

### Community 91 - "Community 91"
Cohesion: 0.18
Nodes (11): Cli, Command, Agents, Git, Presets, Repo, Skills, RepoCmd (+3 more)

### Community 92 - "Community 92"
Cohesion: 0.18
Nodes (11): GitCmd, Clone, Commit, Init, PruneSyncRefs, Pull, Push, Restore (+3 more)

### Community 93 - "Community 93"
Cohesion: 0.33
Nodes (6): hive_db_migrates_to_v8_and_layout_is_created(), blocking_acquire_waits_then_times_out_and_recovers(), lock_file_lives_outside_skills_dir(), open_lock_file(), RepoLock, stamp_is_readable_by_another_handle_while_held()

### Community 94 - "Community 94"
Cohesion: 0.22
Nodes (4): Notifier, RecordingNotifier, StderrNotifier, TauriNotifier

### Community 95 - "Community 95"
Cohesion: 0.27
Nodes (9): base, call(), click(), driver, find(), promoteDevToolsPort(), promotePortFile(), walkForPort() (+1 more)

### Community 96 - "Community 96"
Cohesion: 0.22
Nodes (3): SelectContent(), SelectScrollDownButton(), SelectScrollUpButton()

### Community 97 - "Community 97"
Cohesion: 0.20
Nodes (3): AuditDraft, AuditEntry, MAX_ENTRIES

### Community 98 - "Community 98"
Cohesion: 0.27
Nodes (10): classify_event_paths(), classify_mute(), mute_self_writes(), muted_at(), MuteVerdict, Foreign, Live, SelfWrite (+2 more)

### Community 99 - "Community 99"
Cohesion: 0.20
Nodes (4): membership(), membership(), ScenarioMetaFile, ScenarioSkillMetaFile

### Community 100 - "Community 100"
Cohesion: 0.27
Nodes (10): config_file_path(), config_state_corrupt_json_is_invalid_not_fresh_install(), config_state_missing_file_is_missing(), config_state_valid_json_is_valid(), ConfigState, Invalid, Missing, Valid (+2 more)

### Community 101 - "Community 101"
Cohesion: 0.20
Nodes (3): AuditDraft, AuditEntry, MAX_ENTRIES

### Community 102 - "Community 102"
Cohesion: 0.27
Nodes (10): classify_event_paths(), classify_mute(), mute_self_writes(), muted_at(), MuteVerdict, Foreign, Live, SelfWrite (+2 more)

### Community 103 - "Community 103"
Cohesion: 0.20
Nodes (9): keys, locales, missing, problems, root, tables, unused, used (+1 more)

### Community 104 - "Community 104"
Cohesion: 0.39
Nodes (4): delete_tag_removes_from_all_skills(), rename_tag_to_itself_is_noop_not_delete(), rename_tag_updates_all_and_merges_into_existing(), skill()

### Community 105 - "Community 105"
Cohesion: 0.39
Nodes (7): BASE_DIR_OVERRIDE, HOME_DIR_OVERRIDE, LIBRARY_LEASE, SKILLS_DIR_OVERRIDE, STARTUP_ERROR_LOG, STARTUP_WARNINGS, TEST_BASE_DIR_GUARD

### Community 108 - "Community 108"
Cohesion: 0.25
Nodes (8): AgentDefCmd, Validate, AgentsCmd, AddCustom, Def, Disable, Enable, List

### Community 111 - "Community 111"
Cohesion: 0.33
Nodes (6): baseArgs, canRun(), cargo, extraArgs, resolveCargo(), result

### Community 112 - "Community 112"
Cohesion: 0.29
Nodes (6): downgrade_library_lease(), lease_slot(), migration_clears_regenerable_leftovers_and_records_repoint(), set_test_home_dir_override(), take_library_lease(), test_base_dir_lock()

### Community 113 - "Community 113"
Cohesion: 0.38
Nodes (6): check, commandNames(), differs(), known, render(), selfTest

### Community 114 - "Community 114"
Cohesion: 0.60
Nodes (4): AutoBackupPolicy, backoff_delay(), due(), quiet_period_and_backoff()

### Community 115 - "Community 115"
Cohesion: 0.47
Nodes (3): @tailwindcss/vite, vite, @vitejs/plugin-react

### Community 116 - "Community 116"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 117 - "Community 117"
Cohesion: 0.33
Nodes (5): compilerOptions, baseUrl, paths, files, references

### Community 118 - "Community 118"
Cohesion: 0.40
Nodes (5): TargetClass, Junction, Missing, RealDirectory, Symlink

### Community 120 - "Community 120"
Cohesion: 0.40
Nodes (4): files, header, roots, walk()

### Community 121 - "Community 121"
Cohesion: 0.60
Nodes (4): channel(), luminance(), pairs, ratio()

### Community 124 - "Community 124"
Cohesion: 0.67
Nodes (4): sidecar_dir(), sidecar_read_dir(), skills_dir(), sidecar_reader_falls_back_to_skills_manager_name()

### Community 125 - "Community 125"
Cohesion: 0.50
Nodes (3): printWidth, singleQuote, trailingComma

### Community 137 - "Community 137"
Cohesion: 0.67
Nodes (3): hive-cli, hive-core, skill-hive

## Knowledge Gaps
- **182 isolated node(s):** `Missing`, `Proceed`, `UseSource`, `Blank`, `DocumentProducer` (+177 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 1015 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **63 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `react` connect `Community 25` to `Community 0`, `Community 13`, `Community 18`, `Community 20`, `Community 23`, `Community 35`, `Community 43`, `Community 48`, `Community 53`, `Community 54`, `Community 61`, `Community 69`, `Community 75`, `Community 76`, `Community 77`, `Community 80`, `Community 83`, `Community 84`, `Community 90`, `Community 96`, `Community 107`, `Community 109`, `Community 110`?**
  _High betweenness centrality (0.032) - this node is a cross-community bridge._
- **What connects `Missing`, `Proceed`, `UseSource` to the rest of the system?**
  _182 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.05235103750237959 - nodes in this community are weakly interconnected._
- **Why does `SkillStore` connect `Community 8` to `Community 128`, `Community 129`, `Community 99`, `Community 40`, `Community 81`?**
  _High betweenness centrality (0.024) - this node is a cross-community bridge._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.05016797312430011 - nodes in this community are weakly interconnected._
- **Why does `SkillStore` connect `Community 17` to `Community 99`, `Community 134`, `Community 135`, `Community 104`, `Community 136`, `Community 74`, `Community 87`, `Community 122`, `Community 123`?**
  _High betweenness centrality (0.017) - this node is a cross-community bridge._
- **Should `Community 2` be split into smaller, more focused modules?**
  _Cohesion score 0.05656565656565657 - nodes in this community are weakly interconnected._