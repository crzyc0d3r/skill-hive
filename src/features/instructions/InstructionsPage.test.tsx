import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "@/test/render";
import { api } from "@/lib/api/client";
import { InstructionsPage } from "./InstructionsPage";

const HOME = "/Users/you";
const PROJECT = `${HOME}/dev/acme-api`;
const USER_CLAUDE_MD = `${HOME}/.claude/CLAUDE.md`;

const rail = () => screen.getByRole("complementary", { name: "Instruction files" });
/** The rail entry for `display`: its first line carries the path as a title, the button around it the label, status and note. */
const entry = (display: string) => within(rail()).getByTitle(display).closest("button") as HTMLElement;
const findEntry = (display: string) => within(rail()).findByTitle(display, {}, { timeout: 4000 });
const editor = (display: string) => screen.findByRole("textbox", { name: `Contents of ${display}` });
const openProjectScope = async (user: ReturnType<typeof userEvent.setup>) => {
  await user.click(screen.getByRole("tab", { name: "Project" }));
  await findEntry("CLAUDE.md");
};

describe("InstructionsPage", () => {
  it("lists the user-scope files grouped by agent with their status", async () => {
    renderWithProviders(<InstructionsPage />, { route: "/instructions" });
    await findEntry("~/.claude/CLAUDE.md");
    // Group headings name the agents that read the files under them.
    expect(within(rail()).getByText("Claude Code")).toBeInTheDocument();
    expect(within(rail()).getByText("Codex CLI")).toBeInTheDocument();
    const claude = entry("~/.claude/CLAUDE.md");
    expect(within(claude).getByText("User memory")).toBeInTheDocument();
    expect(within(claude).getByText(/^\d+ lines · ~\d+ tokens$/)).toBeInTheDocument();
    expect(within(claude).queryByText("Missing")).not.toBeInTheDocument();
    const codex = entry("~/.codex/AGENTS.md");
    expect(within(codex).getByText("Global instructions")).toBeInTheDocument();
    expect(within(codex).getByText(/^\d+ lines · ~\d+ tokens$/)).toBeInTheDocument();
    // Nothing is selected yet.
    expect(screen.getByText(/Pick a file on the left/)).toBeInTheDocument();

    // Every user-scope location of the installed agents is on disk; the missing ones are project files.
    const user = userEvent.setup();
    await openProjectScope(user);
    expect(within(entry(".cursorrules")).getByText("Missing")).toBeInTheDocument();
    expect(within(entry("CLAUDE.local.md")).getByText("Missing")).toBeInTheDocument();
    expect(within(entry("CLAUDE.md")).getByText(/^\d+ lines · ~\d+ tokens$/)).toBeInTheDocument();
  });

  it("edits and saves the user CLAUDE.md, keeping backups that can be loaded back into the draft", async () => {
    const user = userEvent.setup();
    const original = (await api.readInstruction(USER_CLAUDE_MD)).text;
    renderWithProviders(<InstructionsPage />, { route: "/instructions" });
    await findEntry("~/.claude/CLAUDE.md");
    await user.click(entry("~/.claude/CLAUDE.md"));
    const textarea = await editor("~/.claude/CLAUDE.md");
    expect(textarea).toHaveValue(original);
    expect(original).toContain("# Preferences");
    const save = screen.getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Revert" })).not.toBeInTheDocument();

    // The preview renders the markdown.
    await user.click(screen.getByRole("tab", { name: "Preview" }));
    expect(await screen.findByRole("heading", { name: "Preferences" })).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "Edit" }));

    // Save with the button.
    await user.type(await editor("~/.claude/CLAUDE.md"), "- Keep answers short.");
    expect(save).toBeEnabled();
    expect(screen.getByRole("button", { name: "Revert" })).toBeInTheDocument();
    await user.click(save);
    await waitFor(() => expect(save).toBeDisabled(), { timeout: 4000 });
    const afterButton = (await api.readInstruction(USER_CLAUDE_MD)).text;
    expect(afterButton).toBe(`${original}- Keep answers short.`);
    let backups = await api.instructionBackups(USER_CLAUDE_MD);
    expect(backups).toHaveLength(1);
    expect(backups[0].sizeBytes).toBe(original.length);

    // Save with Ctrl/Cmd+S from inside the editor.
    await user.type(await editor("~/.claude/CLAUDE.md"), " Lead with the answer.");
    await waitFor(() => expect(save).toBeEnabled());
    await user.keyboard("{Control>}s{/Control}");
    await waitFor(() => expect(save).toBeDisabled(), { timeout: 4000 });
    expect((await api.readInstruction(USER_CLAUDE_MD)).text).toBe(`${afterButton} Lead with the answer.`);
    backups = await api.instructionBackups(USER_CLAUDE_MD);
    expect(backups).toHaveLength(2);

    // The Backups menu lists them newest first; loading the oldest puts the original text back into the draft.
    await user.click(screen.getByRole("button", { name: "Backups" }));
    const menu = await screen.findByRole("menu");
    // "Loading…" and "No backups yet" are disabled items; the versions are the enabled ones.
    const versions = () => within(menu).getAllByRole("menuitem").filter((i) => i.getAttribute("aria-disabled") !== "true");
    await waitFor(() => expect(versions()).toHaveLength(2), { timeout: 4000 });
    expect(within(menu).queryByText("No backups yet")).not.toBeInTheDocument();
    const enabled = versions();
    await user.click(enabled[enabled.length - 1]);
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Contents of ~/.claude/CLAUDE.md" })).toHaveValue(original), { timeout: 4000 });
    expect(save).toBeEnabled();
    // Not written until saved.
    expect((await api.readInstruction(USER_CLAUDE_MD)).text).toBe(`${afterButton} Lead with the answer.`);
  });

  it("shows a starter for a missing file and creates it in place", async () => {
    const user = userEvent.setup();
    renderWithProviders(<InstructionsPage />, { route: "/instructions" });
    await findEntry("~/.claude/CLAUDE.md");
    await openProjectScope(user);
    expect(within(entry("CLAUDE.local.md")).getByText("Missing")).toBeInTheDocument();
    await user.click(entry("CLAUDE.local.md"));
    const textarea = await editor("CLAUDE.local.md");
    expect(textarea).toHaveValue("# Project notes\n\n## Commands\n\n## Conventions\n");
    expect(screen.getByText("Not on disk yet; Create writes this draft.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Save" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Delete" })).not.toBeInTheDocument();
    const create = screen.getByRole("button", { name: "Create" });
    expect(create).toBeEnabled();
    await user.click(create);
    await waitFor(() => expect(within(entry("CLAUDE.local.md")).queryByText("Missing")).not.toBeInTheDocument(), { timeout: 4000 });
    expect(within(entry("CLAUDE.local.md")).getByText(/^\d+ lines · ~\d+ tokens$/)).toBeInTheDocument();
    // The editor now treats it as an existing file.
    expect(await screen.findByRole("button", { name: "Save" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Delete" })).toBeInTheDocument();
    expect((await api.readInstruction(`${PROJECT}/CLAUDE.local.md`, PROJECT)).file.exists).toBe(true);
  });

  it("lists project files once per path with their readers and adds rules to a folder with the right suffix", async () => {
    const user = userEvent.setup();
    renderWithProviders(<InstructionsPage />, { route: "/instructions" });
    await findEntry("~/.claude/CLAUDE.md");
    await openProjectScope(user);

    // AGENTS.md is one entry, grouped under every agent that reads it (installed or not, since
    // the file exists).
    expect(within(rail()).getAllByTitle("AGENTS.md")).toHaveLength(1);
    expect(within(rail()).getByText("Codex CLI · Cursor · GitHub Copilot · OpenCode · Shared (.agents)")).toBeInTheDocument();
    expect(within(entry("AGENTS.md")).getByText("The AGENTS.md convention: one file read by several agents.")).toBeInTheDocument();

    // The rules folder and its two members.
    const folder = entry(".cursor/rules");
    expect(within(folder).getByText("Rules")).toBeInTheDocument();
    expect(within(folder).getByText("Folder")).toBeInTheDocument();
    expect(entry(".cursor/rules/style.mdc")).toBeInTheDocument();
    expect(entry(".cursor/rules/testing.mdc")).toBeInTheDocument();

    // A name without the suffix is refused inline.
    await user.click(screen.getByRole("button", { name: "New rule in .cursor/rules" }));
    const name = screen.getByRole("textbox", { name: "New rule file name in .cursor/rules" });
    await user.type(name, "security");
    expect(screen.getByText("The name must end with .mdc")).toBeInTheDocument();
    expect(name).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByRole("button", { name: "Add" })).toBeDisabled();
    await user.keyboard("{Enter}");
    expect(screen.queryByRole("textbox", { name: "Contents of .cursor/rules/security" })).not.toBeInTheDocument();

    // With the suffix it opens the editor on the new path; Create writes it.
    await user.type(name, ".mdc");
    expect(screen.queryByText("The name must end with .mdc")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Add" }));
    const textarea = await editor(".cursor/rules/security.mdc");
    expect(textarea).toHaveValue("---\ndescription: \nalwaysApply: false\n---\n\n");
    expect(within(entry(".cursor/rules/security.mdc")).getByText("Missing")).toBeInTheDocument();
    await user.type(textarea, "- Never log secrets.");
    await user.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(within(entry(".cursor/rules/security.mdc")).queryByText("Missing")).not.toBeInTheDocument(), { timeout: 4000 });
    const written = await api.readInstruction(`${PROJECT}/.cursor/rules/security.mdc`, PROJECT);
    expect(written.file.parent).toBe(`${PROJECT}/.cursor/rules`);
    expect(written.text).toBe("---\ndescription: \nalwaysApply: false\n---\n\n- Never log secrets.");
  });

  it("deletes an existing rule file behind a confirmation and keeps a backup", async () => {
    const user = userEvent.setup();
    const path = `${PROJECT}/.cursor/rules/testing.mdc`;
    const before = (await api.readInstruction(path, PROJECT)).text;
    renderWithProviders(<InstructionsPage />, { route: "/instructions" });
    await findEntry("~/.claude/CLAUDE.md");
    await openProjectScope(user);
    await user.click(entry(".cursor/rules/testing.mdc"));
    await editor(".cursor/rules/testing.mdc");
    await user.click(screen.getByRole("button", { name: "Delete" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Delete .cursor/rules/testing.mdc?")).toBeInTheDocument();
    expect(within(dialog).getByText("A backup is kept.")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(within(rail()).queryByTitle(".cursor/rules/testing.mdc")).not.toBeInTheDocument(), { timeout: 4000 });
    expect(entry(".cursor/rules/style.mdc")).toBeInTheDocument();
    expect((await api.readInstruction(path, PROJECT)).file.exists).toBe(false);
    const backups = await api.instructionBackups(path);
    expect(backups).toHaveLength(1);
    expect((await api.readInstruction(backups[0].path)).text).toBe(before);
  });

  it("asks before discarding unsaved edits when another file is picked", async () => {
    const user = userEvent.setup();
    renderWithProviders(<InstructionsPage />, { route: "/instructions" });
    await findEntry("~/.claude/CLAUDE.md");
    await user.click(entry("~/.claude/CLAUDE.md"));
    const textarea = await editor("~/.claude/CLAUDE.md");
    await user.type(textarea, " draft");
    const draft = (textarea as HTMLTextAreaElement).value;
    expect(draft.endsWith(" draft")).toBe(true);

    // Cancel keeps the draft and the selection.
    await user.click(entry("~/.codex/AGENTS.md"));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Discard changes?")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(screen.getByRole("textbox", { name: "Contents of ~/.claude/CLAUDE.md" })).toHaveValue(draft);
    expect(entry("~/.claude/CLAUDE.md")).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();

    // Discard moves on and drops the edits.
    await user.click(entry("~/.codex/AGENTS.md"));
    const again = await screen.findByRole("dialog");
    await user.click(within(again).getByRole("button", { name: "Discard" }));
    await editor("~/.codex/AGENTS.md");
    expect(screen.queryByRole("textbox", { name: "Contents of ~/.claude/CLAUDE.md" })).not.toBeInTheDocument();
    expect(entry("~/.codex/AGENTS.md")).toHaveAttribute("aria-current", "page");
    await user.click(entry("~/.claude/CLAUDE.md"));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(await editor("~/.claude/CLAUDE.md")).not.toHaveValue(draft);
  });
});
