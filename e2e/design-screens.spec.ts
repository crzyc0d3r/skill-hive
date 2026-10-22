import fs from "node:fs";
import path from "node:path";
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

const EXTRAS = [
  { id: "install-market", path: "/install?tab=leaderboard", landmark: "install" },
  { id: "install-folder", path: "/install?tab=local", landmark: "install" },
  { id: "install-git", path: "/install?tab=git", landmark: "install" },
  { id: "settings-agents", path: "/settings?tab=agents", landmark: "settings" },
  { id: "workspace-agent", path: "/global-workspace/claude_code", landmark: "global-workspace" },
  { id: "presets", path: "/global-workspace?view=presets", landmark: "global-workspace" },
] as const;

const WIDTHS = [800, 1100, 1440, 1920] as const;
const THEMES = ["light", "dark"] as const;
const LEGACY: Record<number, string> = { 1100: "1100x640", 1440: "1440x860" };

async function boot(page: Page, theme: "light" | "dark", width: number) {
  await page.addInitScript((choice) => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", choice);
  }, theme);
  await page.setViewportSize({ width, height: 900 });
}

async function saveShot(page: Page, theme: "light" | "dark", width: number, name: string) {
  const design = path.join("docs/parity/screenshots", theme, `design-${name}-${width}.png`);
  await page.screenshot({ path: design, fullPage: false });
  const auditDir = path.join("docs/ui-audit/after", theme, String(width));
  fs.mkdirSync(auditDir, { recursive: true });
  fs.copyFileSync(design, path.join(auditDir, `${name}.png`));
  const legacy = LEGACY[width];
  if (legacy) {
    const legacyDir = path.join("docs/ui-audit/after", theme, legacy);
    fs.mkdirSync(legacyDir, { recursive: true });
    fs.copyFileSync(design, path.join(legacyDir, `${name}.png`));
  }
}

for (const theme of THEMES) {
  for (const width of WIDTHS) {
    test(`design ${theme} ${width}`, async ({ page }) => {
      test.setTimeout(240_000);
      await boot(page, theme, width);
      for (const screen of [...SCREENS, ...EXTRAS]) {
        await page.goto(screen.path);
        await expect(page.getByTestId(screen.landmark)).toBeVisible();
        await expect(page.getByTestId("route-error")).toHaveCount(0);
        const overflow = await page.locator("body").evaluate((node) => node.scrollWidth - node.clientWidth);
        expect(overflow).toBeLessThanOrEqual(1);
        await saveShot(page, theme, width, screen.id);
        if (screen.id === "library") {
          await page.goto("/my-skills?crowd=1");
          await expect(page.getByTestId("library")).toBeVisible();
          await page.getByTestId("content").evaluate((node) => {
            node.scrollTop = Math.max(0, (node.scrollHeight - node.clientHeight) / 2);
          });
          await expect.poll(() => page.getByTestId("content").evaluate((node) => node.scrollTop)).toBeGreaterThan(400);
          await expect(page.getByRole("button", { name: /crowd-skill-2\d\d/ }).first()).toBeVisible();
          await saveShot(page, theme, width, "library-scrolled");
          await page.getByTestId("content").evaluate((node) => {
            node.scrollTop = 0;
          });
        }
      }
      await page.goto("/my-skills");
      await page.getByRole("button", { name: /research-notes/ }).first().click();
      await expect(page.getByTestId("detail-panel")).toBeVisible();
      await saveShot(page, theme, width, "detail");
      await page.goto("/my-skills?crash=route");
      await expect(page.getByTestId("route-error")).toBeVisible();
      if (await page.locator("vite-error-overlay").count()) {
        await page.locator("vite-error-overlay").evaluate((node) => node.remove());
      }
      await saveShot(page, theme, width, "error-route");
      await page.goto("/?crash=app");
      await expect(page.getByTestId("route-error")).toBeVisible();
      await expect(page.getByText("Top-level render failed")).toBeVisible();
      const dark = await page.evaluate(() => document.documentElement.classList.contains("dark"));
      expect(dark).toBe(theme === "dark");
      if (await page.locator("vite-error-overlay").count()) {
        await page.locator("vite-error-overlay").evaluate((node) => node.remove());
      }
      await saveShot(page, theme, width, "error-app");
    });

    for (const state of ["empty", "loading", "error"] as const) {
      test(`design ${theme} ${width} ${state}`, async ({ page }) => {
        test.setTimeout(240_000);
        await boot(page, theme, width);
        for (const screen of [...SCREENS, { id: "workspace-agent", path: "/global-workspace/claude_code", landmark: "global-workspace" }]) {
          const join = screen.path.includes("?") ? "&" : "?";
          await page.goto(`${screen.path}${join}state=${state}`);
          await expect(page.getByTestId(screen.landmark)).toBeVisible();
          await saveShot(page, theme, width, `${screen.id}-${state}`);
          if (width === 1440) {
            fs.copyFileSync(
              path.join("docs/parity/screenshots", theme, `design-${screen.id}-${state}-${width}.png`),
              path.join("docs/parity/screenshots", theme, `design-${screen.id}-${state}.png`),
            );
          }
        }
        await page.goto(`/my-skills?skill=skill-research&state=${state}`);
        await expect(page.getByTestId("library")).toBeVisible();
        await expect(page.getByTestId("detail-panel")).toBeVisible();
        await saveShot(page, theme, width, `detail-${state}`);
        if (width === 1440) {
          fs.copyFileSync(
            path.join("docs/parity/screenshots", theme, `design-detail-${state}-${width}.png`),
            path.join("docs/parity/screenshots", theme, `design-detail-${state}.png`),
          );
        }
      });
    }
  }
}
