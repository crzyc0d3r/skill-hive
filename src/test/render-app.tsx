import { QueryClient } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { MemoryRouter } from "react-router-dom";
import { App } from "@/App";
import type { SkillCard } from "@/features/library/model";

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
  mockIPC((cmd, args) => {
    const record = (args ?? {}) as Record<string, string>;
    if (cmd === "get_settings") {
      if (failSettings) throw new Error("database unreadable");
      return record.key === "theme" ? "light" : null;
    }
    if (cmd === "set_settings") return null;
    if (cmd === "log_startup_event") return null;
    if (cmd === "list_base_entries") return [];
    if (cmd === "get_managed_skills") return skills;
    if (cmd === "list_presets") {
      return [
        { id: "research", name: "Research", skillIds: ["skill-research", "skill-review"], synced: 0 },
        { id: "tools", name: "Tools", skillIds: ["skill-plain"], synced: 0 },
      ];
    }
    if (cmd === "get_tool_status") {
      return [{ key: "claude_code", name: "Claude Code", installed: true, enabled: true, path: "~/.claude/skills", projectPath: ".claude/skills", custom: false }];
    }
    if (cmd === "scan_local_skills") return [{ name: "cursor-skill", tool: "cursor", path: "~/.cursor/skills/cursor-skill" }];
    return null;
  });
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
  mockIPC((cmd, args) => {
    const record = (args ?? {}) as Record<string, string>;
    if (cmd === "get_settings") return record.key === "theme" ? "light" : null;
    if (cmd === "set_settings") return null;
    if (cmd === "list_base_entries") return [];
    if (cmd === "get_managed_skills") return [];
    if (cmd === "list_presets") return [];
    if (cmd === "get_tool_status") return [];
    return null;
  });
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App client={client} />
    </MemoryRouter>,
  );
}
