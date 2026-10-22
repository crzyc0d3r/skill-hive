import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetAuthoring } from "@/lib/authoring/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("skill editor", () => {
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
    expect(await screen.findByText("Library is newer")).toBeInTheDocument();
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
});
