import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithProviders } from "@/test/render";
import { LibraryPage } from "./LibraryPage";
import { ItemDetailSheet } from "./ItemDetailSheet";
import { useUi } from "@/lib/store";

describe("LibraryPage", () => {
  it("lists library items from the backend and filters by search", async () => {
    renderWithProviders(<LibraryPage />, { route: "/library" });
    await waitFor(() => expect(screen.getByText("pdf")).toBeInTheDocument());
    expect(screen.getByText("conventional-commits")).toBeInTheDocument();

    const user = userEvent.setup();
    await user.type(screen.getByPlaceholderText(/Search name/), "spreadsheet");
    await waitFor(() => expect(screen.queryByText("conventional-commits")).not.toBeInTheDocument());
    expect(screen.getByText("xlsx")).toBeInTheDocument();
  });

  it("filters by kind and shows effectiveness badges", async () => {
    renderWithProviders(<LibraryPage />, { route: "/library" });
    await waitFor(() => expect(screen.getByText("pdf")).toBeInTheDocument());
    expect(screen.getAllByText(/Helps a lot/).length).toBeGreaterThan(0);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Agents" }));
    await waitFor(() => expect(screen.queryByText("pdf")).not.toBeInTheDocument());
    expect(screen.getByText("code-reviewer")).toBeInTheDocument();
  });

  it("opens the detail panel for an item", async () => {
    renderWithProviders(
      <>
        <LibraryPage />
        <ItemDetailSheet />
      </>,
      { route: "/library" },
    );
    await waitFor(() => expect(screen.getByText("pdf")).toBeInTheDocument());
    const user = userEvent.setup();
    const card = screen.getAllByText("pdf")[0].closest('[role="button"]');
    expect(card).not.toBeNull();
    await user.click(card!);
    await waitFor(() => expect(useUi.getState().detailItemId).not.toBeNull());
    const dialog = await screen.findByRole("dialog");
    await waitFor(() => expect(within(dialog).getByText(/Measured effectiveness/)).toBeInTheDocument());
    expect(within(dialog).getByText(/anthropics\/skills/)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("tab", { name: /Deployments/ }));
    expect(within(dialog).getByText("claude-code")).toBeInTheDocument();
  });
});
