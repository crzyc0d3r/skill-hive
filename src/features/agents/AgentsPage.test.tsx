import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "./testUtils";
import { AgentsPage } from "./AgentsPage";

const rowOf = (name: string) => {
  const table = screen.getByRole("table", { name: "Installed assets" });
  const cell = within(table).getByText(name, { selector: "span" });
  return cell.closest('[role="row"]') as HTMLElement;
};

describe("AgentsPage", () => {
  it("lists installed assets for the selected agent with state and management badges", async () => {
    renderWithProviders(<AgentsPage />, { route: "/agents/claude-code", path: "/agents/:targetId?" });
    await screen.findByText("my-scratch-skill", { selector: "span" }, { timeout: 4000 });
    for (const name of ["pdf", "xlsx", "skill-creator", "systematic-debugging", "code-reviewer", "git-commit-helper"]) {
      expect(rowOf(name)).toBeInTheDocument();
    }
    expect(within(rowOf("pdf")).getByText("Skill Hive")).toBeInTheDocument();
    expect(within(rowOf("pdf")).getByText("Linked")).toBeInTheDocument();
    expect(within(rowOf("my-scratch-skill")).getByText("Unmanaged")).toBeInTheDocument();
    expect(within(rowOf("my-scratch-skill")).getByRole("button", { name: "Add to library" })).toBeInTheDocument();
    // Unmanaged but already in the library: open, no import.
    expect(within(rowOf("git-commit-helper")).getByRole("button", { name: "Open git-commit-helper in library" })).toBeInTheDocument();
    expect(within(rowOf("git-commit-helper")).queryByRole("button", { name: "Add to library" })).not.toBeInTheDocument();
    // The rail shows the detected agents and the not-detected group.
    expect(screen.getByRole("button", { name: /Claude Code/ })).toHaveAttribute("aria-current", "page");
    expect(await screen.findByRole("button", { name: /Not detected/ })).toBeInTheDocument();
  });

  it("adds an unmanaged asset to the library", async () => {
    const user = userEvent.setup();
    renderWithProviders(<AgentsPage />, { route: "/agents/claude-code", path: "/agents/:targetId?" });
    await screen.findByText("my-scratch-skill", { selector: "span" }, { timeout: 4000 });
    await user.click(within(rowOf("my-scratch-skill")).getByRole("button", { name: "Add to library" }));
    await waitFor(() => expect(within(rowOf("my-scratch-skill")).getByRole("button", { name: "Open my-scratch-skill in library" })).toBeInTheDocument(), { timeout: 4000 });
    expect(within(rowOf("my-scratch-skill")).queryByRole("button", { name: "Add to library" })).not.toBeInTheDocument();
  });

  it("removes a managed deployment and an unmanaged file behind a confirmation", async () => {
    const user = userEvent.setup();
    renderWithProviders(<AgentsPage />, { route: "/agents/claude-code", path: "/agents/:targetId?" });
    await screen.findByText("code-reviewer", { selector: "span" }, { timeout: 4000 });

    await user.click(within(rowOf("code-reviewer")).getByRole("button", { name: "Remove code-reviewer" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/Remove code-reviewer from Claude Code\?/)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(screen.queryByText("code-reviewer", { selector: "span" })).not.toBeInTheDocument(), { timeout: 4000 });

    await user.click(within(rowOf("git-commit-helper")).getByRole("button", { name: "Remove git-commit-helper" }));
    const dialog2 = await screen.findByRole("dialog");
    expect(within(dialog2).getByText(/Delete git-commit-helper from disk\?/)).toBeInTheDocument();
    await user.click(within(dialog2).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(screen.queryByText("git-commit-helper", { selector: "span" })).not.toBeInTheDocument(), { timeout: 4000 });
  });
});
