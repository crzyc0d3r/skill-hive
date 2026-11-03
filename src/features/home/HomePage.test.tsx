import { describe, expect, it } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import { renderWithProviders } from "@/test/render";
import { HomePage } from "./HomePage";

describe("HomePage", () => {
  it("shows stats, agents and recent runs from the dashboard", async () => {
    renderWithProviders(<HomePage />);
    await waitFor(() => expect(screen.getByText("Agents on this machine")).toBeInTheDocument());
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Recent evaluations")).toBeInTheDocument();
    expect(screen.getAllByText(/Helps a lot/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Description budget/i).length).toBeGreaterThan(0);
  });
});
