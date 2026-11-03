import { describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type * as T from "@/lib/api/bindings";
import { TooltipProvider } from "@/components/ui";
import { renderWithProviders } from "@/features/agents/testUtils";
import { LoadoutsPage } from "./LoadoutsPage";
import { PlanView } from "./ApplyLoadoutCard";

const planned = (name: string, kind: T.AssetKind = "skill"): T.PlannedItem => ({ itemId: `id-${name}`, name, kind, descriptionTokens: 30, path: `/home/u/.claude/skills/${name}` });

describe("PlanView", () => {
  it("renders adds, removes, keeps and unmanaged with the resulting budget and conflicts", () => {
    const plan: T.LoadoutPlan = {
      adds: [planned("docx"), planned("pptx")],
      removes: [planned("skill-creator")],
      keeps: [planned("pdf")],
      unmanaged: [planned("my-scratch-skill")],
      conflicts: { conflicts: [{ kind: "description_overlap", severity: "medium", itemIds: [], names: ["a", "b"], detail: "Descriptions are 70% similar", similarity: 0.7, sharedTerms: ["commit"] }], score: 0.3 },
      budget: { descriptionTokens: 400, descriptionChars: 1600, bodyTokens: 0, budgetChars: 15000, usedFraction: 0.1, overBudget: false },
    };
    render(
      <TooltipProvider>
        <PlanView plan={plan} />
      </TooltipProvider>,
    );
    expect(within(screen.getByRole("region", { name: "Add" })).getByText("docx")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Add" })).getByText("pptx")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Remove" })).getByText("skill-creator")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Keep" })).getByText("pdf")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Unmanaged" })).getByText("my-scratch-skill")).toBeInTheDocument();
    expect(screen.getByText("Budget after applying")).toBeInTheDocument();
    expect(screen.getByText(/1,600 \/ 15,000 chars/)).toBeInTheDocument();
    expect(screen.getByText("Overlapping descriptions")).toBeInTheDocument();
  });
});

describe("LoadoutsPage", () => {
  it("shows the loadout, plans an exclusive apply against the preselected target and enables Apply", async () => {
    const user = userEvent.setup();
    renderWithProviders(<LoadoutsPage />, { route: "/loadouts/lo-1?target=claude-code", path: "/loadouts/:loadoutId?" });

    expect(await screen.findByDisplayValue("Document work", {}, { timeout: 4000 })).toBeInTheDocument();
    // Items of the loadout are listed.
    for (const name of ["pdf", "xlsx", "docx", "pptx"]) expect(await screen.findByRole("button", { name })).toBeInTheDocument();
    const apply = screen.getByRole("button", { name: "Apply" });
    expect(apply).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "Plan" }));
    const plan = await screen.findByLabelText("Plan", {}, { timeout: 4000 });
    const add = within(plan).getByRole("region", { name: "Add" });
    expect(within(add).getByText("docx")).toBeInTheDocument();
    expect(within(add).getByText("pptx")).toBeInTheDocument();
    const keep = within(plan).getByRole("region", { name: "Keep" });
    expect(within(keep).getByText("pdf")).toBeInTheDocument();
    expect(within(keep).getByText("xlsx")).toBeInTheDocument();
    const remove = within(plan).getByRole("region", { name: "Remove" });
    for (const name of ["skill-creator", "systematic-debugging", "code-reviewer"]) expect(within(remove).getByText(name)).toBeInTheDocument();
    expect(within(within(plan).getByRole("region", { name: "Unmanaged" })).getByText("my-scratch-skill")).toBeInTheDocument();
    expect(apply).toBeEnabled();

    // Switching strategy invalidates the plan until it is re-run.
    await user.click(screen.getByRole("radio", { name: /Additive/ }));
    expect(screen.getByText(/Plan again before applying/)).toBeInTheDocument();
    expect(apply).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Plan" }));
    const plan2 = await screen.findByLabelText("Plan", {}, { timeout: 4000 });
    expect(within(plan2).queryByRole("region", { name: "Remove" })).not.toBeInTheDocument();
    expect(apply).toBeEnabled();
  });
});
