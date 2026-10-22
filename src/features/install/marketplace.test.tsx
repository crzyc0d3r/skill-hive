import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { InstallView } from "@/features/install/InstallView";
import { MarketplacePanel } from "@/features/install/MarketplacePanel";
import { setLocale, t } from "@/lib/i18n";
import { setProjectSkills } from "@/features/project/store";
import { renderShell as renderAt } from "@/test/render-app";
import { MemoryRouter } from "react-router-dom";

describe("marketplace", () => {
  afterEach(() => {
    cleanup();
    setProjectSkills([]);
    setLocale("en");
    localStorage.clear();
  });

it("searches the marketplace, installs one skill, and cancels the next", async () => {
    const user = userEvent.setup();
    renderAt("/install?tab=leaderboard");
    const search = await screen.findByLabelText("Search marketplace");
    await user.type(search, "review");
    expect(await screen.findByText("acme/review")).toBeInTheDocument();
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

  it("stays in English and hides an unreachable marketplace", async () => {
    setLocale("zh-CN");
    expect(t("backup.title")).toBe("Backup");
    setLocale("en");
    render(
      <MemoryRouter initialEntries={["/install?tab=leaderboard"]}>
        <InstallView scanned={[]} marketplaceReachable={false} onImportAll={() => {}} onImportOne={() => {}} onPreview={() => {}} onImportGit={() => {}} onCancel={() => {}} onInstallLocal={() => {}} onInstallArchive={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.queryByRole("tab", { name: "Marketplace" })).not.toBeInTheDocument();
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MarketplacePanel reachable={false} />
      </QueryClientProvider>,
    );
    expect(screen.getByText("The marketplace arrives in a later phase.")).toBeInTheDocument();
  });
});
