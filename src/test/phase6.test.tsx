import { QueryClient } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { windowRows, warmStartWithinBudget } from "../lib/perf";
import { decideUpdate } from "../lib/updater";
import { SettingsView } from "../views/phase1/SettingsView";
import { App } from "../App";

function renderAbout() {
  return render(
    <MemoryRouter initialEntries={["/settings?tab=about"]}>
      <SettingsView theme="light" setTheme={() => {}} agents={[]} />
    </MemoryRouter>,
  );
}

describe("phase 6 updater", () => {
  afterEach(() => {
    localStorage.clear();
  });

  it("checks for updates without installing", async () => {
    const user = userEvent.setup();
    renderAbout();
    await user.click(screen.getByRole("button", { name: "Check for Updates" }));
    expect(screen.getByText("You are on the latest version.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Install Update" })).not.toBeInTheDocument();
  });

  it("offers Download on Linux and Install Update plus Restart Now on macOS", async () => {
    const user = userEvent.setup();
    localStorage.setItem("skill-hive-update-version", "1.0.0");
    localStorage.setItem("skill-hive-platform", "linux");
    renderAbout();
    await user.click(screen.getByRole("button", { name: "Check for Updates" }));
    expect(screen.getByRole("link", { name: "Download" })).toHaveAttribute("href", "https://github.com/crzyc0d3r/skill-hive/releases/latest");
    localStorage.setItem("skill-hive-platform", "macos");
    render(
      <MemoryRouter initialEntries={["/settings?tab=about"]}>
        <SettingsView theme="light" setTheme={() => {}} agents={[]} />
      </MemoryRouter>,
    );
    await user.click(screen.getAllByRole("button", { name: "Check for Updates" })[1]);
    await user.click(screen.getByRole("button", { name: "Install Update" }));
    expect(screen.getByRole("button", { name: "Restart Now" })).toBeInTheDocument();
  });

  it("blocks install while a run is active", async () => {
    const user = userEvent.setup();
    localStorage.setItem("skill-hive-update-version", "1.0.0");
    localStorage.setItem("skill-hive-active-runs", "2");
    localStorage.setItem("skill-hive-platform", "windows");
    renderAbout();
    await user.click(screen.getByRole("button", { name: "Check for Updates" }));
    expect(screen.getByText("A test run is still active.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Install Update" })).not.toBeInTheDocument();
  });

  it("shows the Skills Manager MIT notice", () => {
    renderAbout();
    expect(screen.getByRole("heading", { name: "Acknowledgements" })).toBeInTheDocument();
    expect(screen.getByText(/MIT License/)).toBeInTheDocument();
    expect(screen.getByText(/Tianliang Zhang/)).toBeInTheDocument();
  });
});

describe("phase 6 budgets", () => {
  it("windows the library and a 5000-step transcript", () => {
    const skills = Array.from({ length: 1000 }, (_, index) => index);
    expect(windowRows(skills, 100)).toHaveLength(100);
    const steps = Array.from({ length: 5000 }, (_, index) => index);
    expect(windowRows(steps, 100)).toHaveLength(100);
    expect(warmStartWithinBudget(1499)).toBe(true);
    expect(warmStartWithinBudget(1500)).toBe(false);
  });

  it("chooses install on macOS and Windows and download on Linux", () => {
    for (const platform of ["macos", "windows"] as const) {
      const decision = decideUpdate({ remoteVersion: "1.0.0", platform, activeRuns: 0 });
      expect(decision.state === "available" && decision.action).toBe("install");
    }
    const linux = decideUpdate({ remoteVersion: "1.0.0", platform: "linux", activeRuns: 0 });
    expect(linux.state === "available" && linux.action).toBe("download");
  });
});

describe("help guide links", () => {
  it("links the library card to the user guide", async () => {
    const user = userEvent.setup();
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <MemoryRouter>
        <App client={client} />
      </MemoryRouter>,
    );
    await user.click(await screen.findByRole("button", { name: "Help" }));
    const library = screen.getByRole("link", { name: "Library keeps every skill in one place." });
    expect(library).toHaveAttribute("href", "https://github.com/crzyc0d3r/skill-hive/blob/main/docs/user-guide.md#library");
  });
});
