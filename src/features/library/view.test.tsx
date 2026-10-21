import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { installIpc, renderAt, SKILLS } from "@/test/render-app";

describe("library", () => {
  it("filters, renames a tag, syncs two cards, and opens the diff", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/my-skills");
    expect(await screen.findByText("research-notes")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Local folder" }));
    expect(screen.queryByText("review-diff")).not.toBeInTheDocument();
    await act(async () => {
      screen.getByRole("button", { name: "research" }).dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
    });
    const field = await screen.findByLabelText("Rename tag");
    await user.clear(field);
    await user.type(field, "sources");
    await user.click(screen.getByRole("button", { name: "Rename tag" }));
    expect(screen.getByRole("button", { name: "sources" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Local folder" }));
    await user.click(screen.getByRole("checkbox", { name: "Select research-notes" }));
    await user.click(screen.getByRole("checkbox", { name: "Select review-diff" }));
    await user.click(screen.getByRole("button", { name: "Sync to agents" }));
    await user.click(screen.getByRole("checkbox", { name: "Claude Code" }));
    await user.click(screen.getAllByRole("button", { name: "Sync to agents" })[1]);
    expect(screen.getByTestId("sync-dots-skill-research").querySelector("[data-filled='true']")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: /research-notes/ }));
    await user.click(screen.getByRole("button", { name: "Diff" }));
    expect(screen.getByTestId("diff-view")).toHaveTextContent("Source body");
  });

  it("renders the library at extra large without truncating names", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/settings");
    await user.click(await screen.findByRole("button", { name: "Extra Large" }));
    expect(document.documentElement.dataset.textSize).toBe("xl");
    renderAt("/my-skills");
    const name = await screen.findByText("research-notes");
    expect(name.className).not.toContain("truncate");
  });

  it("reorders a preset, uses the card menu, and deletes a tag", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/my-skills");
    expect(await screen.findByText("research-notes")).toBeInTheDocument();
    const members = document.querySelectorAll("[data-member]");
    fireEvent.dragStart(members[1]);
    fireEvent.drop(members[0]);
    expect(screen.getByTestId("preset-order").textContent?.indexOf("review-diff")).toBeLessThan(
      screen.getByTestId("preset-order").textContent?.indexOf("research-notes") ?? 0,
    );
    await user.click(screen.getByRole("checkbox", { name: "Preset membership research-notes" }));
    expect(screen.getByTestId("preset-order")).not.toHaveTextContent("research-notes");
    await user.click(screen.getAllByRole("button", { name: "Card menu" })[0]);
    await user.click(screen.getByRole("button", { name: "Check" }));
    expect(screen.getByText("Checked research-notes")).toBeInTheDocument();
    await act(async () => {
      screen.getByRole("button", { name: "research" }).dispatchEvent(new MouseEvent("contextmenu", { bubbles: true }));
    });
    await user.click(screen.getByRole("button", { name: "Delete tag" }));
    expect(screen.queryByRole("button", { name: "research" })).not.toBeInTheDocument();
    await user.click(screen.getAllByRole("button", { name: "Card menu" })[0]);
    await user.click(screen.getByRole("button", { name: "Delete" }));
    expect(screen.queryByText("research-notes")).not.toBeInTheDocument();
  });

  it("runs batch actions, agent toggles, and the per-file diff", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/my-skills");
    await screen.findByText("plain");
    await user.click(screen.getByRole("checkbox", { name: "Select plain" }));
    await user.click(screen.getByRole("button", { name: "Enable" }));
    await user.click(screen.getByRole("button", { name: "Available" }));
    expect(screen.queryByText("plain")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "All" }));
    await user.click(screen.getByRole("checkbox", { name: "Select plain" }));
    await user.click(screen.getByRole("checkbox", { name: "Select research-notes" }));
    await user.click(screen.getByRole("button", { name: "Tags" }));
    await user.type(screen.getByRole("textbox", { name: "Tags" }), "batch");
    await user.click(screen.getAllByRole("button", { name: "Tags" })[1]);
    expect(screen.getByRole("button", { name: "Remove tag batch" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /research-notes/ }));
    await user.click(screen.getByRole("button", { name: "Add to agent" }));
    const addChoices = screen.getAllByRole("button", { name: "Claude Code" });
    await user.click(addChoices[addChoices.length - 1]);
    expect(screen.getByRole("button", { name: "Remove Claude Code" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /review-diff/ }));
    await user.click(screen.getByRole("button", { name: "Diff" }));
    expect(screen.getByTestId("file-diff")).toHaveTextContent("SKILL.md");
    expect(screen.getByTestId("file-diff")).toHaveTextContent("right");
    await user.click(screen.getByRole("checkbox", { name: "Select plain" }));
    await user.click(screen.getByRole("button", { name: "Delete selected" }));
    expect(screen.queryByText("plain")).not.toBeInTheDocument();
  });

  it("renders one card for a duplicated name and keeps a large library windowed", async () => {
    const skills = Array.from({ length: 520 }, (_, index) => ({
      ...SKILLS[0],
      id: `skill-${index}`,
      name: index < 4 ? "autopilot" : `skill-${index}`,
      tags: [],
      agents: index < 4 ? [["claude_code", "codex", "cursor", "windsurf"][index]] : [],
      locations: index < 4 ? [`/tmp/${index}/autopilot`] : [],
    }));
    installIpc(skills);
    renderAt("/my-skills");
    expect(await screen.findByText("autopilot")).toBeInTheDocument();
    expect(screen.getAllByText("autopilot")).toHaveLength(1);
    const list = document.querySelector("[data-testid='library-cards']");
    expect(list?.children.length ?? 0).toBeLessThanOrEqual(24);
    expect(list?.children.length ?? 0).toBeGreaterThan(0);
  });
});
