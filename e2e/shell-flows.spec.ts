import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", "light");
  });
  await page.setViewportSize({ width: 1100, height: 640 });
});
test("close guard remembers and lobster stays hidden", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("All Lobster Agents")).toHaveCount(0);
  await page.getByTestId("request-close").click();
  await page.getByRole("checkbox", { name: "Remember my choice" }).check();
  await page.getByRole("button", { name: "Quit" }).click();
  await expect.poll(() => page.evaluate(() => localStorage.getItem("skill-hive-close"))).toBe("quit");
  await page.getByRole("button", { name: "Help" }).click();
  await expect(page.locator("[data-help-card]")).toHaveCount(8);
});

test("openclaw route redirects into the lobster workspace", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("skill-hive-home", ".openclaw");
  });
  await page.goto("/global-workspace/openclaw");
  await expect(page.getByTestId("lobster")).toBeVisible();
  await expect(page.getByText("All Lobster Agents")).toBeVisible();
});
