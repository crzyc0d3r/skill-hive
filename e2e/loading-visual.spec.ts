import { expect, test, type Page } from "@playwright/test";

const screens = [
  { name: "loading-dashboard", path: "/?state=loading", landmark: "dashboard" },
  { name: "loading-library", path: "/my-skills?state=loading", landmark: "library" },
  { name: "loading-agents", path: "/agents?state=loading", landmark: "agents" },
  { name: "loading-install", path: "/install?state=loading", landmark: "install" },
  { name: "loading-backup", path: "/backup?state=loading", landmark: "backup" },
  { name: "loading-project", path: "/project/alpha?state=loading", landmark: "project" },
  { name: "loading-settings", path: "/settings?state=loading", landmark: "settings" },
  { name: "loading-endpoints", path: "/settings/endpoints?state=loading", landmark: "endpoints" },
  { name: "loading-workspace", path: "/global-workspace?state=loading", landmark: "global-workspace" },
  { name: "loading-effectiveness", path: "/effectiveness?state=loading", landmark: "effectiveness" },
  { name: "loading-suite", path: "/skills/skill-research/tests?state=loading", landmark: "suite" },
  { name: "loading-results", path: "/runs/run-fixture?state=loading", landmark: "results" },
  { name: "loading-compare", path: "/compare/fixture?state=loading", landmark: "compare" },
  { name: "loading-history", path: "/skills/skill-research/history?state=loading", landmark: "history" },
] as const;

async function prepare(page: Page, theme: "light" | "dark", width: number) {
  await page.addInitScript((choice) => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", choice);
  }, theme);
  await page.setViewportSize({ width, height: 640 });
}

async function shoot(page: Page, theme: "light" | "dark", name: string, width: number) {
  await expect(page.getByTestId("loading-state")).toBeVisible();
  await expect(page.getByRole("heading", { level: 1, name: "Loading" })).toHaveCount(0);
  await expect(page.getByTestId("status-bar")).toContainText("Skill Hive");
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow, `${name} horizontal overflow`).toBeLessThanOrEqual(1);
  const suffix = width === 1440 ? "-1440" : "";
  await page.screenshot({
    path: `docs/parity/screenshots/${theme}/${name}${suffix}.png`,
    fullPage: false,
  });
}

for (const theme of ["light", "dark"] as const) {
  for (const width of [1100, 1440] as const) {
    test.describe(`loading ${theme} ${width}`, () => {
      for (const screen of screens) {
        test(screen.name, async ({ page }) => {
          await prepare(page, theme, width);
          await page.goto(screen.path);
          await expect(page.getByTestId(screen.landmark)).toBeVisible();
          await shoot(page, theme, screen.name, width);
        });
      }
    });
  }

  test(`job progress ${theme}`, async ({ page }) => {
    await prepare(page, theme, 1100);
    await page.goto("/?job=sync");
    await expect(page.getByTestId("dashboard")).toBeVisible();
    const job = page.getByTestId("job-progress");
    await expect(job).toBeVisible();
    await expect(job).toContainText("Syncing skills…");
    await expect(job).toContainText("40%");
    await expect(job).toContainText("2 of 5");
    await expect(job.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "40");
    await expect(page.getByRole("button", { name: "Cancel" })).toBeEnabled();
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow).toBeLessThanOrEqual(1);
    await page.screenshot({
      path: `docs/parity/screenshots/${theme}/loading-job.png`,
      fullPage: false,
    });
  });
}
