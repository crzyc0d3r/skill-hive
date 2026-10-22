import { act, cleanup, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { resetAuthoring } from "@/lib/authoring/store";
import { resetGenerate } from "@/features/generate/store";
import { setProjectSkills } from "@/features/project/store";
import { setLocale } from "@/lib/i18n";
import {
  installIpc,
  ipcLog,
  renderAt,
  seedSetting,
  setBackupRemote,
  setComparePayload,
  setGeneratedPayload,
  setLogExcerpt,
  setOptimizePayload,
  setProjectRows,
  setProjectSkillRows,
  setUpdateCount,
  SKILLS,
} from "@/test/render-app";

function calls(cmd: string) {
  return ipcLog.filter((entry) => entry.cmd === cmd);
}

describe("ui parity invokes", () => {
  afterEach(() => {
    cleanup();
    resetGenerate();
    resetAuthoring();
    setProjectSkills([]);
    setLocale("en");
    localStorage.clear();
    vi.useRealTimers();
  });

  it("switches the GitHub form from the saved auth method and persists backup settings", async () => {
    const user = userEvent.setup();
    installIpc();
    seedSetting("github_auth_method", "device");
    setBackupRemote("https://github.com/acme/hive-backup");
    const device = renderAt("/backup");
    expect(await screen.findByRole("button", { name: "Start device login" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Personal access token")).not.toBeInTheDocument();
    expect(screen.queryByText(/OAuth client id/)).not.toBeInTheDocument();
    expect(screen.getByTestId("backup-status")).toHaveTextContent("main");
    expect(screen.getByTestId("backup-status")).toHaveTextContent("Clean");
    expect(screen.getByRole("link", { name: "Open remote deletion page" })).toHaveAttribute("href", "https://github.com/acme/hive-backup");
    device.unmount();

    installIpc();
    seedSetting("github_auth_method", "pat");
    renderAt("/backup");
    expect(await screen.findByLabelText("Personal access token")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Start device login" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("checkbox", { name: "Automatic backup" }));
    await user.click(screen.getByRole("checkbox", { name: "Presets" }));
    expect(calls("set_settings").some((entry) => entry.args.key === "backup_auto" && entry.args.value === "1")).toBe(true);
    expect(calls("set_settings").some((entry) => entry.args.key === "backup_scope" && String(entry.args.value).includes('"presets":false'))).toBe(true);
    await user.click(screen.getByRole("button", { name: "Back Up Now" }));
    expect(await screen.findByText("Backing up")).toBeInTheDocument();
  });

  it("syncs the clicked agent and the viewed preset, then updates the selection", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/my-skills");
    expect(await screen.findByText("research-notes")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /^research-notes/ }));
    await user.click(screen.getByRole("button", { name: "Add to agent" }));
    await user.click(screen.getByRole("button", { name: "Claude Code" }));
    expect(calls("sync_skill_to_tool").some((entry) => entry.args.tool === "claude_code" && entry.args.skillId === "skill-research")).toBe(true);

    await user.click(screen.getByRole("link", { name: /Tools/ }));
    expect(calls("switch_preset").some((entry) => entry.args.presetId === "tools")).toBe(true);
    await user.click(screen.getByRole("link", { name: "Library" }));
    await user.click(await screen.findByRole("checkbox", { name: "Preset membership research-notes" }));
    expect(calls("add_skill_to_preset").some((entry) => entry.args.presetId === "tools" && entry.args.skillId === "skill-research")).toBe(true);

    await user.click(screen.getByRole("button", { name: /review-diff/ }));
    expect(screen.getByTestId("origin-meta")).toHaveTextContent("Source type");
    expect(screen.getByTestId("origin-meta")).toHaveTextContent("/src/research");
    await user.click(screen.getAllByRole("button", { name: "Card menu" })[1]);
    const menu = document.querySelector("[role='menu']");
    expect(menu).toBeTruthy();
    await user.click(within(menu as HTMLElement).getByRole("button", { name: "Update" }));
    expect(calls("check_skill_update").some((entry) => entry.args.skillId === "skill-review")).toBe(true);
    expect(calls("update_skill").some((entry) => entry.args.skillId === "skill-review")).toBe(true);

    await user.click(screen.getByRole("checkbox", { name: "Select research-notes" }));
    await user.click(screen.getByRole("checkbox", { name: "Select review-diff" }));
    await user.click(screen.getByRole("button", { name: "Update selected" }));
    const batch = calls("batch_update_skills").at(-1);
    expect(batch?.args.ids).toEqual(expect.arrayContaining(["skill-research", "skill-review"]));
    expect(calls("apply_available_skill_updates").length).toBeGreaterThan(0);
  });

  it("debounces marketplace search, rescans, and imports every selected git skill", async () => {
    vi.useFakeTimers();
    installIpc();
    renderAt("/install?tab=leaderboard");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20);
    });
    fireEvent.change(screen.getByLabelText("Search marketplace"), { target: { value: "review" } });
    expect(calls("search_skillssh")).toHaveLength(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
    expect(calls("search_skillssh").some((entry) => entry.args.query === "review")).toBe(true);
    vi.useRealTimers();

    const user = userEvent.setup();
    cleanup();
    installIpc();
    renderAt("/install?tab=scan");
    const before = calls("scan_local_skills").length;
    await user.click(await screen.findByRole("button", { name: "Rescan" }));
    expect(calls("scan_local_skills").length).toBeGreaterThan(before);

    cleanup();
    installIpc();
    renderAt("/install?tab=git");
    await user.type(await screen.findByLabelText("Git URL"), "https://github.com/acme/multi");
    await user.click(screen.getByRole("button", { name: "Clone & Preview" }));
    expect(await screen.findByRole("checkbox", { name: "Select alpha" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select beta" })).toBeChecked();
    await user.click(screen.getByRole("button", { name: "Import Selected" }));
    const relatives = calls("confirm_git_install_skill").map((entry) => entry.args.relative);
    expect(relatives).toEqual(expect.arrayContaining(["alpha", "beta"]));
  });

  it("computes preset coverage from agent membership and splits pull from upload", async () => {
    installIpc([{ ...SKILLS[0], agents: ["claude_code"] }, SKILLS[1], SKILLS[2]]);
    const user = userEvent.setup();
    const overview = renderAt("/global-workspace");
    expect(await screen.findByRole("button", { name: "Research" })).toHaveAttribute("data-fill", "partial");
    expect(screen.getByText("1 of 2")).toBeInTheDocument();
    overview.unmount();

    installIpc([{ ...SKILLS[0], agents: ["claude_code"] }, SKILLS[1], SKILLS[2]]);
    renderAt("/global-workspace/claude_code");
    await user.click(await screen.findByRole("button", { name: "More research-notes" }));
    await user.click(screen.getByRole("menuitem", { name: "Upload" }));
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(calls("sync_skill_to_tool").some((entry) => entry.args.tool === "claude_code")).toBe(true);
    expect(calls("import_global_local_skill_to_center")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "More research-notes" }));
    await user.click(screen.getByRole("menuitem", { name: "Pull" }));
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(calls("import_global_local_skill_to_center").some((entry) => entry.args.tool === "claude_code")).toBe(true);

    await user.click(screen.getByRole("button", { name: "Add from Library" }));
    await user.type(screen.getByRole("textbox", { name: "Search library" }), "plain");
    const sheet = screen.getByTestId("add-sheet");
    expect(within(sheet).queryByRole("button", { name: "research-notes" })).not.toBeInTheDocument();
    expect(within(sheet).getByRole("button", { name: "plain" })).toBeInTheDocument();
  });

  it("renders lobster routes with the workspace and keeps the lobster landmark", async () => {
    installIpc();
    localStorage.setItem("skill-hive-home", ".openclaw");
    renderAt("/lobster-workspace/openclaw");
    expect(await screen.findByTestId("lobster")).toBeInTheDocument();
    expect(screen.getByTestId("agent-page")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "OpenClaw" })).toBeInTheDocument();
  });

  it("links a skills root, updates and deletes a project skill, and remembers agents", async () => {
    installIpc();
    setProjectSkillRows([{ name: "billing", path: "/tmp/billing", enabled: true, agent: "claude_code" }]);
    const user = userEvent.setup();
    renderAt("/project/field");
    expect((await screen.findAllByText("billing")).length).toBeGreaterThan(0);
    expect(screen.queryByText(/^notes$/)).not.toBeInTheDocument();
    await user.type(screen.getByLabelText("Skills root"), "/tmp/skills-root");
    await user.click(screen.getByTestId("link-project"));
    expect(calls("add_project").some((entry) => entry.args.path === "/tmp/skills-root")).toBe(true);
    expect(calls("add_linked_workspace").some((entry) => entry.args.path === "/tmp/skills-root")).toBe(true);
    expect(calls("reveal_project_folder")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "More billing" }));
    await user.click(screen.getByRole("menuitem", { name: "Update project" }));
    expect(calls("update_project_skill_from_center").some((entry) => entry.args.id === "field" && entry.args.skillId === "billing")).toBe(true);
    await user.click(screen.getByRole("button", { name: "More billing" }));
    await user.click(screen.getByRole("menuitem", { name: "Update center" }));
    expect(calls("update_project_skill_to_center").some((entry) => entry.args.id === "field" && entry.args.name === "billing")).toBe(true);
    await user.selectOptions(screen.getByRole("combobox", { name: "Agents" }), "codex");
    expect(calls("set_settings").some((entry) => entry.args.key === "project_last_agents" && String(entry.args.value).includes("codex"))).toBe(true);
    await user.click(screen.getByRole("button", { name: "More billing" }));
    await user.click(screen.getByRole("menuitem", { name: "Delete local skill" }));
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(calls("delete_project_skill").some((entry) => entry.args.name === "billing")).toBe(true);

    cleanup();
    installIpc();
    renderAt("/project/field");
    await user.click(await screen.findByRole("button", { name: "Add from library" }));
    await user.click(screen.getByRole("button", { name: "Select all" }));
    await user.click(screen.getAllByRole("button", { name: "Add from library" })[1]);
    const exported = calls("export_skill_to_project").map((entry) => entry.args.skillId);
    expect(new Set(exported).size).toBeGreaterThan(1);
  });

  it("pulls the notes fixture after linking when the library already has skills", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/project/fixture");
    expect((await screen.findAllByText("research-notes")).length).toBeGreaterThan(0);
    await user.click(screen.getByTestId("link-project"));
    expect(screen.queryByText("research-notes")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "More notes" }));
    await user.click(screen.getByRole("menuitem", { name: "Pull center" }));
    expect(calls("pull_project_center")).toEqual([
      expect.objectContaining({ args: expect.objectContaining({ name: "notes", body: "center v2" }) }),
    ]);
  });

  it("persists agent enablement, text size, language, the version line, and the issue link", async () => {
    installIpc();
    seedSetting("ui_text_size", "large");
    seedSetting("ui_language", "zh-CN");
    const loaded = renderAt("/settings?tab=appearance");
    await waitFor(() => expect(document.documentElement.dataset.textSize).toBe("large"));
    await waitFor(() => expect(document.documentElement.lang).toBe("en"));
    loaded.unmount();
    setLocale("en");

    installIpc();
    const user = userEvent.setup();
    renderAt("/settings?tab=agents");
    await user.click(await screen.findByRole("switch", { name: "Enable Claude Code" }));
    expect(calls("set_tool_enabled").some((entry) => entry.args.key === "claude_code" && entry.args.enabled === false)).toBe(true);
    await user.click(screen.getByRole("tab", { name: "Appearance" }));
    await user.click(screen.getByRole("button", { name: "Small" }));
    expect(calls("set_settings").some((entry) => entry.args.key === "ui_text_size" && entry.args.value === "small")).toBe(true);
    expect(screen.queryByRole("button", { name: "Simplified Chinese" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Traditional Chinese" })).not.toBeInTheDocument();
    setLocale("en");
    await user.click(screen.getByRole("tab", { name: "About" }));
    expect(await screen.findByText(/Version 1\.0\.0 current/)).toBeInTheDocument();
    await waitFor(() => expect(decodeURIComponent(screen.getByRole("link", { name: "Report Issue" }).getAttribute("href") ?? "")).toContain("/tmp/hive-diag"));
  });

  it("moves the palette selection and lists linked projects with a delete", async () => {
    installIpc();
    setUpdateCount(3);
    setProjectRows([{ id: "field", name: "Field Notes", path: "/tmp/field" }]);
    const user = userEvent.setup();
    renderAt("/");
    expect(await screen.findByTestId("update-dot")).toBeInTheDocument();
    await user.keyboard("{Control>}k{/Control}");
    expect(screen.getByText("Projects")).toBeInTheDocument();
    expect(screen.getAllByText("Field Notes").length).toBeGreaterThan(0);
    const first = screen.getByTestId("palette-active").textContent;
    await user.keyboard("{ArrowDown}");
    expect(screen.getByTestId("palette-active").textContent).not.toBe(first);
    await user.keyboard("{Escape}");
    await user.click(screen.getByRole("button", { name: "Remove project Field Notes" }));
    expect(calls("remove_project").some((entry) => entry.args.id === "field")).toBe(true);
    expect(screen.queryByRole("link", { name: "Field Notes" })).not.toBeInTheDocument();
  });

  it("stores agent order from the settings list", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/settings?tab=agents");
    await user.click(await screen.findByRole("button", { name: "Add agent" }));
    await user.type(screen.getByRole("textbox", { name: "Agent name" }), "Field Notes");
    await user.type(screen.getByRole("textbox", { name: "Agent path" }), "~/.field-notes/skills");
    await user.click(screen.getByRole("button", { name: "New Agent" }));
    const rows = document.querySelectorAll("[data-agent]");
    fireEvent.dragStart(rows[1]);
    fireEvent.drop(rows[0]);
    expect(calls("set_settings").some((entry) => entry.args.key === "agent_order" && String(entry.args.value).includes("field_notes"))).toBe(true);
  });

  it("loads live analytics from command payloads and keeps demo samples for the wizard", async () => {
    installIpc();
    setLogExcerpt({ path: "/tmp/run.json", text: JSON.stringify({ cells: [{ id: "c1", label: "alpha-cell", passRate: 1 }], steps: [{ id: 2, tool: "Write", useTs: 1, resultTs: 2 }] }) });
    const liveResults = renderAt("/runs/live-run");
    expect(await screen.findByText(/alpha-cell/)).toBeInTheDocument();
    expect(screen.queryByText(/without 1/)).not.toBeInTheDocument();
    liveResults.unmount();

    installIpc();
    renderAt("/runs/empty-run");
    expect(await screen.findByText("No measured results yet.")).toBeInTheDocument();
    expect(screen.queryByText(/without 1/)).not.toBeInTheDocument();

    cleanup();
    installIpc();
    setComparePayload("alpha beats beta");
    const user = userEvent.setup();
    renderAt("/compare/live");
    expect(screen.queryByText("Findings list the seeded vulnerability.")).not.toBeInTheDocument();
    expect(await screen.findByText("No comparison yet.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Record verdict" }));
    expect(await screen.findByTestId("compare-live")).toHaveTextContent("alpha beats beta");
    expect(calls("record_compare_verdict").some((entry) => entry.args.verdict === "Output A")).toBe(true);

    cleanup();
    installIpc();
    setGeneratedPayload(JSON.stringify(["Which files should this skill read?"]));
    renderAt("/generate/live");
    await user.type(await screen.findByLabelText("Goal"), "Extract invoices");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByText("No interview questions yet.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Accept all defaults" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(await screen.findByText("Which files should this skill read?")).toBeInTheDocument();
    expect(screen.queryByText("Problems: 0")).not.toBeInTheDocument();
    expect(calls("save_generated_draft").some((entry) => entry.args.goal === "Extract invoices")).toBe(true);

    cleanup();
    installIpc();
    setOptimizePayload("/tmp/suite/evals/evals.json");
    renderAt("/skills/skill-research/tests");
    await user.click(await screen.findByRole("button", { name: "Optimize description" }));
    await user.click(screen.getByRole("button", { name: "Apply" }));
    expect(await screen.findByTestId("optimize-preview")).toHaveTextContent("/tmp/suite/evals/evals.json");
    expect(calls("apply_optimized_description").some((entry) => entry.args.description === "Extract invoices" || typeof entry.args.description === "string")).toBe(true);
  });

  it("deploys through publish when validation is clean and shows agent fields", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/skills/warnings/edit");
    expect(await screen.findByRole("button", { name: "Package" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Deploy" })).toBeDisabled();
    cleanup();
    resetAuthoring();
    renderAt("/skills/copy-skill/edit");
    const deploy = await screen.findByRole("button", { name: "Deploy" });
    expect(deploy).toBeEnabled();
    await user.click(deploy);
    expect(calls("publish_skill_draft").some((entry) => entry.args.id === "copy-skill")).toBe(true);
    await user.click(screen.getByRole("button", { name: "Package" }));
    expect(calls("publish_skill_draft").length).toBeGreaterThan(1);
    cleanup();
    resetAuthoring();
    renderAt("/agents/reviewer/edit");
    expect(await screen.findByTestId("description-length")).toHaveTextContent("13");
    expect(screen.getByTestId("agent-fields")).toHaveTextContent("disallowedTools");
    expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();
  });
});
