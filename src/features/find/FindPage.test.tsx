import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { api } from "@/lib/api/client";
import { renderWithProviders } from "@/features/agents/testUtils";
import { ALL_ITEMS_QUERY } from "@/features/agents/queries";
import { FindPage } from "./FindPage";

describe("FindPage", () => {
  it("ranks the library for a task and lets the user edit the suggested set", async () => {
    const user = userEvent.setup();
    renderWithProviders(<FindPage />, { route: "/find", path: "/find" });

    const box = screen.getByLabelText("Task description");
    const find = screen.getByRole("button", { name: "Find" });
    expect(find).toBeDisabled();
    await user.type(box, "pull the tables out of a PDF invoice and build a spreadsheet");
    expect(find).toBeEnabled();
    await user.click(find);

    const panel = await screen.findByRole("complementary", { name: "Suggested set" }, { timeout: 4000 });
    await within(panel).findByText("pdf", {}, { timeout: 4000 });
    expect(within(panel).getByText("xlsx")).toBeInTheDocument();
    expect(screen.getByText(/matches/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Deploy set…" })).toBeEnabled();
    await within(panel).findByText("No conflicts detected", {}, { timeout: 4000 });

    // Exclude xlsx from the results list: the panel follows and offers a reset.
    await user.click(screen.getByLabelText("Include xlsx"));
    await waitFor(() => expect(within(panel).queryByText("xlsx")).not.toBeInTheDocument());
    expect(screen.getByText("Description budget (estimate)")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Reset to suggestion" }));
    expect(within(panel).getByText("xlsx")).toBeInTheDocument();
  });

  it("analyses a set given as ?ids=", async () => {
    const items = await api.listItems(ALL_ITEMS_QUERY);
    const pdf = items.find((i) => i.name === "pdf")!;
    const helper = items.find((i) => i.name === "git-commit-helper")!;
    const commits = items.find((i) => i.name === "conventional-commits")!;
    renderWithProviders(<FindPage />, { route: `/find?ids=${pdf.id},${helper.id},${commits.id}`, path: "/find" });

    await screen.findByText("Items being checked", {}, { timeout: 4000 });
    const panel = screen.getByRole("complementary", { name: "This set" });
    await within(panel).findByText("pdf");
    expect(within(panel).getByText("git-commit-helper")).toBeInTheDocument();
    // The two commit skills overlap; the analysis says so.
    await within(panel).findByText("Overlapping descriptions", {}, { timeout: 4000 });
    expect(screen.getByRole("button", { name: "Save as loadout…" })).toBeEnabled();
  });
});
