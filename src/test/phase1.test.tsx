import { QueryClient } from "@tanstack/react-query";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { MemoryRouter } from "react-router-dom";
import { App } from "../App";
import type { SkillCard } from "../lib/phase1/model";

const SKILLS: SkillCard[] = [
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

function installIpc(skills = SKILLS, failSettings = false) {
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
    if (cmd === "list_presets") return [{ id: "research", name: "Research", skillIds: ["skill-research", "skill-review"], synced: 0 }];
    if (cmd === "get_tool_status") {
      return [{ key: "claude_code", name: "Claude Code", installed: true, enabled: true, path: "~/.claude/skills", projectPath: ".claude/skills", custom: false }];
    }
    if (cmd === "scan_local_skills") return [{ name: "cursor-skill", tool: "cursor", path: "~/.cursor/skills/cursor-skill" }];
    return null;
  });
}

function renderAt(path: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App client={client} />
    </MemoryRouter>,
  );
}

describe("phase 1 screens", () => {
  it("shows an empty dashboard and the scan action", async () => {
    installIpc([]);
    renderAt("/");
    expect(await screen.findByText("0 skills")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Scan & Import Local Skills" })).toHaveAttribute("href", "/install?tab=scan");
  });

  it("filters, renames a tag, syncs two cards, and opens the diff", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/my-skills");
    expect(await screen.findByText("research-notes")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "local" }));
    expect(screen.queryByText("review-diff")).not.toBeInTheDocument();
    await act(async () => {
      screen.getByRole("button", { name: "research" }).dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
    });
    const field = await screen.findByLabelText("Rename tag");
    await user.clear(field);
    await user.type(field, "sources");
    await user.click(screen.getByRole("button", { name: "Rename tag" }));
    expect(screen.getByRole("button", { name: "sources" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "local" }));
    await user.click(screen.getByRole("checkbox", { name: "Select research-notes" }));
    await user.click(screen.getByRole("checkbox", { name: "Select review-diff" }));
    await user.click(screen.getByRole("button", { name: "Sync to agents" }));
    await user.click(screen.getByRole("checkbox", { name: "Claude Code" }));
    await user.click(screen.getAllByRole("button", { name: "Sync to agents" })[1]);
    expect(screen.getByTestId("sync-dots-skill-research").querySelector("[data-filled='true']")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: /research-notes/ }));
    await user.click(screen.getByRole("button", { name: "Diff" }));
    expect(screen.getByTestId("diff-view")).toHaveTextContent("Source body");
  });

  it("opens a skill from the command palette", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/");
    await user.keyboard("{Control>}k{/Control}");
    await user.type(screen.getByLabelText("Command search"), "research-notes{Enter}");
    expect(await screen.findByTestId("detail-panel")).toBeInTheDocument();
    expect(screen.getByText("Local body")).toBeInTheDocument();
  });

  it("applies a preset from the global workspace", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/global-workspace");
    const pill = await screen.findByRole("button", { name: "Research" });
    await user.click(pill);
    expect(screen.getByRole("button", { name: "Research check" })).toHaveAttribute("data-fill", "active");
  });

  it("adds a custom agent and shows the repository restart notice", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/settings?tab=agents");
    await user.click(await screen.findByRole("button", { name: "New Agent" }));
    expect(screen.getByText("Field Notes")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.click(screen.getByRole("button", { name: "Save path" }));
    expect(screen.getByText("Restart to apply the new central repository path.")).toBeInTheDocument();
  });

  it("shows the load banner with Retry when settings fail", async () => {
    installIpc([], true);
    const user = userEvent.setup();
    renderAt("/");
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not load settings.");
    installIpc([], false);
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByText("0 skills")).toBeInTheDocument();
  });

  it("renders the library at extra large without truncating names", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/settings");
    await user.click(await screen.findByRole("button", { name: "Extra Large" }));
    expect(document.documentElement.dataset.textSize).toBe("xl");
    renderAt("/my-skills");
    const name = await screen.findByText("research-notes");
    expect(name.className).not.toContain("truncate");
  });
});
