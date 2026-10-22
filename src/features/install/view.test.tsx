import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { InstallView } from "@/features/install/InstallView";
import { installIpc, renderAt } from "@/test/render-app";

describe("install", () => {
  it("deep-links install tabs, installs a folder, and imports one scanned skill", async () => {
    installIpc();
    const user = userEvent.setup();
    const leaderboard = renderAt("/install?tab=leaderboard");
    expect(await screen.findByRole("tab", { name: "Marketplace" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId("marketplace")).toBeInTheDocument();
    expect(screen.getByLabelText("Search marketplace")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "From a folder" }));
    await user.type(screen.getByLabelText("Select folder"), "/path/to/demo");
    await user.click(screen.getByRole("button", { name: "Install local" }));
    expect(screen.getByText("Imported demo")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Select archive"), "/path/to/demo.skill");
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
    const url = await screen.findByLabelText("Git URL");
    await user.type(url, "https://github.com/acme/skills/tree/feature/slash/tools/demo");
    await user.click(screen.getByRole("button", { name: "Clone & Preview" }));
    expect(screen.getByTestId("git-preview")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Rename skill"), "renamed-demo");
    await user.click(screen.getByRole("button", { name: "Import Selected" }));
    expect(screen.getByText("Imported renamed-demo")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel clone" }));
    expect(screen.getByText("Temporary clone directory removed.")).toBeInTheDocument();
  });

  it("keeps read failures out of the new count and names the reason", () => {
    render(
      <MemoryRouter initialEntries={["/install?tab=scan"]}>
        <InstallView
          scanned={[
            { name: "notes", tool: "cursor", path: "~/.cursor/skills/notes" },
            { name: "locked-skill", tool: "claude_code", path: "~/.claude/skills/locked-skill", reason: "error" },
            { name: "blank-skill", tool: "cursor", path: "~/.cursor/skills/blank-skill", reason: "parse" },
          ]}
          onImportAll={() => {}}
          onImportOne={() => {}}
          onPreview={() => {}}
          onImportGit={() => {}}
          onCancel={() => {}}
          onInstallLocal={() => {}}
          onInstallArchive={() => {}}
        />
      </MemoryRouter>,
    );
    expect(screen.getByText("We found 3 skills in 2 agents. 1 are new.")).toBeInTheDocument();
    expect(screen.getByText("permission denied").closest("td")).toHaveClass("text-destructive");
    expect(screen.getByText("Parse error").closest("td")).toHaveClass("text-destructive");
  });
});
