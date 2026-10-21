import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetEndpoints } from "@/features/endpoints/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("suites", () => {
  afterEach(() => {
    cleanup();
    resetEndpoints();
    localStorage.clear();
  });

it("confirms a budget pause and shows the run chip", async () => {
    const user = userEvent.setup();
    renderAt("/skills/skill-research/tests");
    expect(await screen.findByTestId("run-chip")).toHaveAttribute("href", "/runs/run-fixture");
    expect(screen.getByTestId("suite")).toBeInTheDocument();
    expect(screen.getByTestId("suite-matrix")).toHaveTextContent("3");
    await user.click(screen.getByRole("button", { name: "Run suite" }));
    expect(await screen.findByTestId("suite-progress")).toBeInTheDocument();
    cleanup();
    renderAt("/runs/run-fixture");
    await user.click(await screen.findByRole("button", { name: "Budget" }));
    expect(screen.getByTestId("budget-dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.getByText("Paused")).toBeInTheDocument();
  });
});
