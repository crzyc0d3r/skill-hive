import fs from "node:fs";

const rustPaths = ["src-tauri/src/lib.rs", "src-tauri/src/commands.rs"];
const dtsPath = "src/lib/ipc/ipc.d.ts";

const known = {
  get_settings: { args: "{ key: string }", result: "string | null" },
  set_settings: { args: "{ key: string; value: string }", result: "null" },
  log_startup_event: { args: "{ label: string; elapsedMs: number }", result: "null" },
  list_base_entries: { args: "Record<string, never>", result: "string[]" },
  check_app_update: { args: "Record<string, never>", result: "string" },
  update_install_blocker: { args: "{ activeRuns: number }", result: "string | null" },
  updater_proxy: { args: "{ url: string }", result: "string | null" },
  get_managed_skills: { args: "Record<string, never>", result: "import('@/features/library/model').SkillCard[]" },
  list_presets: { args: "Record<string, never>", result: "import('@/lib/ipc/catalog').PresetRow[]" },
  get_tool_status: { args: "Record<string, never>", result: "import('@/lib/ipc/catalog').AgentRow[]" },
  scan_local_skills: { args: "Record<string, never>", result: "import('@/lib/ipc/catalog').ScanRow[]" },
  install_local_skill: { args: "{ path: string }", result: "{ id: string; name: string }" },
  install_archive_skill: { args: "{ path: string }", result: "{ id: string; name: string }" },
  import_existing_skill: { args: "{ path: string; rename: string | null }", result: "{ id: string; name: string }" },
  import_all_discovered: { args: "Record<string, never>", result: "string[]" },
  preview_git_install: { args: "{ url: string }", result: "{ id: string; skills: { relative: string; name: string }[] }" },
  confirm_git_install_skill: { args: "{ previewId: string; relative: string; rename: string | null }", result: "string[]" },
  cancel_install: { args: "{ previewId: string | null }", result: "boolean" },
  delete_managed_skills: { args: "{ ids: string[] }", result: "null" },
  set_skills_enabled: { args: "{ ids: string[]; enabled: boolean }", result: "null" },
  set_skill_tags: { args: "{ skillId: string; tags: string[] }", result: "null" },
  rename_tag: { args: "{ from: string; to: string }", result: "string[]" },
  delete_tag: { args: "{ tag: string }", result: "string[]" },
  sync_skill_to_tool: { args: "{ skillId: string; tool: string }", result: "null" },
  unsync_skill_from_tool: { args: "{ skillId: string; tool: string }", result: "null" },
  add_skill_to_preset: { args: "{ presetId: string; skillId: string }", result: "null" },
  remove_skill_from_preset: { args: "{ presetId: string; skillId: string }", result: "null" },
  reorder_preset_skills: { args: "{ presetId: string; skillIds: string[] }", result: "null" },
  create_preset: { args: "{ name: string }", result: "string" },
  update_preset: { args: "{ id: string; name: string }", result: "null" },
  delete_preset: { args: "{ id: string }", result: "null" },
  reorder_presets: { args: "{ ids: string[] }", result: "null" },
  apply_preset: { args: "{ presetId: string; tool: string | null }", result: "number" },
  add_custom_tool: { args: "{ name: string; path: string; projectPath: string | null }", result: "import('@/lib/ipc/catalog').AgentRow" },
  remove_custom_tool: { args: "{ key: string }", result: "null" },
  set_tool_enabled: { args: "{ key: string; enabled: boolean }", result: "null" },
  set_all_tools_enabled: { args: "{ enabled: boolean }", result: "null" },
  set_tool_order_cmd: { args: "{ keys: string[] }", result: "null" },
  set_custom_tool_path: { args: "{ key: string; path: string }", result: "null" },
  reset_custom_tool_path: { args: "{ key: string }", result: "string" },
  set_custom_tool_project_path: { args: "{ key: string; path: string }", result: "null" },
  get_central_repo_path: { args: "Record<string, never>", result: "string" },
  set_central_repo_path: { args: "{ path: string }", result: "string" },
  check_all_skill_updates: { args: "Record<string, never>", result: "number" },
  check_skill_update: { args: "{ skillId: string }", result: "string" },
  git_backup_status: { args: "Record<string, never>", result: "{ branch: string; clean: boolean; remote: string | null; head: string }" },
  git_backup_list_versions: { args: "Record<string, never>", result: "{ id: string; label: string }[]" },
  git_backup_sync: { args: "{ message: string | null }", result: "{ id: string; label: string }" },
  git_backup_restore_version: { args: "{ tag: string }", result: "string" },
  git_backup_set_remote: { args: "{ url: string }", result: "null" },
  git_backup_remove_remote: { args: "Record<string, never>", result: "null" },
  backup_set_device_name: { args: "{ name: string }", result: "null" },
  add_project: { args: "{ name: string; path: string }", result: "string" },
  fetch_leaderboard: { args: "{ board: string | null }", result: "{ owner: string; name: string; installs: number; source: string }[]" },
  search_skillssh: { args: "{ query: string; limit: number }", result: "{ owner: string; name: string; installs: number; source: string }[]" },
  install_from_skillssh: { args: "{ source: string; name: string }", result: "{ id: string; name: string }" },
  cancel_market_install: { args: "Record<string, never>", result: "boolean" },
  save_skill_draft: { args: "{ id: string; files: { path: string; content: string }[] }", result: "string" },
  snapshot_skill_draft: { args: "{ id: string; label: string }", result: "string" },
  restore_skill_draft: { args: "{ id: string; versionId: string }", result: "string" },
  publish_skill_draft: { args: "{ id: string; confirmed: boolean; deployments: { agent: string; mode: string; target: string }[] }", result: "string" },
  fix_skill_draft: { args: "{ id: string; fixId: string; path: string | null; message: string | null }", result: "boolean" },
  save_agent_draft: { args: "{ id: string; text: string }", result: "string" },
  export_agent_draft: { args: "{ id: string; text: string }", result: "string" },
  save_generated_draft: { args: "{ goal: string }", result: "string" },
  apply_optimized_description: { args: "{ description: string }", result: "string" },
  record_compare_verdict: { args: "{ id: string; verdict: string }", result: "string" },
  write_history_baseline: { args: "{ skillName: string }", result: "string" },
  set_provider_secret: { args: "{ id: string; secret: string }", result: "string" },
  clear_endpoint_secret: { args: "{ id: string }", result: "boolean" },
  store_backup_pat: { args: "{ token: string }", result: "string" },
  revoke_backup_token: { args: "Record<string, never>", result: "boolean" },
  set_aside_backup: { args: "{ note: string }", result: "string" },
  pull_project_center: { args: "{ name: string; body: string }", result: "string" },
  export_diagnostics: { args: "{ path: string }", result: "string" },
  git_backup_restore_remote: { args: "{ url: string }", result: "string" },
  skip_first_run_restore: { args: "Record<string, never>", result: "boolean" },
  download_and_install_update: { args: "Record<string, never>", result: "string" },
  rebuild_tray_menu: { args: "Record<string, never>", result: "string[]" },
};

export function commandNames(source) {
  const names = [];
  const re = /#\[tauri::command\]\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)/g;
  let match;
  while ((match = re.exec(source))) names.push(match[1]);
  return names;
}

export function render(names) {
  const fields = names.map((name) => {
    const spec = known[name] ?? { args: "Record<string, unknown>", result: "unknown" };
    return `  ${name}: { args: ${spec.args}; result: ${spec.result} };`;
  });
  return `export interface IpcCommands {\n${fields.join("\n")}\n}\n`;
}

function differs(rust, dts) {
  const rendered = render(commandNames(rust));
  return { rendered, match: rendered.trim() === dts.trim() };
}

const selfTest = process.argv.includes("--self-test");
const check = process.argv.includes("--check");

if (selfTest) {
  const rust = "#[tauri::command]\nfn get_settings() {}\n#[tauri::command]\nfn extra_widget() {}\n";
  const dts = "export interface IpcCommands {\n  get_settings: { args: { key: string }; result: string | null };\n}\n";
  const result = differs(rust, dts);
  if (result.match || !result.rendered.includes("extra_widget")) {
    console.error("self-test failed: a command missing from ipc.d.ts must be reported");
    process.exit(1);
  }
  console.log("self-test: deliberate command/type mismatch detected");
}

if (!selfTest || check) {
  const rust = rustPaths.map((file) => fs.readFileSync(file, "utf8")).join("\n");
  const names = commandNames(rust);
  const rendered = render(names);
  if (names.length === 0) {
    console.error(`no #[tauri::command] fns in ${rustPaths.join(", ")}`);
    process.exit(1);
  }
  if (check) {
    const current = fs.readFileSync(dtsPath, "utf8");
    if (current.trim() !== rendered.trim()) {
      console.error(`${dtsPath} is out of date with ${rustPaths.join(", ")}`);
      console.error(rendered);
      process.exit(1);
    }
    console.log(`${dtsPath} matches ${names.join(", ")}`);
  } else if (!selfTest) {
    fs.writeFileSync(dtsPath, rendered);
    console.log(`wrote ${dtsPath}`);
  }
}
