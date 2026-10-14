import { QueryClient } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { resetAuthoring } from "../lib/phase3/store";

function renderAt(path: string) {
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

describe("phase 3 editors", () => {
  afterEach(() => {
    cleanup();
    resetAuthoring();
    localStorage.clear();
  });

  it("shows every warning-fixture finding and the status count", async () => {
    renderAt("/skills/warnings/edit");
    expect(await screen.findByTestId("editor-tabs")).toBeInTheDocument();
    expect(await screen.findByText("S-18")).toBeInTheDocument();
    expect(screen.getByText("S-23")).toBeInTheDocument();
    expect(screen.getByText("S-30")).toBeInTheDocument();
    expect(screen.getByText("S-32")).toBeInTheDocument();
    expect(screen.getByText("S-33")).toBeInTheDocument();
    expect(screen.getByTestId("status-bar")).toHaveTextContent("2 errors · 3 warnings");
    expect(screen.getByRole("button", { name: "Publish" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Publish" })).toHaveAttribute("title", "2 blocking errors");
  });

  it("turns the body gauge amber at 500 lines", async () => {
    renderAt("/skills/gauge/edit");
    const gauge = await screen.findByTestId("body-gauge");
    expect(gauge).toHaveAttribute("data-amber", "true");
    expect(screen.getByText("S-27")).toBeInTheDocument();
  });

  it("does not run a preview command until shell execution is enabled and clicked", async () => {
    const user = userEvent.setup();
    renderAt("/skills/preview-skill/edit");
    await user.click(await screen.findByRole("tab", { name: "Preview" }));
    await user.click(screen.getByRole("button", { name: "Loaded" }));
    expect(screen.getByText(/\/skills\/preview-skill/)).toBeInTheDocument();
    expect(screen.getByText("command output not shown")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Run now" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Allow skill shell execution in previews" }));
    await user.click(screen.getByRole("button", { name: "Run now" }));
    expect(screen.getByText("echo hi")).toBeInTheDocument();
  });

  it("asks before a symlink publish and marks a copy Center changed", async () => {
    const user = userEvent.setup();
    renderAt("/skills/symlink-skill/edit");
    await user.click(await screen.findByRole("button", { name: "Publish" }));
    expect(screen.getByRole("dialog", { name: "Publish to symlinked agents" })).toBeInTheDocument();
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    cleanup();
    renderAt("/skills/copy-skill/edit");
    await user.click(await screen.findByRole("button", { name: "Publish" }));
    cleanup();
    renderAt("/global-workspace/claude_code");
    expect(await screen.findByText("Center changed")).toBeInTheDocument();
  });

  it("creates a draft from Mod+N and lists it in the library", async () => {
    const user = userEvent.setup();
    renderAt("/");
    await user.keyboard("{Control>}n{/Control}");
    expect(await screen.findByRole("dialog", { name: "Choose a template" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Blank" }));
    await user.click(screen.getByRole("button", { name: "Create draft" }));
    expect(await screen.findByTestId("skill-editor")).toBeInTheDocument();
    await user.click(screen.getByRole("link", { name: "Library" }));
    expect(await screen.findByText("draft-skill")).toBeInTheDocument();
    expect(screen.getByTestId("draft-badge")).toHaveTextContent("Draft");
  });

  it("lists New agent in the palette and keeps Run tests disabled", async () => {
    const user = userEvent.setup();
    renderAt("/");
    await user.keyboard("{Control>}k{/Control}");
    expect(screen.getByText("New agent")).toBeInTheDocument();
    expect(screen.getByText("Generate skill…")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await user.click(screen.getByRole("button", { name: "Generate…" }));
    expect(screen.getByText("The generate wizard arrives in a later phase.")).toBeInTheDocument();
  });

  it("fixes an agent near-miss, exports bytes that match the preview, then goes stale", async () => {
    const user = userEvent.setup();
    renderAt("/agents/reviewer/edit");
    expect(await screen.findByText(/A-05/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Fix" }));
    const prompt = screen.getByLabelText("System prompt") as HTMLTextAreaElement;
    expect(prompt.value).toContain("disallowedTools: Bash");
    expect(prompt.value).not.toContain("disallowed_tools");
    await user.click(screen.getByRole("tab", { name: "Preview" }));
    await user.click(screen.getByRole("button", { name: "File view" }));
    const preview = screen.getByTestId("agent-file").textContent ?? "";
    await user.click(screen.getByRole("button", { name: "User" }));
    expect(screen.getByTestId("export-status")).toHaveTextContent("current");
    expect(preview).toContain("disallowedTools: Bash");
    await user.type(screen.getByLabelText("System prompt"), " more");
    expect(screen.getByTestId("export-status")).toHaveTextContent("stale");
    await user.click(screen.getByRole("button", { name: "Agents JSON" }));
    expect(screen.getByTestId("agents-json")).toHaveTextContent("prompt");
  });

  it("offers replace, copy, or cancel when an import name collides", async () => {
    const user = userEvent.setup();
    renderAt("/agents/reviewer/edit");
    await screen.findByLabelText("System prompt");
    cleanup();
    renderAt("/agents");
    await user.click(screen.getByRole("button", { name: "Import agents" }));
    expect(screen.getByRole("dialog", { name: "An agent with this name already exists" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Import as copy" }));
    expect(screen.getByRole("link", { name: "reviewer-copy" })).toBeInTheDocument();
  });

  it("lists editor shortcuts on the keyboard settings tab", async () => {
    renderAt("/settings?tab=keyboard");
    expect(await screen.findByText("Mod+S")).toBeInTheDocument();
    expect(screen.getByText("Mod+J")).toBeInTheDocument();
    expect(screen.getByText("F2")).toBeInTheDocument();
    expect(screen.getByText("Mod+N")).toBeInTheDocument();
  });
});
