import { beforeEach, describe, expect, it } from "vitest";
import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { useUi } from "@/lib/store";
import { renderWithProviders } from "@/test/render";
import { ConflictList } from "@/components/app/common";
import { ALL_ITEMS_QUERY } from "@/features/agents/queries";
import { LibraryPage } from "./LibraryPage";
import { MergeSkillsDialog } from "./MergeSkillsDialog";

// The mock backend keeps one library for the whole file, so each merge that is applied uses its
// own pair of skills and later tests only rely on skills no earlier test retired.
const byName = async (...names: string[]) => {
  const items = await api.listItems(ALL_ITEMS_QUERY);
  return names.map((n) => items.find((i) => i.name === n)!);
};

/** Opens the dialog for `ids`, drafts the merge and waits for the review step. */
async function draft(ids: string[]) {
  const user = userEvent.setup();
  renderWithProviders(<MergeSkillsDialog />, { route: "/library" });
  act(() => useUi.getState().openMerge(ids));
  const dialog = await screen.findByRole("dialog");
  await waitFor(() => expect(within(dialog).getByRole("button", { name: /Draft merged skill/ })).toBeEnabled());
  await user.click(within(dialog).getByRole("button", { name: /Draft merged skill/ }));
  await waitFor(() => expect(within(dialog).getByRole("button", { name: /Create merged skill|Replace / })).toBeInTheDocument(), { timeout: 3000 });
  return { user, dialog };
}

beforeEach(() => {
  useUi.setState({ mergeIds: null, detailItemId: null, selectedIds: [] });
});

describe("MergeSkillsDialog", () => {
  it("opens for two skills, defaults the provider and drafts a reviewable proposal", async () => {
    const [a, b] = await byName("conventional-commits", "git-commit-helper");
    const user = userEvent.setup();
    renderWithProviders(<MergeSkillsDialog />, { route: "/library" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    act(() => useUi.getState().openMerge([a.id, b.id]));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Merge skills")).toBeInTheDocument();
    expect(await within(dialog).findByText("conventional-commits")).toBeInTheDocument();
    expect(within(dialog).getByText("git-commit-helper")).toBeInTheDocument();
    expect(within(dialog).getByText("Sources (2)")).toBeInTheDocument();
    // The recommender is the default provider.
    await waitFor(() => expect(within(dialog).getByRole("combobox", { name: "Provider" })).toHaveTextContent("Claude Code · Claude Code CLI"));
    expect(within(dialog).getByRole("checkbox", { name: /Retire the originals/ })).toHaveAttribute("aria-checked", "true");
    expect(within(dialog).getByRole("checkbox", { name: /Carry their evaluation cases/ })).toHaveAttribute("aria-checked", "true");

    await user.click(within(dialog).getByRole("button", { name: /Draft merged skill/ }));
    expect(within(dialog).getByText(/Asking Claude Code to combine 2 skills/)).toBeInTheDocument();

    // Review step.
    await waitFor(() => expect(within(dialog).getByLabelText("Name")).toHaveValue("conventional-commits-git-commit-helper"), { timeout: 3000 });
    expect(within(dialog).getByText("Review the merged skill")).toBeInTheDocument();
    expect(within(dialog).getByText(/Kept conventional-commits's overview as the opening/)).toBeInTheDocument();
    expect(within(dialog).getByText(/^Instructions \S+ → \S+ tokens$/)).toBeInTheDocument();
    expect(within(dialog).getByText(/^Description \d+ → \d+ tokens$/)).toBeInTheDocument();
    expect(within(dialog).getByText(/^Claude Code · claude-sonnet-4-5 · \S+ in \/ \S+ out · /)).toBeInTheDocument();
    expect(within(dialog).queryByText(/eval cases? carried/)).not.toBeInTheDocument();
    expect((within(dialog).getByLabelText("Description") as HTMLTextAreaElement).value).toMatch(/Use when the task involves conventional commits or git commit helper\.$/);
    expect((within(dialog).getByLabelText("Instructions") as HTMLTextAreaElement).value).toContain("## Resolved differences");

    // Bundled files: the first source's files are kept, the second's duplicate script is not.
    expect(within(dialog).getByRole("checkbox", { name: "Keep scripts/extract.py" })).toHaveAttribute("aria-checked", "true");
    expect(within(dialog).getByRole("checkbox", { name: "Keep reference.md" })).toHaveAttribute("aria-checked", "true");
    expect(within(dialog).getByRole("checkbox", { name: "Keep git-commit-helper/scripts/extract.py" })).toHaveAttribute("aria-checked", "false");
    expect(within(dialog).getByText("Files the body refers to stay checked.")).toBeInTheDocument();

    // Warnings and what the model left out.
    expect(within(dialog).getByText(/normalised to `conventional-commits-git-commit-helper`/)).toBeInTheDocument();
    expect(within(dialog).getByText("Left out")).toBeInTheDocument();
    expect(within(dialog).getByText(/git-commit-helper's closing note about keeping the file under 500 lines/)).toBeInTheDocument();
    expect(within(dialog).getByText("git")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Create merged skill" })).toBeEnabled();
  });

  it("creates the merged skill, retires the sources and moves their deployments", async () => {
    const [pdf, xlsx] = await byName("pdf", "xlsx");
    const before = await api.deployments();
    expect(before.filter((d) => d.itemId === pdf.id).map((d) => d.targetId).sort()).toEqual(["claude-code", "codex"]);
    expect(before.filter((d) => d.itemId === xlsx.id).map((d) => d.targetId)).toEqual(["claude-code"]);

    const { user, dialog } = await draft([pdf.id, xlsx.id]);
    expect(within(dialog).getByText("3 eval cases carried")).toBeInTheDocument();
    const name = within(dialog).getByLabelText("Name");
    await user.clear(name);
    await user.type(name, "office-documents");
    expect(within(dialog).queryByText(/Replaces .* in place/)).not.toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Create merged skill" }));

    await waitFor(() => expect(useUi.getState().mergeIds).toBeNull());
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    const items = await api.listItems(ALL_ITEMS_QUERY);
    const merged = items.find((i) => i.name === "office-documents");
    expect(merged).toBeDefined();
    expect(merged!.kind).toBe("skill");
    expect(merged!.tags.sort()).toEqual([...new Set([...pdf.tags, ...xlsx.tags])].sort());
    expect(useUi.getState().detailItemId).toBe(merged!.id);
    expect(items.find((i) => i.id === pdf.id)).toBeUndefined();
    expect(items.find((i) => i.id === xlsx.id)).toBeUndefined();

    // pdf's two deployments now point at the merged skill; xlsx's duplicate on claude-code was dropped.
    const after = await api.deployments();
    const moved = after.filter((d) => d.itemId === merged!.id);
    expect(moved.map((d) => d.targetId).sort()).toEqual(["claude-code", "codex"]);
    expect(moved.every((d) => d.itemName === "office-documents" && d.path.endsWith("/office-documents"))).toBe(true);
    expect(after.some((d) => d.itemId === pdf.id || d.itemId === xlsx.id)).toBe(false);
  });

  it("replaces a source in place when the merged skill keeps its name", async () => {
    const [tdd, debugging] = await byName("test-driven-development", "systematic-debugging");
    const { user, dialog } = await draft([tdd.id, debugging.id]);

    const name = within(dialog).getByLabelText("Name");
    await user.clear(name);
    await user.type(name, "systematic-debugging");
    expect(within(dialog).getByText(/Replaces systematic-debugging in place/)).toBeInTheDocument();
    const description = (within(dialog).getByLabelText("Description") as HTMLTextAreaElement).value;
    expect(description).not.toBe(debugging.description);

    const replace = within(dialog).getByRole("button", { name: "Replace systematic-debugging" });
    expect(replace).toBeEnabled();
    await user.click(replace);
    await waitFor(() => expect(useUi.getState().mergeIds).toBeNull());
    expect(useUi.getState().detailItemId).toBe(debugging.id);

    const items = await api.listItems(ALL_ITEMS_QUERY);
    const kept = items.find((i) => i.id === debugging.id);
    expect(kept).toBeDefined();
    expect(kept!.name).toBe("systematic-debugging");
    expect(kept!.description).toBe(description);
    expect(items.find((i) => i.id === tdd.id)).toBeUndefined();
    const deployments = await api.deployments();
    expect(deployments.filter((d) => d.itemId === debugging.id).map((d) => d.targetId)).toEqual(["claude-code"]);
  });

  it("blocks drafting on an invalid name or fewer than two skills", async () => {
    const [docx, pptx] = await byName("docx", "pptx");
    const user = userEvent.setup();
    renderWithProviders(<MergeSkillsDialog />, { route: "/library" });

    act(() => useUi.getState().openMerge([docx.id, pptx.id]));
    const dialog = await screen.findByRole("dialog");
    const draftButton = within(dialog).getByRole("button", { name: /Draft merged skill/ });
    await waitFor(() => expect(draftButton).toBeEnabled());
    await user.type(within(dialog).getByLabelText("Name"), "Not A Slug");
    expect(draftButton).toBeDisabled();
    expect(within(dialog).getByText(/Lowercase letters, digits and single hyphens/)).toBeInTheDocument();
    await user.clear(within(dialog).getByLabelText("Name"));
    expect(draftButton).toBeEnabled();

    act(() => useUi.getState().openMerge([docx.id]));
    await waitFor(() => expect(screen.getByText("Sources (1)")).toBeInTheDocument());
    expect(await screen.findByText("Pick at least two skills to merge.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Draft merged skill/ })).toBeDisabled();
  });
});

describe("merge entry points", () => {
  it("offers to merge the skills of a mergeable conflict", async () => {
    const report: T.ConflictReport = {
      conflicts: [
        { kind: "description_overlap", severity: "medium", itemIds: ["item-a", "item-b"], names: ["alpha", "beta"], detail: "Descriptions are 71% similar", similarity: 0.71, sharedTerms: ["commit"] },
        { kind: "budget_exceeded", severity: "high", itemIds: [], names: [], detail: "Descriptions total 16000 characters against a budget of 15000", similarity: 1.07, sharedTerms: [] },
      ],
      score: 0.75,
    };
    const user = userEvent.setup();
    renderWithProviders(<ConflictList report={report} />);
    expect(screen.getByText("Overlapping descriptions")).toBeInTheDocument();
    expect(screen.getByText("Context budget")).toBeInTheDocument();
    const buttons = screen.getAllByRole("button", { name: /Merge/ });
    expect(buttons).toHaveLength(1);
    expect(buttons[0]).toHaveAccessibleName("Merge alpha and beta with AI");
    await user.click(buttons[0]);
    expect(useUi.getState().mergeIds).toEqual(["item-a", "item-b"]);
  });

  it("shows the library selection's merge button for skills only", async () => {
    const user = userEvent.setup();
    renderWithProviders(<LibraryPage />, { route: "/library" });
    await waitFor(() => expect(screen.getByText("docx")).toBeInTheDocument());
    expect(screen.queryByRole("button", { name: /Merge with AI/ })).not.toBeInTheDocument();

    await user.click(screen.getByLabelText("Select docx"));
    expect(screen.queryByRole("button", { name: /Merge with AI/ })).not.toBeInTheDocument();
    await user.click(screen.getByLabelText("Select pptx"));
    const merge = screen.getByRole("button", { name: /Merge with AI/ });
    expect(merge).toBeEnabled();
    expect(screen.queryByText("Only skills can be merged")).not.toBeInTheDocument();

    await user.click(screen.getByLabelText("Select code-reviewer"));
    expect(screen.getByRole("button", { name: /Merge with AI/ })).toBeDisabled();
    expect(screen.getByText("Only skills can be merged")).toBeInTheDocument();

    await user.click(screen.getByLabelText("Select code-reviewer"));
    await user.click(screen.getByRole("button", { name: /Merge with AI/ }));
    const [docx, pptx] = await byName("docx", "pptx");
    expect(useUi.getState().mergeIds).toEqual([docx.id, pptx.id]);
  });
});
