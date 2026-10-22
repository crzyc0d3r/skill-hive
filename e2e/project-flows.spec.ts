import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", "light");
  });
  await page.setViewportSize({ width: 1100, height: 640 });
});
test("project export flow changes the sidebar dot", async ({ page }) => {
  await page.goto("/project/fixture");
  await expect(page.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "grey");
  await page.getByRole("button", { name: "Link project" }).click();
  await expect(page.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "red");
  await page.getByRole("button", { name: "More notes" }).click();
  await page.getByRole("menuitem", { name: "Add to agents" }).click();
  await expect(page.getByTestId("project")).toContainText("Claude Code, Codex");
  await page.getByRole("button", { name: "More notes" }).click();
  await page.getByRole("menuitem", { name: "Toggle skill" }).click();
  await expect(page.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "grey");
  await page.getByRole("button", { name: "More notes" }).click();
  await page.getByRole("menuitem", { name: "Toggle skill" }).click();
  await page.getByRole("button", { name: "More notes" }).click();
  await page.getByRole("menuitem", { name: "Pull center" }).click();
  await expect(page.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "green");
});
