import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetEndpoints } from "@/features/endpoints/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("results", () => {
  afterEach(() => {
    cleanup();
    resetEndpoints();
    localStorage.clear();
  });

it("opens a heatmap cell with the keyboard and jumps to evidence", async () => {
    localStorage.setItem("skill-hive-demo", "1");
    const user = userEvent.setup();
    renderAt("/runs/run-fixture");
    const grid = await screen.findByTestId("heatmap");
    grid.focus();
    await user.keyboard("{ArrowRight}{Enter}");
    expect(screen.getByTestId("heatmap-open")).toHaveTextContent("without-1");
    await user.type(screen.getByLabelText("Jump to evidence"), "2");
    await user.click(screen.getAllByRole("button", { name: "Jump to evidence" })[0]);
    expect(screen.getByTestId("evidence-anchor")).toHaveTextContent("step-2");
    await user.clear(screen.getByLabelText("Jump to evidence"));
    await user.type(screen.getByLabelText("Jump to evidence"), "missing");
    await user.click(screen.getAllByRole("button", { name: "Jump to evidence" })[0]);
    expect(screen.getByTestId("evidence-anchor")).toHaveTextContent("No anchor");
  });
});
