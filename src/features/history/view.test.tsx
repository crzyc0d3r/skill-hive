import { cleanup, screen } from "@testing-library/react";
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
    renderAt("/skills/skill-research/history");
    expect(await screen.findByTestId("history-v1")).toBeInTheDocument();
    expect(screen.getByTestId("history-v2-optimized")).toHaveTextContent("0.97");
  });
});
