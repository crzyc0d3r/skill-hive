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
  await page.getByRole("button", { name: "Add to agents" }).click();
  await expect(page.getByTestId("project")).toContainText("claude_code, codex");
  await page.getByRole("button", { name: "Toggle skill" }).click();
  await expect(page.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "grey");
  await page.getByRole("button", { name: "Toggle skill" }).click();
  await page.getByRole("button", { name: "Pull center" }).click();
  await expect(page.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "green");
});

test("marketplace fixture install and cancel", async ({ page }) => {
  await page.goto("/install?tab=leaderboard");
  await page.getByLabel("Search marketplace").fill("review");
  await expect(page.getByText("acme/review")).toBeVisible();
  await page.getByRole("button", { name: "Contributor acme" }).click();
  await page.getByRole("button", { name: "Install" }).click();
  await expect(page.getByText("Installed review")).toBeVisible();
  await page.getByLabel("Search marketplace").fill("");
  await page.getByRole("button", { name: "Contributor acme" }).click();
  await page.getByRole("button", { name: "Install" }).nth(1).click();
  await page.getByRole("button", { name: "Cancel install" }).click();
  await expect(page.getByText("Installed review")).toHaveCount(0);
});

test("held-back removal then backup restore", async ({ page }) => {
  await page.goto("/my-skills");
  await page.getByRole("button", { name: "Check All" }).click();
  await expect(page.getByRole("dialog", { name: "This update removes files" })).toBeVisible();
  await page.getByRole("button", { name: "Update anyway" }).click();
  await expect(page.getByText("Update All")).toBeVisible();
  await page.goto("/backup");
  await page.getByRole("button", { name: "Back Up Now" }).click();
  await page.getByRole("button", { name: "Restore" }).click();
  await expect(page.getByText(/safety-snap-1/)).toBeVisible();
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
