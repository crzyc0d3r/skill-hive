import { QueryClient } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { setLocale, t } from "../lib/i18n";
import { setProjectSkills } from "../lib/phase2/store";
import { InstallView } from "../views/phase1/InstallView";
import { MarketplacePanel } from "../views/phase2/screens";

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

describe("phase 2 screens", () => {
  afterEach(() => {
    cleanup();
    setProjectSkills([]);
    setLocale("en");
    localStorage.clear();
  });

  it("links a project, shares it, and changes the sidebar dot", async () => {
    const user = userEvent.setup();
    renderAt("/project/fixture");
    expect(screen.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "grey");
    await user.click(await screen.findByRole("button", { name: "Link project" }));
    expect(screen.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "red");
    await user.click(screen.getByRole("button", { name: "Add to agents" }));
    expect(screen.getByText(/claude_code, codex/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Toggle skill" }));
    expect(screen.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "grey");
    await user.click(screen.getByRole("button", { name: "Toggle skill" }));
    await user.click(screen.getByRole("button", { name: "Pull center" }));
    expect(screen.getByTestId("project-dot")).toHaveAttribute("data-color", "green");
    expect(screen.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "green");
  });

  it("searches the marketplace, installs one skill, and cancels the next", async () => {
    const user = userEvent.setup();
    renderAt("/install?tab=leaderboard");
    const search = await screen.findByLabelText("Search marketplace");
    await user.type(search, "review");
    expect(screen.getByText("acme/review")).toBeInTheDocument();
    expect(screen.queryByText("beta/lint")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Contributor acme" }));
    await user.click(screen.getByRole("button", { name: "Install" }));
    expect(screen.getByText("Installed review")).toBeInTheDocument();
    await user.clear(search);
    await user.click(screen.getByRole("button", { name: "Contributor acme" }));
    await user.click(screen.getAllByRole("button", { name: "Install" })[1]);
    await user.click(screen.getByRole("button", { name: "Cancel install" }));
    expect(screen.queryByText(/Installed/)).not.toBeInTheDocument();
  });

  it("holds a file removal until Update anyway", async () => {
    const user = userEvent.setup();
    renderAt("/my-skills");
    await user.click(await screen.findByRole("button", { name: "Check All" }));
    expect(screen.getByRole("dialog", { name: "This update removes files" })).toBeInTheDocument();
    expect(screen.getByText("notes.txt")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Update anyway" }));
    expect(screen.getByText("Update All")).toBeInTheDocument();
  });

  it("backs up and restores a snapshot", async () => {
    const user = userEvent.setup();
    renderAt("/backup");
    expect(await screen.findByText("First-run restore is ready when a remote exists.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Back Up Now" }));
    await user.click(screen.getByRole("button", { name: "Restore" }));
    expect(screen.getByText(/safety-snap-1/)).toBeInTheDocument();
  });

  it("remembers a close choice and lists eight help cards", async () => {
    const user = userEvent.setup();
    renderAt("/");
    await user.click(await screen.findByTestId("request-close"));
    await user.click(screen.getByRole("checkbox", { name: "Remember my choice" }));
    await user.click(screen.getByRole("button", { name: "Minimize to tray" }));
    expect(localStorage.getItem("skill-hive-close")).toBe("minimize");
    await user.click(screen.getByRole("button", { name: "Help" }));
    const dialog = screen.getByRole("dialog", { name: "Help" });
    expect(dialog.querySelectorAll("[data-help-card]")).toHaveLength(8);
  });

  it("shows lobster agents only when the home marker is present", async () => {
    renderAt("/");
    expect(screen.queryByText("All Lobster Agents")).not.toBeInTheDocument();
    cleanup();
    localStorage.setItem("skill-hive-home", ".openclaw");
    renderAt("/global-workspace/openclaw");
    expect(await screen.findByTestId("lobster")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "All Lobster Agents" })).toBeInTheDocument();
  });

  it("switches the visible locale and hides an unreachable marketplace", async () => {
    setLocale("zh-CN");
    expect(t("backup.title")).toBe("备份");
    setLocale("zh-TW");
    expect(t("backup.title")).toBe("備份");
    setLocale("en");
    render(
      <MemoryRouter initialEntries={["/install?tab=leaderboard"]}>
        <InstallView scanned={[]} marketplaceReachable={false} />
      </MemoryRouter>,
    );
    expect(screen.queryByRole("tab", { name: "Leaderboard" })).not.toBeInTheDocument();
    render(<MarketplacePanel reachable={false} />);
    expect(screen.getByText("The marketplace arrives in a later phase.")).toBeInTheDocument();
  });
});
