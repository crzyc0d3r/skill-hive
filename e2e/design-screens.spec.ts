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
  { id: "install-market", path: "/install?tab=leaderboard&market=initial", landmark: "install" },
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

async function settle(page: Page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    const deadline = performance.now() + 1200;
    while (performance.now() < deadline) {
      const pending = document.getAnimations().filter((animation) => {
        if (animation.playState !== "running") return false;
        const effect = animation.effect;
        if (effect && "getTiming" in effect && effect.getTiming().iterations === Infinity) return false;
        return true;
      });
      if (pending.length === 0) break;
      await Promise.race([
        Promise.all(pending.map((animation) => animation.finished.catch(() => undefined))),
        new Promise((resolve) => window.setTimeout(resolve, 40)),
      ]);
    }
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve(undefined))));
  });
}

async function saveShot(page: Page, theme: "light" | "dark", width: number, name: string) {
  await settle(page);
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
        if (screen.id === "install-folder") {
          await page.getByLabel("Select folder").fill("~/.claude/skills");
        }
        if (screen.id === "install-market") {
          await expect(page.getByText("Popular skills show up here. Search to look one up.")).toBeVisible();
          await expect(page.getByRole("button", { name: "Load more" })).toHaveCount(0);
        }
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
      await expect(page.getByRole("button", { name: "Try again" })).toBeVisible();
      await expect(page.locator("summary", { hasText: "Details" })).toBeVisible();
      if (await page.locator("vite-error-overlay").count()) {
        await page.locator("vite-error-overlay").evaluate((node) => node.remove());
      }
      await saveShot(page, theme, width, "error-route");
      await page.locator("summary", { hasText: "Details" }).click();
      await expect(page.getByText("Route render failed")).toBeVisible();
      await page.goto("/?crash=app");
      await expect(page.getByTestId("route-error")).toBeVisible();
      await expect(page.getByRole("button", { name: "Try again" })).toBeVisible();
      await expect(page.locator("summary", { hasText: "Details" })).toBeVisible();
      const dark = await page.evaluate(() => document.documentElement.classList.contains("dark"));
      expect(dark).toBe(theme === "dark");
      if (await page.locator("vite-error-overlay").count()) {
        await page.locator("vite-error-overlay").evaluate((node) => node.remove());
      }
      await saveShot(page, theme, width, "error-app");
      await page.locator("summary", { hasText: "Details" }).click();
      await expect(page.getByText("Top-level render failed")).toBeVisible();
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

const OFFLINE_ROUTES = [
  ["effectiveness", "/effectiveness"],
  ["results", "/runs/run-fixture"],
  ["compare", "/compare/fixture"],
  ["history", "/skills/skill-research/history"],
] as const;

test.describe("layout evidence", () => {
  for (const theme of THEMES) {
    for (const width of WIDTHS) {
      test(`${theme} ${width}`, async ({ page }) => {
        test.setTimeout(180_000);
        await boot(page, theme, width);
        await page.goto("/install?tab=scan");
        await expect(page.getByTestId("install")).toBeVisible();
        await expect(page.getByText("permission denied", { exact: true })).toBeVisible();
        await expect(page.getByText("We found 4 skills in 2 agents. 2 are new.")).toBeVisible();
        await saveShot(page, theme, width, "install");
        await page.goto("/install?tab=scan&state=error");
        await expect(page.getByText("Couldn't read Cursor's folder — permission denied")).toBeVisible();
        await expect(page.getByText("permission denied", { exact: true })).toBeVisible();
        await saveShot(page, theme, width, "install-error");
        if (width === 1440) {
          fs.copyFileSync(
            path.join("docs/parity/screenshots", theme, `design-install-error-${width}.png`),
            path.join("docs/parity/screenshots", theme, "design-install-error.png"),
          );
        }
        await page.goto("/install?tab=local");
        await page.getByLabel("Select folder").fill("~/.claude/skills");
        await expect(page.getByLabel("Select folder")).toHaveValue("~/.claude/skills");
        await saveShot(page, theme, width, "install-folder");
        await page.goto("/install?tab=leaderboard&market=initial");
        await expect(page.getByText("Popular skills show up here. Search to look one up.")).toBeVisible();
        await expect(page.getByRole("button", { name: "Load more" })).toHaveCount(0);
        await saveShot(page, theme, width, "install-market");
        if (width === 1440) {
          fs.copyFileSync(
            path.join("docs/parity/screenshots", theme, `design-install-market-${width}.png`),
            path.join("docs/parity/screenshots", theme, "design-install-market.png"),
          );
        }
        await page.goto("/install?tab=leaderboard&market=offline");
        await expect(page.getByTestId("marketplace-offline")).toBeVisible();
        await expect(page.getByText("The marketplace is offline. Check the connection and try again.")).toBeVisible();
        await saveShot(page, theme, width, "install-market-offline");
        await page.goto("/my-skills");
        await expect(page.getByTestId("library")).toBeVisible();
        await saveShot(page, theme, width, "library");
        await page.goto("/my-skills?crowd=1");
        await expect(page.getByText(/Generated skill/).first()).toBeVisible();
        await saveShot(page, theme, width, "library-scrolled");
        const plain = await page.getByTestId("content").evaluate((node) => node.getBoundingClientRect().top);
        await page.goto("/?banner=load");
        await expect(page.getByTestId("status-banner")).toContainText("Could not load settings.");
        const placed = await page.evaluate(() => {
          const header = document.querySelector("[data-testid=header]")?.getBoundingClientRect();
          const search = [...document.querySelectorAll("button")].find((node) => node.textContent?.includes("Search"))?.getBoundingClientRect();
          const alert = document.querySelector("[data-testid=status-banner]")?.getBoundingClientRect();
          const content = document.querySelector("[data-testid=content]")?.getBoundingClientRect();
          return {
            headerBottom: header?.bottom ?? 0,
            searchBottom: search?.bottom ?? 0,
            bannerTop: alert?.top ?? 0,
            contentTop: content?.top ?? 0,
          };
        });
        expect(placed.bannerTop).toBeGreaterThanOrEqual(placed.headerBottom - 1);
        expect(placed.bannerTop).toBeGreaterThanOrEqual(placed.searchBottom - 1);
        expect(placed.contentTop).toBe(plain);
        await saveShot(page, theme, width, "banner");
        for (const [name, route] of OFFLINE_ROUTES) {
          await page.goto(`${route}?state=offline`);
          await expect(page.getByText(/offline/i).first()).toBeVisible();
          const legacy = width === 1100 ? `${name}-offline.png` : `${name}-offline-${width}.png`;
          await page.screenshot({ path: path.join("docs/parity/screenshots", theme, legacy), fullPage: false });
          if (width === 1100) {
            await page.screenshot({ path: path.join("docs/parity/screenshots", theme, `${name}-offline-1100.png`), fullPage: false });
          }
        }
      });
    }
  }
});
