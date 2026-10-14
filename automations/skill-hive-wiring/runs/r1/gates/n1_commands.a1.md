# gate n1_commands attempt 1: pass (2026-10-14T17:06:47+00:00)

snapshot before this attempt: n/a (node is not risky)

## check.cmd: pass

```
`grep -q 'ipc-ok' automations/skill-hive-wiring/out/n1_commands.md` → exit 0: (no output)
```

## node.report: pass

```
exit 0: src/lib/ipc/ipc.d.ts matches get_settings, set_settings, log_startup_event, check_app_update, update_install_blocker, updater_proxy, list_base_entries, get_managed_skills, list_presets, get_tool_status, scan_local_skills, install_local_skill, install_archive_skill, import_existing_skill, import_all_discovered, preview_git_install, confirm_git_install_skill, cancel_install, delete_managed_skills, set_skills_enabled, set_skill_tags, rename_tag, delete_tag, sync_skill_to_tool, unsync_skill_from_tool, add_skill_to_preset, remove_skill_from_preset, reorder_preset_skills, create_preset, update_preset, delete_preset, reorder_presets, apply_preset, add_custom_tool, remove_custom_tool, set_tool_enabled, set_all_tools_enabled, set_tool_order_cmd, set_custom_tool_path, reset_custom_tool_path, set_custom_tool_project_path, get_central_repo_path, set_central_repo_path, check_all_skill_updates, check_skill_update, git_backup_status, git_backup_list_versions, git_backup_sync, git_backup_restore_version, git_backup_set_remote, git_backup_remove_remote, backup_set_device_name, add_project, fetch_leaderboard, search_skillssh, install_from_skillssh, record_effect
```
