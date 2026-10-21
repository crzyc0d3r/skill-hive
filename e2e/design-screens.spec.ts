import { expect, test, type Page } from "@playwright/test";

const SCREENS = [
  { id: "dashboard", path: "/", landmark: "dashboard" },
  { id: "library", path: "/my-skills", landmark: "library" },
  { id: "agents", path: "/agents", landmark: "agents" },
  { id: "install", path: "/install?tab=scan", landmark: "install" },
  { id: "settings", path: "/settings", landmark: "settings" },
  { id: "workspace", path: "/global-workspace", landmark: "global-workspace" },
  { id: "project", path: "/project/fixture", landmark: "project" },
  { id: "effectiveness", path: "/effectiveness", landmark: "effectiveness" },
  { id: "suite", path: "/skills/skill-research/tests", landmark: "suite" },
  { id: "results", path: "/runs/run-fixture", landmark: "results" },
  { id: "compare", path: "/compare/fixture", landmark: "compare" },
  { id: "history", path: "/skills/skill-research/history", landmark: "history" },
  { id: "backup", path: "/backup", landmark: "backup" },
] as const;

const WIDTHS = [800, 1100, 1440, 1920] as const;
const THEMES = ["light", "dark"] as const;

async function boot(page: Page, theme: "light" | "dark", width: number) {
  await page.addInitScript((choice) => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", choice);
  }, theme);
  await page.setViewportSize({ width, height: 900 });
}

for (const theme of THEMES) {
  for (const width of WIDTHS) {
    test(`design ${theme} ${width}`, async ({ page }) => {
      test.setTimeout(180_000);
      await boot(page, theme, width);
      for (const screen of SCREENS) {
        await page.goto(screen.path);
        await expect(page.getByTestId(screen.landmark)).toBeVisible();
        await expect(page.getByTestId("route-error")).toHaveCount(0);
        const overflow = await page.locator("body").evaluate((node) => node.scrollWidth - node.clientWidth);
        expect(overflow).toBeLessThanOrEqual(1);
        await page.screenshot({
          path: `docs/parity/screenshots/${theme}/design-${screen.id}-${width}.png`,
          fullPage: false,
        });
      }
      await page.goto("/my-skills");
      await page.getByRole("button", { name: /research-notes/ }).first().click();
      await expect(page.getByTestId("detail-panel")).toBeVisible();
      await page.screenshot({
        path: `docs/parity/screenshots/${theme}/design-detail-${width}.png`,
        fullPage: false,
      });
      await page.goto("/my-skills?crash=route");
      await expect(page.getByTestId("route-error")).toBeVisible();
      if (await page.locator("vite-error-overlay").count()) {
        await page.locator("vite-error-overlay").evaluate((node) => node.remove());
      }
      await page.screenshot({
        path: `docs/parity/screenshots/${theme}/design-error-route-${width}.png`,
        fullPage: false,
      });
      await page.goto("/?crash=app");
      await expect(page.getByTestId("route-error")).toBeVisible();
      await expect(page.getByText("Top-level render failed")).toBeVisible();
      const dark = await page.evaluate(() => document.documentElement.classList.contains("dark"));
      expect(dark).toBe(theme === "dark");
      if (await page.locator("vite-error-overlay").count()) {
        await page.locator("vite-error-overlay").evaluate((node) => node.remove());
      }
      await page.screenshot({
        path: `docs/parity/screenshots/${theme}/design-error-app-${width}.png`,
        fullPage: false,
      });
    });
  }

  for (const state of ["empty", "loading", "error"] as const) {
    test(`design ${theme} ${state}`, async ({ page }) => {
      test.setTimeout(180_000);
      await boot(page, theme, 1440);
      for (const screen of SCREENS) {
        const join = screen.path.includes("?") ? "&" : "?";
        await page.goto(`${screen.path}${join}state=${state}`);
        await expect(page.getByTestId(screen.landmark)).toBeVisible();
        await page.screenshot({
          path: `docs/parity/screenshots/${theme}/design-${screen.id}-${state}.png`,
          fullPage: false,
        });
      }
    });
  }
}
