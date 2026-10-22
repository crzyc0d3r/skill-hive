import { expect, test } from "@playwright/test";

test.describe("generate, compare, and history", () => {
  test("wizard, badges, comparison, history, and optimize", async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem("skill-hive-demo", "1");
      localStorage.setItem("demo-setting:theme", "light");
      localStorage.setItem(
        "skill-hive-generate",
        JSON.stringify({
          tested: ["skill-research"],
          regressed: ["skill-scan"],
          regressions: [{ skillId: "skill-scan", skill: "scan", rule: "R2", message: "columns failed" }],
        }),
      );
    });
    await page.setViewportSize({ width: 1100, height: 640 });
    await page.goto("/my-skills");
    await expect(page.getByTestId("tested-badge")).toBeVisible();
    await expect(page.getByTestId("regressed-badge")).toBeVisible();
    await page.goto("/");
    await expect(page.getByTestId("regressions")).toContainText("R2");
    await page.getByRole("link", { name: "Generate…" }).evaluate((node) => (node as HTMLAnchorElement).click());
    await expect(page.getByTestId("generate-wizard")).toBeVisible();
    await page.getByLabel("Goal").fill("Extract invoices");
    await page.getByRole("button", { name: "Continue" }).click();
    await page.getByRole("button", { name: "Accept all defaults" }).click();
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page.getByTestId("generate-problems")).toContainText("0");
    await expect(page.getByRole("button", { name: "Save" })).toBeDisabled();
    await page.getByRole("button", { name: "Open diff" }).click();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog", { name: "Discard this draft?" })).toBeVisible();
    await page.goto("/compare/cmp-fixture");
    await expect(page.getByLabel("Output A")).toBeVisible();
    await expect(page.getByText("security-reviewer v2")).toHaveCount(0);
    await page.getByRole("button", { name: "Record verdict" }).click();
    await expect(page.getByLabel("Output A")).toHaveValue("security-reviewer v2");
    await page.goto("/skills/skill-research/history");
    await expect(page.getByTestId("history-v2-optimized")).toContainText("0.97");
    await page.goto("/skills/skill-research/tests");
    await page.getByRole("button", { name: "Optimize description" }).click();
    await expect(page.getByTestId("optimize-preview")).toBeVisible();
    await expect(page.getByTestId("optimize-applied")).toHaveCount(0);
    await page.getByRole("button", { name: "Apply" }).click();
    await expect(page.getByTestId("optimize-applied")).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(1);
  });
});
