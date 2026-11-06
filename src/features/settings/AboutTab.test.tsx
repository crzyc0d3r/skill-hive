import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { api } from "@/lib/api/client";
import { useUpdate } from "@/lib/update";
import { SettingsPage } from "./SettingsPage";
import { renderWithProviders } from "@/features/evals/test-utils";

afterEach(() => {
  vi.restoreAllMocks();
  useUpdate.getState().reset();
});

const renderAbout = () => renderWithProviders(<SettingsPage />, { route: "/settings/about", path: "/settings/:tab?" });

describe("Settings · About · Updates", () => {
  it("reports an up-to-date build", async () => {
    const user = userEvent.setup();
    const spy = vi.spyOn(api, "updateCheck");
    renderAbout();
    await user.click(await screen.findByRole("button", { name: /check for updates/i }));
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("This is the latest release.")).toBeInTheDocument();
    expect(screen.getByText("up to date")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /install and restart/i })).not.toBeInTheDocument();
  });

  it("offers to install a newer release and shows its notes", async () => {
    const user = userEvent.setup();
    vi.spyOn(api, "updateCheck").mockResolvedValue({
      currentVersion: "0.1.0",
      available: true,
      version: "9.9.9",
      notes: "## What's Changed\n* Faster merges",
      date: "2026-11-06",
      target: "linux-x86_64",
    });
    const install = vi.spyOn(api, "updateInstall").mockResolvedValue(undefined);
    renderAbout();
    await user.click(await screen.findByRole("button", { name: /check for updates/i }));
    expect(await screen.findByText("v9.9.9 available")).toBeInTheDocument();
    expect(screen.getByText(/Skill Hive 9\.9\.9 is available, released 2026-11-06/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /what changed/i }));
    expect(await screen.findByText("Faster merges")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /install and restart/i }));
    await waitFor(() => expect(install).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("Installed. Skill Hive is restarting.")).toBeInTheDocument();
  });

  it("keeps the update offered when the install fails", async () => {
    const user = userEvent.setup();
    vi.spyOn(api, "updateCheck").mockResolvedValue({ currentVersion: "0.1.0", available: true, version: "9.9.9", notes: null, date: null, target: "linux-x86_64" });
    vi.spyOn(api, "updateInstall").mockRejectedValue(new Error("signature mismatch"));
    renderAbout();
    await user.click(await screen.findByRole("button", { name: /check for updates/i }));
    await user.click(await screen.findByRole("button", { name: /install and restart/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent("signature mismatch");
    expect(screen.getByRole("button", { name: /install and restart/i })).toBeEnabled();
    expect(useUpdate.getState().status).toBe("available");
  });

  it("shows a failed check without hiding the section", async () => {
    const user = userEvent.setup();
    vi.spyOn(api, "updateCheck").mockRejectedValue(new Error("could not reach the release server"));
    renderAbout();
    await user.click(await screen.findByRole("button", { name: /check for updates/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent("could not reach the release server");
    expect(screen.getByText("The check did not complete.")).toBeInTheDocument();
  });

  it("saves the startup check setting", async () => {
    const user = userEvent.setup();
    const spy = vi.spyOn(api, "updateSettings");
    renderAbout();
    const toggle = await screen.findByRole("switch", { name: /check for updates on startup/i });
    await waitFor(() => expect(toggle).toBeEnabled());
    expect(toggle).toHaveAttribute("aria-checked", "true");
    await user.click(toggle);
    await waitFor(() => expect(spy).toHaveBeenCalledWith(expect.objectContaining({ autoUpdateCheck: false, theme: null })));
    await waitFor(() => expect(screen.getByRole("switch", { name: /check for updates on startup/i })).toHaveAttribute("aria-checked", "false"));
  });
});
