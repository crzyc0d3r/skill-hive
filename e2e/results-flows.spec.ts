import { expect, test } from "@playwright/test";

test.describe("results", () => {
  test("endpoints, suite, heatmap, evidence, and the run chip", async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem("skill-hive-demo", "1");
      localStorage.setItem("demo-setting:theme", "light");
    });
    await page.setViewportSize({ width: 1100, height: 640 });
    await page.goto("/settings/endpoints");
    await expect(page.getByTestId("endpoints")).toBeVisible();
    await expect(page.getByTestId("probe-stepper")).toBeVisible();
    await page.getByLabel("API key").fill("sk-ant-endpoint-fixture-secret");
    await page.getByRole("button", { name: "Save key" }).click();
    await page.getByRole("button", { name: "Export" }).click();
    await expect(page.getByTestId("endpoint-export")).not.toContainText("sk-ant-endpoint-fixture-secret");
    await page.goto("/my-skills");
    await page.getByRole("button", { name: "Card menu" }).first().click();
    await page.getByRole("link", { name: "Run tests" }).first().click();
    await expect(page.getByTestId("suite")).toBeVisible();
    await page.getByTestId("run-chip").click();
    await expect(page.getByTestId("results")).toBeVisible();
    const grid = page.getByTestId("heatmap");
    await grid.focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("heatmap-open")).toHaveText("without-1");
    await page.getByLabel("Jump to evidence").fill("2");
    await page.getByRole("button", { name: "Jump to evidence" }).first().click();
    await expect(page.getByTestId("evidence-anchor")).toHaveText("step-2");
    await page.getByLabel("Jump to evidence").fill("nowhere");
    await page.getByRole("button", { name: "Jump to evidence" }).first().click();
    await expect(page.getByTestId("evidence-anchor")).toHaveText("No anchor");
    await page.getByRole("button", { name: "Budget" }).click();
    await expect(page.getByTestId("budget-dialog")).toBeVisible();
    await page.getByRole("button", { name: "Confirm" }).click();
    await expect(page.getByText("Paused")).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(1);
    await page.screenshot({ path: "docs/parity/screenshots/light/results.png", fullPage: false });
    await page.evaluate(() => document.documentElement.setAttribute("data-theme", "dark"));
    await page.screenshot({ path: "docs/parity/screenshots/dark/results.png", fullPage: false });
    await page.setViewportSize({ width: 1440, height: 860 });
    await page.evaluate(() => {
      document.documentElement.setAttribute("data-theme", "light");
      document.documentElement.classList.remove("dark");
    });
    await page.screenshot({ path: "docs/parity/screenshots/light/results-1440.png", fullPage: false });
    await page.evaluate(() => {
      document.documentElement.setAttribute("data-theme", "dark");
      document.documentElement.classList.add("dark");
    });
    await page.screenshot({ path: "docs/parity/screenshots/dark/results-1440.png", fullPage: false });
  });
});
