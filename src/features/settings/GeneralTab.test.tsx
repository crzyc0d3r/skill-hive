import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { api } from "@/lib/api/client";
import { useUi } from "@/lib/store";
import { SettingsPage } from "./SettingsPage";
import { renderWithProviders } from "@/features/evals/test-utils";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Settings · General", () => {
  it("saves each field through updateSettings with a full patch", async () => {
    const user = userEvent.setup();
    const spy = vi.spyOn(api, "updateSettings");
    renderWithProviders(<SettingsPage />, { route: "/settings", path: "/settings/:tab?" });

    // Link mode radio
    const copy = await screen.findByRole("radio", { name: /^Copy/ }, { timeout: 3000 });
    expect(copy).toHaveAttribute("aria-checked", "false");
    await user.click(copy);
    await waitFor(() => expect(spy).toHaveBeenCalledWith(expect.objectContaining({ defaultLinkMode: "copy", theme: null, githubToken: null })));
    await waitFor(() => expect(screen.getByRole("radio", { name: /^Copy/ })).toHaveAttribute("aria-checked", "true"));

    // Description budget commits on Enter
    const budget = screen.getByLabelText("Description budget characters");
    await user.clear(budget);
    await user.type(budget, "12000{Enter}");
    await waitFor(() => expect(spy).toHaveBeenCalledWith(expect.objectContaining({ defaultDescriptionBudgetChars: 12000 })));

    // Theme syncs the UI store and the backend
    await user.click(screen.getByRole("radio", { name: /Dark/ }));
    expect(useUi.getState().theme).toBe("dark");
    await waitFor(() => expect(spy).toHaveBeenCalledWith(expect.objectContaining({ theme: "dark" })));
    useUi.getState().setTheme("system");

    // Scan dir removal
    await user.click(screen.getByRole("button", { name: /^Remove .*team-skills$/ }));
    await waitFor(() => expect(spy).toHaveBeenCalledWith(expect.objectContaining({ extraScanDirs: [] })));
    expect(await screen.findByText("None.")).toBeInTheDocument();
  });

  it("routes tabs through the URL", async () => {
    const user = userEvent.setup();
    renderWithProviders(<SettingsPage />, { route: "/settings/providers", path: "/settings/:tab?" });
    expect(await screen.findByRole("button", { name: /add provider/i })).toBeInTheDocument();
    expect((await screen.findAllByText("Anthropic API", {}, { timeout: 3000 })).length).toBeGreaterThan(0);
    expect(screen.getAllByText("Codex CLI").length).toBeGreaterThan(0);
    await user.click(screen.getByRole("tab", { name: /about/i }));
    expect(await screen.findByText("Health check")).toBeInTheDocument();
    expect(await screen.findByText(/secrets backend: os-keychain/, {}, { timeout: 3000 })).toBeInTheDocument();
  });
});
