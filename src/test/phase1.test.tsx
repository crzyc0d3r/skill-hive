import { QueryClient } from "@tanstack/react-query";
import { act, fireEvent, render, screen, within } from "@testing-library/react";
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

  it("reorders a preset, uses the card menu, and deletes a tag", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/my-skills");
    expect(await screen.findByText("research-notes")).toBeInTheDocument();
    const members = document.querySelectorAll("[data-member]");
    fireEvent.dragStart(members[1]);
    fireEvent.drop(members[0]);
    expect(screen.getByTestId("preset-order").textContent?.indexOf("review-diff")).toBeLessThan(
      screen.getByTestId("preset-order").textContent?.indexOf("research-notes") ?? 0,
    );
    await user.click(screen.getByRole("checkbox", { name: "Preset membership research-notes" }));
    expect(screen.getByTestId("preset-order")).not.toHaveTextContent("research-notes");
    await user.click(screen.getAllByRole("button", { name: "Card menu" })[0]);
    await user.click(screen.getByRole("button", { name: "Check" }));
    expect(screen.getByText("Checked research-notes")).toBeInTheDocument();
    await act(async () => {
      screen.getByRole("button", { name: "research" }).dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
    });
    await user.click(screen.getByRole("button", { name: "Delete tag" }));
    expect(screen.queryByRole("button", { name: "research" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(screen.queryByText("research-notes")).not.toBeInTheDocument();
  });

  it("runs batch actions, agent toggles, and the per-file diff", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/my-skills");
    await screen.findByText("plain");
    await user.click(screen.getByRole("checkbox", { name: "Select plain" }));
    await user.click(screen.getByRole("button", { name: "Enable" }));
    await user.click(screen.getByRole("button", { name: "Available" }));
    expect(screen.queryByText("plain")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "All" }));
    await user.click(screen.getByRole("checkbox", { name: "Select plain" }));
    await user.click(screen.getByRole("checkbox", { name: "Select research-notes" }));
    await user.click(screen.getByRole("button", { name: "Tags" }));
    await user.type(screen.getByRole("textbox", { name: "Tags" }), "batch");
    await user.click(screen.getAllByRole("button", { name: "Tags" })[1]);
    expect(screen.getByRole("button", { name: "Remove tag batch" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /research-notes/ }));
    await user.click(screen.getByRole("button", { name: "claude_code" }));
    expect(screen.getByRole("button", { name: "claude_code" })).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByRole("button", { name: /review-diff/ }));
    await user.click(screen.getByRole("button", { name: "Diff" }));
    expect(screen.getByTestId("file-diff")).toHaveTextContent("SKILL.md");
    expect(screen.getByTestId("file-diff")).toHaveTextContent("right");
    await user.click(screen.getByRole("checkbox", { name: "Select plain" }));
    await user.click(screen.getByRole("button", { name: "Delete selected" }));
    expect(screen.queryByText("plain")).not.toBeInTheDocument();
  });

  it("deep-links install tabs, installs a folder, and imports one scanned skill", async () => {
    installIpc();
    const user = userEvent.setup();
    const leaderboard = renderAt("/install?tab=leaderboard");
    expect(await screen.findByRole("tab", { name: "Leaderboard" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId("marketplace")).toBeInTheDocument();
    expect(screen.getByLabelText("Search marketplace")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "Local Install" }));
    await user.click(screen.getByRole("button", { name: "Install local" }));
    expect(screen.getByText("Imported demo")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Install archive" }));
    expect(screen.getByText("Imported demo.skill")).toBeInTheDocument();
    leaderboard.unmount();
    renderAt("/install?tab=scan");
    const rename = await screen.findByLabelText("Rename scan cursor-skill");
    await user.clear(rename);
    await user.type(rename, "renamed-cursor");
    await user.click(screen.getByRole("button", { name: "Import cursor-skill" }));
    expect(screen.getByText("Imported renamed-cursor")).toBeInTheDocument();
  });

  it("previews, renames, and cancels a git clone", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/install?tab=git");
    expect(await screen.findByLabelText("Git URL")).toHaveValue("https://github.com/acme/skills/tree/feature/slash/tools/demo");
    await user.click(screen.getByRole("button", { name: "Clone & Preview" }));
    expect(screen.getByTestId("git-preview")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Import Selected" }));
    expect(screen.getByText("Imported renamed-demo")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel clone" }));
    expect(screen.getByText("Temporary clone directory removed.")).toBeInTheDocument();
  });

  it("stores agent order, sync mode, motion, and the update notice", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/settings?tab=agents");
    expect(await screen.findByText("Claude Code")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Disable All" }));
    expect(screen.getByText("Disabled")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Enable All" }));
    expect(screen.getByText("Enabled")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Refresh" }));
    expect(screen.getByText("Agents refreshed")).toBeInTheDocument();
    const path = screen.getByRole("textbox", { name: "Reset path Claude Code" });
    await user.clear(path);
    await user.type(path, "/tmp/claude");
    await user.click(screen.getByRole("button", { name: "Reset path" }));
    expect(path).toHaveValue("~/.claude/skills");
    await user.clear(screen.getByRole("textbox", { name: "Project path Claude Code" }));
    await user.type(screen.getByRole("textbox", { name: "Project path Claude Code" }), "custom/skills");
    expect(screen.getByRole("textbox", { name: "Project path Claude Code" })).toHaveValue("custom/skills");
    await user.click(screen.getByRole("button", { name: "New Agent" }));
    const rows = document.querySelectorAll("[data-agent]");
    fireEvent.dragStart(rows[1]);
    fireEvent.drop(rows[0]);
    expect(document.querySelector("[data-testid='agent-order']")?.textContent?.indexOf("Field Notes")).toBeLessThan(
      document.querySelector("[data-testid='agent-order']")?.textContent?.indexOf("Claude Code") ?? 0,
    );
    await user.click(screen.getByRole("button", { name: "Remove Field Notes" }));
    expect(screen.queryByText("Field Notes")).not.toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.click(screen.getByRole("button", { name: "File Copy" }));
    expect(screen.getByRole("button", { name: "File Copy" })).toHaveAttribute("aria-pressed", "true");
    await user.clear(screen.getByRole("textbox", { name: "Central repository path" }));
    await user.type(screen.getByRole("textbox", { name: "Central repository path" }), "/tmp/hive");
    await user.click(screen.getByRole("button", { name: "Reset repository path" }));
    expect(screen.getByRole("textbox", { name: "Central repository path" })).toHaveValue("~/.skill-hive");
    await user.click(screen.getByRole("tab", { name: "Appearance" }));
    await user.click(screen.getByRole("button", { name: "Reduce motion" }));
    expect(document.documentElement.dataset.reduceMotion).toBe("on");
    await user.click(screen.getByRole("tab", { name: "About" }));
    await user.click(screen.getByRole("button", { name: "Check for Updates" }));
    expect(screen.getByText("You are on the latest version.")).toBeInTheDocument();
  });

  it("opens the agent page and confirms upload, pull, remove, and a refused delete", async () => {
    installIpc([{ ...SKILLS[0], agents: ["claude_code"] }, SKILLS[1], SKILLS[2]]);
    const user = userEvent.setup();
    const overview = renderAt("/global-workspace");
    expect(await within(await screen.findByTestId("agent-catalog")).findByRole("link", { name: "Claude Code" })).toHaveAttribute("href", "/global-workspace/claude_code");
    const tools = document.querySelector("[data-preset='tools']");
    const research = document.querySelector("[data-preset='research']");
    fireEvent.dragStart(tools!);
    fireEvent.drop(research!);
    const order = [...document.querySelectorAll("[data-preset]")].map((node) => node.getAttribute("data-preset"));
    expect(order).toEqual(["tools", "research"]);
    await user.click(screen.getByRole("button", { name: "Edit preset Research" }));
    const editor = screen.getByRole("textbox", { name: "Edit preset" });
    await user.clear(editor);
    await user.type(editor, "Sources");
    await user.click(screen.getByRole("button", { name: "Save preset" }));
    expect(screen.getByRole("link", { name: "Sources" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Delete preset Tools" }));
    expect(screen.queryByRole("link", { name: "Tools" })).not.toBeInTheDocument();
    overview.unmount();

    renderAt("/global-workspace/claude_code");
    expect(await screen.findByTestId("agent-page")).toBeInTheDocument();
    await user.type(screen.getByRole("textbox", { name: "Search agent skills" }), "review");
    expect(screen.queryByText("research-notes")).not.toBeInTheDocument();
    await user.clear(screen.getByRole("textbox", { name: "Search agent skills" }));
    await user.click(screen.getByRole("button", { name: "research" }));
    expect(screen.queryByText("plain")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "research" }));
    await user.click(screen.getByRole("button", { name: "List" }));
    expect(document.querySelector("[data-layout='list']")).toBeTruthy();
    await user.click(screen.getAllByRole("button", { name: "Delete local skill" })[0]);
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.getByText("refusing to delete managed skill")).toBeInTheDocument();
    expect(screen.getByText("research-notes")).toBeInTheDocument();
    await user.click(screen.getAllByRole("button", { name: "Pull" })[1]);
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    await user.click(screen.getAllByRole("button", { name: "Remove from agent" })[2]);
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.queryByText("plain")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Add from Library" }));
    await user.click(within(screen.getByTestId("add-sheet")).getByRole("button", { name: "plain" }));
    expect(screen.getByText("plain")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "review-diff" }));
    await user.click(screen.getByRole("button", { name: "Center" }));
    expect(screen.getByTestId("workspace-detail")).toHaveTextContent("right");
    await user.click(screen.getByRole("checkbox", { name: "Select review-diff" }));
    await user.click(screen.getByRole("button", { name: "Batch remove" }));
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.queryByText("review-diff")).not.toBeInTheDocument();
  });
});
