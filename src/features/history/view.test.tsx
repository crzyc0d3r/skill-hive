import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetGenerate } from "@/features/generate/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("history", () => {
  afterEach(() => {
    cleanup();
    resetGenerate();
    localStorage.clear();
  });

it("shows the history chain with one current best", async () => {
    const user = userEvent.setup();
    renderAt("/skills/skill-research/history");
    expect(await screen.findByText("No measured runs yet.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Save as baseline" }));
    expect(await screen.findByTestId("history-v1")).toBeInTheDocument();
    expect(screen.getByTestId("history-v2-optimized")).toHaveTextContent("0.97");
    expect(screen.queryByText("No measured runs yet.")).not.toBeInTheDocument();
  });
});
