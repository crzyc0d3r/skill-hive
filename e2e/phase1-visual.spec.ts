import { expect, test, type Page } from "@playwright/test";

const screens = [
  { name: "dashboard", path: "/", landmark: "dashboard" },
  { name: "library", path: "/my-skills", landmark: "library" },
  { name: "install-local", path: "/install?tab=local", landmark: "install" },
  { name: "install-git", path: "/install?tab=git", landmark: "install" },
  { name: "install-scan", path: "/install?tab=scan", landmark: "install" },
  { name: "install-leaderboard", path: "/install?tab=leaderboard", landmark: "install" },
  { name: "workspace", path: "/global-workspace", landmark: "global-workspace" },
  { name: "workspace-agent", path: "/global-workspace/claude_code", landmark: "global-workspace" },
  { name: "settings-appearance", path: "/settings", landmark: "settings" },
  { name: "settings-general", path: "/settings?tab=general", landmark: "settings" },
  { name: "settings-agents", path: "/settings?tab=agents", landmark: "settings" },
  { name: "settings-about", path: "/settings?tab=about", landmark: "settings" },
] as const;

async function prepare(page: Page, theme: "light" | "dark") {
  await page.addInitScript((choice) => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", choice);
  }, theme);
  await page.setViewportSize({ width: 1100, height: 640 });
}

async function assertFrame(page: Page, theme: "light" | "dark", name: string) {
  await expect(page.getByTestId("status-bar")).toContainText("Skill Hive 1.0.0");
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  if (theme === "dark") await expect(page.locator("html")).toHaveClass(/dark/);
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow, `${name} horizontal overflow`).toBeLessThanOrEqual(1);
  await page.screenshot({
    path: `docs/parity/screenshots/${theme}/${name}.png`,
    fullPage: false,
  });
}

for (const theme of ["light", "dark"] as const) {
  test.describe(`phase 1 ${theme} 1100x640`, () => {
    for (const screen of screens) {
      test(screen.name, async ({ page }) => {
        await prepare(page, theme);
        await page.goto(screen.path);
        await expect(page.getByTestId(screen.landmark)).toBeVisible();
        await assertFrame(page, theme, screen.name);
      });
    }

    test("command palette", async ({ page }) => {
      await prepare(page, theme);
      await page.goto("/");
      await page.getByTestId("dashboard").waitFor();
      await page.keyboard.press("Control+k");
      await expect(page.getByTestId("command-palette")).toBeVisible();
      await expect(page.getByLabel("Command search")).toBeVisible();
      await assertFrame(page, theme, "command-palette");
    });
  });
}

test("extra large library names stay inside 1100x640", async ({ page }) => {
  await prepare(page, "light");
  await page.goto("/settings");
  await page.getByRole("button", { name: "Extra Large" }).click();
  await page.goto("/my-skills");
  const name = page.getByRole("button", { name: "research-notes Collect sources" }).locator("span").first();
  await expect(name).toBeVisible();
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(1);
  const truncated = await name.evaluate((node) => node.className.includes("truncate"));
  expect(truncated).toBe(false);
});
