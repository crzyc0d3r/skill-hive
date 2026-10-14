import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { installIpc, renderAt } from "@/test/render-app";

describe("install", () => {
  it("deep-links install tabs, installs a folder, and imports one scanned skill", async () => {
    installIpc();
    const user = userEvent.setup();
    const leaderboard = renderAt("/install?tab=leaderboard");
    expect(await screen.findByRole("tab", { name: "Leaderboard" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId("marketplace")).toBeInTheDocument();
    expect(screen.getByLabelText("Search marketplace")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "Local Install" }));
    await user.click(screen.getByRole("button", { name: "Install local" }));
    expect(screen.getByText("Imported demo")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Install archive" }));
    expect(screen.getByText("Imported demo.skill")).toBeInTheDocument();
    leaderboard.unmount();
    renderAt("/install?tab=scan");
    const rename = await screen.findByLabelText("Rename scan cursor-skill");
    await user.clear(rename);
    await user.type(rename, "renamed-cursor");
    await user.click(screen.getByRole("button", { name: "Import cursor-skill" }));
    expect(screen.getByText("Imported renamed-cursor")).toBeInTheDocument();
  });

  it("previews, renames, and cancels a git clone", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/install?tab=git");
    expect(await screen.findByLabelText("Git URL")).toHaveValue("https://github.com/acme/skills/tree/feature/slash/tools/demo");
    await user.click(screen.getByRole("button", { name: "Clone & Preview" }));
    expect(screen.getByTestId("git-preview")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Import Selected" }));
    expect(screen.getByText("Imported renamed-demo")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel clone" }));
    expect(screen.getByText("Temporary clone directory removed.")).toBeInTheDocument();
  });
});
