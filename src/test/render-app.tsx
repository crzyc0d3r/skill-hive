import { QueryClient } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { MemoryRouter } from "react-router-dom";
import { App } from "@/App";
import { MARKET_FIXTURE } from "@/features/install/market";
import type { SkillCard } from "@/features/library/model";
import type { AgentRow, PresetRow } from "@/lib/ipc/catalog";

interface IpcState {
  skills: SkillCard[];
  presets: PresetRow[];
  agents: AgentRow[];
  marketStatus: string;
}

function installState(skills: SkillCard[], agents: AgentRow[]): IpcState {
  return {
    skills: skills.map((skill) => ({ ...skill, tags: [...skill.tags], agents: [...skill.agents] })),
    presets: [
      { id: "research", name: "Research", skillIds: ["skill-research", "skill-review"], synced: 0 },
      { id: "tools", name: "Tools", skillIds: ["skill-plain"], synced: 0 },
    ],
    agents: agents.map((agent) => ({ ...agent })),
    marketStatus: "",
  };
}

function handleIpc(state: IpcState, cmd: string, args: Record<string, unknown>, failSettings: boolean) {
  const text = (key: string) => String(args[key] ?? "");
  if (cmd === "get_settings") {
    if (failSettings) throw new Error("database unreadable");
    return text("key") === "theme" ? "light" : null;
  }
  if (cmd === "set_settings" || cmd === "log_startup_event") return null;
  if (cmd === "list_base_entries") return [];
  if (cmd === "check_app_update") return "current";
  if (cmd === "get_managed_skills") return state.skills;
  if (cmd === "list_presets") return state.presets;
  if (cmd === "get_tool_status") return state.agents;
  if (cmd === "scan_local_skills") return [{ name: "cursor-skill", tool: "cursor", path: "~/.cursor/skills/cursor-skill", alreadyManaged: false }];
  if (cmd === "fetch_leaderboard" || cmd === "search_skillssh") {
    return MARKET_FIXTURE.map((skill) => ({ ...skill, source: `${skill.owner}/${skill.name}` }));
  }
  if (cmd === "rename_tag") {
    state.skills = state.skills.map((skill) => ({ ...skill, tags: skill.tags.map((tag) => (tag === text("from") ? text("to") : tag)) }));
    return [];
  }
  if (cmd === "delete_tag") {
    state.skills = state.skills.map((skill) => ({ ...skill, tags: skill.tags.filter((tag) => tag !== text("tag")) }));
    return [];
  }
  if (cmd === "set_skill_tags") {
    const tags = args.tags as string[];
    state.skills = state.skills.map((skill) => (skill.id === text("skillId") ? { ...skill, tags } : skill));
    return null;
  }
  if (cmd === "set_skills_enabled") {
    const ids = args.ids as string[];
    state.skills = state.skills.map((skill) => (ids.includes(skill.id) ? { ...skill, enabled: Boolean(args.enabled) } : skill));
    return null;
  }
  if (cmd === "delete_managed_skills") {
    const ids = args.ids as string[];
    state.skills = state.skills.filter((skill) => !ids.includes(skill.id));
    return null;
  }
  if (cmd === "sync_skill_to_tool") {
    state.skills = state.skills.map((skill) =>
      skill.id === text("skillId") && !skill.agents.includes(text("tool")) ? { ...skill, agents: [...skill.agents, text("tool")] } : skill,
    );
    return null;
  }
  if (cmd === "unsync_skill_from_tool") {
    state.skills = state.skills.map((skill) =>
      skill.id === text("skillId") ? { ...skill, agents: skill.agents.filter((agent) => agent !== text("tool")) } : skill,
    );
    return null;
  }
  if (cmd === "add_skill_to_preset" || cmd === "remove_skill_from_preset") {
    state.presets = state.presets.map((preset) => {
      if (preset.id !== text("presetId")) return preset;
      const skillIds = cmd === "add_skill_to_preset"
        ? [...preset.skillIds, text("skillId")]
        : preset.skillIds.filter((id) => id !== text("skillId"));
      return { ...preset, skillIds };
    });
    return null;
  }
  if (cmd === "reorder_preset_skills") {
    state.presets = state.presets.map((preset) => (preset.id === text("presetId") ? { ...preset, skillIds: args.skillIds as string[] } : preset));
    return null;
  }
  if (cmd === "reorder_presets") {
    const ids = args.ids as string[];
    state.presets = ids.map((id) => state.presets.find((preset) => preset.id === id)).filter((preset): preset is PresetRow => Boolean(preset));
    return null;
  }
  if (cmd === "update_preset") {
    state.presets = state.presets.map((preset) => (preset.id === text("id") ? { ...preset, name: text("name") } : preset));
    return null;
  }
  if (cmd === "delete_preset") {
    state.presets = state.presets.filter((preset) => preset.id !== text("id"));
    return null;
  }
  if (cmd === "create_preset") {
    const id = `preset-${state.presets.length + 1}`;
    state.presets.push({ id, name: text("name"), skillIds: [], synced: 0 });
    return id;
  }
  if (cmd === "apply_preset") return 1;
  if (cmd === "add_custom_tool") {
    const agent = {
      key: text("name").toLowerCase().replace(/\s+/g, "_"),
      name: text("name"),
      installed: true,
      enabled: true,
      path: text("path"),
      projectPath: "",
      custom: true,
    };
    state.agents.push(agent);
    return agent;
  }
  if (cmd === "remove_custom_tool") {
    state.agents = state.agents.filter((agent) => agent.key !== text("key"));
    return null;
  }
  if (cmd === "set_all_tools_enabled") {
    state.agents = state.agents.map((agent) => ({ ...agent, enabled: Boolean(args.enabled) }));
    return null;
  }
  if (cmd === "set_custom_tool_path") {
    state.agents = state.agents.map((agent) => (agent.key === text("key") ? { ...agent, path: text("path") } : agent));
    return null;
  }
  if (cmd === "set_custom_tool_project_path") {
    state.agents = state.agents.map((agent) => (agent.key === text("key") ? { ...agent, projectPath: text("path") } : agent));
    return null;
  }
  if (cmd === "reset_custom_tool_path") return "~/.claude/skills";
  if (cmd === "set_tool_order_cmd") return null;
  if (cmd === "preview_git_install") return { id: "preview-1", skills: [{ relative: "seeded-git", name: "seeded-git" }] };
  if (cmd === "confirm_git_install_skill" || cmd === "install_local_skill" || cmd === "install_archive_skill" || cmd === "import_existing_skill" || cmd === "install_from_skillssh") {
    return { id: "installed", name: text("name") || text("path") || "skill" };
  }
  if (cmd === "begin_market_install") {
    state.marketStatus = `installed:installed:${text("name") || "skill"}`;
    return true;
  }
  if (cmd === "market_install_result") return state.marketStatus || "running";
  if (cmd === "cancel_market_install") {
    state.marketStatus = "error:cancelled";
    return true;
  }
  if (cmd === "import_all_discovered") return ["cursor-skill"];
  if (cmd === "cancel_install") return true;
  if (cmd === "git_backup_sync") return { id: "snap-1", label: "Snapshot 1" };
  if (cmd === "git_backup_restore_version") return `safety-${text("tag")}`;
  if (cmd === "git_backup_status") return { branch: "main", clean: true, remote: null, head: "abc" };
  if (cmd === "git_backup_list_versions") return [];
  if (cmd === "git_backup_set_remote" || cmd === "git_backup_remove_remote" || cmd === "backup_set_device_name") return null;
  if (cmd === "add_project") return "project-1";
  if (cmd === "get_central_repo_path") return "~/.skill-hive";
  if (cmd === "set_central_repo_path") return text("path") || "~/.skill-hive";
  if (cmd === "check_all_skill_updates") return 0;
  if (cmd === "check_skill_update") return "current";
  return null;
}

export const SKILLS: SkillCard[] = [
  {
    id: "skill-research",
    name: "research-notes",
    description: "Collect sources",
    source: "local",
    tags: ["research"],
    enabled: true,
    body: "Local body",
    sourceBody: "Source body",
    agents: [],
  },
  {
    id: "skill-review",
    name: "review-diff",
    description: "Review",
    source: "git",
    tags: ["tools"],
    enabled: true,
    body: "left",
    sourceBody: "right",
    agents: [],
    files: [{ path: "SKILL.md", local: "left", remote: "right" }],
  },
  {
    id: "skill-plain",
    name: "plain",
    description: "Plain",
    source: "scan",
    tags: [],
    enabled: false,
    body: "plain",
    sourceBody: "plain",
    agents: [],
  },
];

export function installIpc(skills = SKILLS, failSettings = false) {
  const state = installState(skills, [
    { key: "claude_code", name: "Claude Code", installed: true, enabled: true, path: "~/.claude/skills", projectPath: ".claude/skills", custom: false },
  ]);
  mockIPC((cmd, args) => handleIpc(state, cmd, (args ?? {}) as Record<string, unknown>, failSettings));
}

export function renderAt(path: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App client={client} />
    </MemoryRouter>,
  );
}

export function renderShell(path: string) {
  const state = installState([], []);
  mockIPC((cmd, args) => handleIpc(state, cmd, (args ?? {}) as Record<string, unknown>, false));
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App client={client} />
    </MemoryRouter>,
  );
}
