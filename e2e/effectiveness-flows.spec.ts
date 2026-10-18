import { expect, test } from "@playwright/test";

const report = JSON.stringify({
  cells: [
    { id: "with-live", label: "with_skill 1", passRate: 1 },
    { id: "without-live", label: "without_skill 1", passRate: 0 },
  ],
  steps: [{ id: 4, tool: "Write", useTs: 1736000007, resultTs: 1736000008 }],
  with_rate: 1,
  without_rate: 0,
  exportCsv: "configuration,pass_rate\nwith_skill,1\nwithout_skill,0\n",
  history: {
    iterations: [
      { version: "without_skill", parent: null, expectation_pass_rate: 0, grading_result: "baseline", is_current_best: false },
      { version: "with_skill", parent: "without_skill", expectation_pass_rate: 1, grading_result: "won", is_current_best: true },
    ],
  },
  outputA: "PINECONE",
  outputB: "plain",
  sealed: "with_skill",
});

test.describe("effectiveness screens", () => {
  test("measure, results, compare, history, and ask in light and dark", async ({ page }) => {
    await page.addInitScript((payload) => {
      window.__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args?: { key?: string }) => {
          if (cmd === "get_settings") return args?.key === "theme" ? "light" : null;
          if (cmd === "get_effectiveness_report" || cmd === "run_effectiveness_suite") return payload;
          if (cmd === "interview_generation") return JSON.stringify({ questions: ["What output format?"] });
          if (cmd === "get_managed_skills" || cmd === "list_presets" || cmd === "get_tool_status" || cmd === "get_projects") return [];
          return null;
        },
        transformCallback: () => 0,
        unregisterCallback: () => {},
        convertFileSrc: (path: string) => path,
      };
    }, report);
    await page.setViewportSize({ width: 1100, height: 640 });

    async function shot(name: string) {
      await page.screenshot({ path: `docs/effectiveness/screenshots/light/${name}.png`, fullPage: false });
      await page.evaluate(() => document.documentElement.classList.add("dark"));
      await page.screenshot({ path: `docs/effectiveness/screenshots/dark/${name}.png`, fullPage: false });
      await page.evaluate(() => document.documentElement.classList.remove("dark"));
    }

    await page.goto("/skills/skill-research/tests");
    await expect(page.getByRole("button", { name: "Measure" })).toBeVisible();
    await page.getByLabel("Endpoint").fill("http://127.0.0.1:4010");
    await page.getByRole("button", { name: "Measure" }).click();
    await expect(page.getByTestId("suite-measured")).toBeVisible();
    await shot("suite");

    await page.goto("/runs/measured");
    await expect(page.getByText("with_skill 1")).toBeVisible();
    await expect(page.getByTestId("results-summary")).toContainText("1 / 0");
    await page.getByRole("button", { name: "Export" }).click();
    await expect(page.getByTestId("results-export")).toContainText("with_skill,1");
    await shot("results");

    await page.goto("/compare/measured");
    await expect(page.getByRole("heading", { name: "Output A" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "with_skill" })).toHaveCount(0);
    await page.getByRole("button", { name: "Record verdict" }).click();
    await expect(page.getByRole("heading", { name: "with_skill" })).toBeVisible();
    await shot("compare");

    await page.goto("/skills/skill-research/history");
    await expect(page.getByTestId("history-with_skill")).toContainText("1");
    await shot("history");

    await page.goto("/generate/measured");
    await page.getByLabel("Goal").fill("classify the dock sample");
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page.getByText("No interview questions yet.")).toBeVisible();
    await page.getByRole("button", { name: "Ask the model" }).click();
    await expect(page.getByText("What output format?")).toBeVisible();
    await shot("generate");
  });
});
