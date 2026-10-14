import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { AgentRow, PresetRow, ScanRow } from "@/lib/ipc/catalog";
import type { SkillCard } from "@/features/library/model";
import { demoEnabled } from "@/lib/demo";

export interface InstalledSkill {
  id: string;
  name: string;
}

export interface GitPreviewSkill {
  relative: string;
  name: string;
}

export interface GitPreview {
  id: string;
  skills: GitPreviewSkill[];
}

export interface MarketSkill {
  owner: string;
  name: string;
  installs: number;
  source: string;
}

export interface SnapshotRow {
  id: string;
  label: string;
}

export interface BackupStatus {
  branch: string;
  clean: boolean;
  remote: string | null;
  head: string;
}

function messageOf(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) return String(error.message);
  return "Request failed";
}

export async function command<T>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>(name, args);
  } catch (error) {
    const message = messageOf(error);
    toast.error(message);
    throw new Error(message);
  }
}

export const listManagedSkills = () => command<SkillCard[]>("get_managed_skills");
export const installLocalSkill = (path: string) => command<InstalledSkill>("install_local_skill", { path });
export const installArchiveSkill = (path: string) => command<InstalledSkill>("install_archive_skill", { path });
export const importExistingSkill = (path: string, rename?: string) =>
  command<InstalledSkill>("import_existing_skill", { path, rename: rename ?? null });
export const importAllDiscovered = () => command<string[]>("import_all_discovered");
export const previewGitInstall = (url: string) => command<GitPreview>("preview_git_install", { url });
export const confirmGitInstall = (previewId: string, relative: string, rename?: string) =>
  command<string[]>("confirm_git_install_skill", { previewId, relative, rename: rename ?? null });
export const cancelInstall = (previewId?: string) => command<boolean>("cancel_install", { previewId: previewId ?? null });
export const deleteManagedSkills = (ids: string[]) => command<null>("delete_managed_skills", { ids });
export const setSkillsEnabled = (ids: string[], enabled: boolean) => command<null>("set_skills_enabled", { ids, enabled });
export const setSkillTags = (skillId: string, tags: string[]) => command<null>("set_skill_tags", { skillId, tags });
export const renameTag = (from: string, to: string) => command<string[]>("rename_tag", { from, to });
export const deleteTag = (tag: string) => command<string[]>("delete_tag", { tag });
export const syncSkillToTool = (skillId: string, tool: string) => command<null>("sync_skill_to_tool", { skillId, tool });
export const unsyncSkillFromTool = (skillId: string, tool: string) => command<null>("unsync_skill_from_tool", { skillId, tool });
export const addSkillToPreset = (presetId: string, skillId: string) => command<null>("add_skill_to_preset", { presetId, skillId });
export const removeSkillFromPreset = (presetId: string, skillId: string) =>
  command<null>("remove_skill_from_preset", { presetId, skillId });
export const reorderPresetSkills = (presetId: string, skillIds: string[]) =>
  command<null>("reorder_preset_skills", { presetId, skillIds });
export const createPreset = (name: string) => command<string>("create_preset", { name });
export const updatePreset = (id: string, name: string) => command<null>("update_preset", { id, name });
export const deletePreset = (id: string) => command<null>("delete_preset", { id });
export const reorderPresets = (ids: string[]) => command<null>("reorder_presets", { ids });
export const applyPreset = (presetId: string, tool?: string) => command<number>("apply_preset", { presetId, tool: tool ?? null });
export const addCustomTool = (name: string, path: string, projectPath?: string) =>
  command<AgentRow>("add_custom_tool", { name, path, projectPath: projectPath ?? null });
export const removeCustomTool = (key: string) => command<null>("remove_custom_tool", { key });
export const setToolEnabled = (key: string, enabled: boolean) => command<null>("set_tool_enabled", { key, enabled });
export const setAllToolsEnabled = (enabled: boolean) => command<null>("set_all_tools_enabled", { enabled });
export const setToolOrder = (keys: string[]) => command<null>("set_tool_order_cmd", { keys });
export const setCustomToolPath = (key: string, path: string) => command<null>("set_custom_tool_path", { key, path });
export const resetCustomToolPath = (key: string) => command<string>("reset_custom_tool_path", { key });
export const setCustomToolProjectPath = (key: string, path: string) =>
  command<null>("set_custom_tool_project_path", { key, path });
export const getCentralRepoPath = () => command<string>("get_central_repo_path");
export const setCentralRepoPath = (path: string) => command<string>("set_central_repo_path", { path });
export const checkAllSkillUpdates = () => command<number>("check_all_skill_updates");
export const checkSkillUpdate = (skillId: string) => command<string>("check_skill_update", { skillId });
export const gitBackupStatus = () => command<BackupStatus>("git_backup_status");
export const gitBackupSync = (message?: string) => command<SnapshotRow>("git_backup_sync", { message: message ?? null });
export const gitBackupRestore = (tag: string) => command<string>("git_backup_restore_version", { tag });
export const gitBackupSetRemote = (url: string) => command<null>("git_backup_set_remote", { url });
export const gitBackupRemoveRemote = () => command<null>("git_backup_remove_remote");
export const backupSetDeviceName = (name: string) => command<null>("backup_set_device_name", { name });
export const addProject = (name: string, path: string) => command<string>("add_project", { name, path });
export const fetchLeaderboard = (board?: string) => command<MarketSkill[]>("fetch_leaderboard", { board: board ?? null });
export const searchSkillsSh = (query: string) => command<MarketSkill[]>("search_skillssh", { query, limit: 24 });
export const installFromSkillsSh = (source: string, name: string) =>
  command<InstalledSkill>("install_from_skillssh", { source, name });
export function dispatch<T = unknown>(name: string, args: Record<string, unknown> = {}): Promise<T | undefined> {
  if (demoEnabled()) return Promise.resolve(undefined);
  return command<T>(name, args);
}

export const cancelMarketInstall = () => dispatch("cancel_market_install");
export const saveSkillDraft = (id: string, files: { path: string; content: string }[]) =>
  dispatch("save_skill_draft", { id, files });
export const snapshotSkillDraft = (id: string, label: string) => dispatch("snapshot_skill_draft", { id, label });
export const restoreSkillDraft = (id: string, versionId: string) => dispatch("restore_skill_draft", { id, versionId });
export const publishSkillDraft = (
  id: string,
  confirmed: boolean,
  deployments: { agent: string; mode: string; target: string }[],
) => dispatch("publish_skill_draft", { id, confirmed, deployments });
export const fixSkillDraft = (id: string, fixId: string, path: string, message: string) =>
  dispatch("fix_skill_draft", { id, fixId, path, message });
export const saveAgentDraft = (id: string, text: string) => dispatch("save_agent_draft", { id, text });
export const exportAgentDraft = (id: string, text: string) => dispatch("export_agent_draft", { id, text });
export const saveGeneratedDraft = (goal: string) => dispatch("save_generated_draft", { goal });
export const applyOptimizedDescription = (description: string) =>
  dispatch("apply_optimized_description", { description });
export const recordCompareVerdict = (id: string, verdict: string) =>
  dispatch("record_compare_verdict", { id, verdict });
export const writeHistoryBaseline = (skillName: string) => dispatch("write_history_baseline", { skillName });
export const setProviderSecret = (id: string, secret: string) => dispatch("set_provider_secret", { id, secret });
export const storeBackupPat = (token: string) => dispatch("store_backup_pat", { token });
export const revokeBackupToken = () => dispatch("revoke_backup_token");
export const setAsideBackup = (note: string) => dispatch("set_aside_backup", { note });
export const pullProjectCenter = (name: string, body: string) => dispatch("pull_project_center", { name, body });
export const exportDiagnostics = (path: string) => dispatch("export_diagnostics", { path });
export const restoreBackupRemote = (url: string) => dispatch("git_backup_restore_remote", { url });
export const skipFirstRunRestore = () => dispatch("skip_first_run_restore");
export const downloadAndInstallUpdate = () => dispatch("download_and_install_update");
export const rebuildTrayMenu = () => dispatch("rebuild_tray_menu");
export const listPresetRows = () => command<PresetRow[]>("list_presets");
export const listAgentRows = () => command<AgentRow[]>("get_tool_status");
export const listScanRows = () => command<ScanRow[]>("scan_local_skills");
