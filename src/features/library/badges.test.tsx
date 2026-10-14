import { cleanup, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { resetGenerate } from "@/features/generate/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("library badges", () => {
  afterEach(() => {
    cleanup();
    resetGenerate();
    localStorage.clear();
  });

it("shows tested and regressed badges and the dashboard regressions block", async () => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem(
      "skill-hive-generate",
      JSON.stringify({
        tested: ["skill-research"],
        regressed: ["skill-scan"],
        regressions: [{ skillId: "skill-scan", skill: "scan", rule: "R2", message: "columns failed" }],
      }),
    );
    renderAt("/my-skills");
    expect(await screen.findByTestId("tested-badge")).toHaveTextContent("Tested");
    expect(screen.getByTestId("regressed-badge")).toHaveTextContent("Regressed");
    cleanup();
    renderAt("/");
    expect(await screen.findByTestId("tested-count")).toHaveTextContent("1");
    expect(screen.getByTestId("regression-row")).toHaveTextContent("R2");
  });
});
