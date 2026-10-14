import { expect, test, type Page } from "@playwright/test";

async function prepare(page: Page, theme: "light" | "dark") {
  await page.addInitScript((choice) => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", choice);
  }, theme);
  await page.setViewportSize({ width: 1100, height: 640 });
}

for (const theme of ["light", "dark"] as const) {
  test(`skill editor ${theme} 1100x640`, async ({ page }) => {
    await prepare(page, theme);
    await page.goto("/skills/warnings/edit");
    await expect(page.getByTestId("skill-editor")).toBeVisible();
    await expect(page.getByTestId("editor-tabs")).toBeVisible();
    await expect(page.getByTestId("status-bar")).toContainText("2 errors · 3 warnings");
    await expect(page.getByTestId("status-bar")).toContainText("Skill Hive 1.0.0");
    await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(1);
    await page.getByRole("tab", { name: "Preview" }).click();
    await page.getByRole("tab", { name: "Versions" }).click();
    await page.keyboard.press("Control+j");
    await expect(page.getByRole("tab", { name: "Problems" })).toHaveAttribute("aria-selected", "true");
    await page.keyboard.press("F2");
    await expect(page.getByLabel("Rename file")).toBeVisible();
    await page.keyboard.press("Control+n");
    await expect(page.getByRole("dialog", { name: "Choose a template" })).toBeVisible();
    await page.screenshot({ path: `docs/parity/screenshots/${theme}/skill-editor.png`, fullPage: false });
  });
}

test("agent editor fits 1100x640", async ({ page }) => {
  await prepare(page, "light");
  await page.goto("/agents/reviewer/edit");
  await expect(page.getByTestId("agent-editor")).toBeVisible();
  await expect(page.getByTestId("editor-tabs")).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
  await page.screenshot({ path: "docs/parity/screenshots/light/agent-editor.png", fullPage: false });
});
