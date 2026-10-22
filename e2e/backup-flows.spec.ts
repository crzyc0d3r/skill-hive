import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", "light");
  });
  await page.setViewportSize({ width: 1100, height: 640 });
});
test("held-back removal then backup restore", async ({ page }) => {
  await page.goto("/my-skills");
  await page.getByRole("button", { name: "More" }).click();
  await page.getByRole("button", { name: "Check All" }).click();
  await expect(page.getByRole("dialog", { name: "This update removes files" })).toBeVisible();
  await page.getByRole("button", { name: "Update anyway" }).click();
  await expect(page.getByText("Update All")).toBeVisible();
  await page.goto("/backup");
  await page.getByRole("button", { name: "Back Up Now" }).click();
  await page.getByRole("button", { name: /Snapshot actions/ }).first().click();
  await page.getByRole("menuitem", { name: "Restore" }).click();
  await expect(page.getByText(/safety-snap-1/)).toBeVisible();
});
