import { fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { installIpc, renderAt } from "@/test/render-app";

describe("settings", () => {
  it("adds a custom agent and shows the repository restart notice", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/settings?tab=agents");
    await user.type(await screen.findByRole("textbox", { name: "Agent name" }), "Field Notes");
    await user.type(screen.getByRole("textbox", { name: "Agent path" }), "~/.field-notes/skills");
    await user.click(screen.getByRole("button", { name: "New Agent" }));
    expect(screen.getByText("Field Notes")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.click(screen.getByRole("button", { name: "Save path" }));
    expect(screen.getByText("Restart to apply the new central repository path.")).toBeInTheDocument();
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
    const path = screen.getByRole("textbox", { name: "Global path Claude Code" });
    await user.clear(path);
    await user.type(path, "/tmp/claude");
    await user.click(screen.getByRole("button", { name: "Reset path" }));
    expect(path).toHaveValue("~/.claude/skills");
    await user.clear(screen.getByRole("textbox", { name: "Project path Claude Code" }));
    await user.type(screen.getByRole("textbox", { name: "Project path Claude Code" }), "custom/skills");
    expect(screen.getByRole("textbox", { name: "Project path Claude Code" })).toHaveValue("custom/skills");
    await user.type(screen.getByRole("textbox", { name: "Agent name" }), "Field Notes");
    await user.type(screen.getByRole("textbox", { name: "Agent path" }), "~/.field-notes/skills");
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
});
