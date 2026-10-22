import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", "light");
  });
  await page.setViewportSize({ width: 1100, height: 640 });
});
test("marketplace fixture install and cancel", async ({ page }) => {
  await page.goto("/install?tab=leaderboard");
  await page.getByLabel("Search marketplace").fill("review");
  await expect(page.getByText("review", { exact: true })).toBeVisible();
  await expect(page.getByText("By acme")).toBeVisible();
  await page.getByRole("button", { name: "Contributor acme" }).click();
  await page.getByRole("button", { name: "Install" }).click();
  await expect(page.getByText("Installed review")).toBeVisible();
  await page.getByLabel("Search marketplace").fill("");
  await page.getByRole("button", { name: "Contributor acme" }).click();
  await page.getByRole("button", { name: "Install" }).nth(1).click();
  await page.getByRole("button", { name: "Cancel install" }).click();
  await expect(page.getByText("Installed review")).toHaveCount(0);
});
