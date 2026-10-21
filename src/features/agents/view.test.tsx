import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetAuthoring } from "@/lib/authoring/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("agents", () => {
  afterEach(() => {
    cleanup();
    resetAuthoring();
    localStorage.clear();
  });

it("lists New agent in the palette and keeps Run tests disabled", async () => {
    const user = userEvent.setup();
    renderAt("/");
    await user.keyboard("{Control>}k{/Control}");
    expect(screen.getByText("New agent")).toBeInTheDocument();
    expect(screen.getByText("Generate skill…")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await user.click(screen.getByRole("link", { name: "Generate…" }));
    expect(await screen.findByTestId("generate-wizard")).toBeInTheDocument();
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
    await user.click(await screen.findByRole("button", { name: "Import agents" }));
    expect(screen.getByRole("dialog", { name: "An agent with this name already exists" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Import as copy" }));
    expect(screen.getByRole("link", { name: "reviewer-copy" })).toBeInTheDocument();
  });
});
