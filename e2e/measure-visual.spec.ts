import { expect, test, type Page } from "@playwright/test";
import { expandForFullPage, expectRadarLabelsInside } from "./full-page";

const sizes = [
  { name: "1100", width: 1100, height: 640 },
  { name: "1440", width: 1440, height: 860 },
] as const;

async function prepare(page: Page, theme: "light" | "dark", width: number, height: number) {
  await page.addInitScript((choice) => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", choice);
  }, theme);
  await page.setViewportSize({ width, height });
}

async function shot(page: Page, theme: "light" | "dark", name: string) {
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow, `${name} horizontal overflow`).toBeLessThanOrEqual(1);
  await page.screenshot({ path: `docs/parity/screenshots/${theme}/${name}.png`, fullPage: false });
}

for (const theme of ["light", "dark"] as const) {
  for (const size of sizes) {
    const suffix = size.name === "1100" ? "" : "-1440";
    test(`measure layout ${theme} ${size.width}x${size.height}`, async ({ page }) => {
      test.skip(size.name === "1100", "1100 viewport shots stay in shell-visual");
      await prepare(page, theme, size.width, size.height);
      await page.goto("/effectiveness");
      await expect(page.getByTestId("effect-grade")).toBeVisible();
      await shot(page, theme, `effectiveness${suffix}`);
      await page.getByRole("button", { name: "Show the charts" }).click();
      await expect(page.getByTestId("effect-map")).toBeVisible();
      await expect(page.getByTestId("effect-overlay")).toBeVisible();
      const chartsOverflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      expect(chartsOverflow, "charts horizontal overflow").toBeLessThanOrEqual(1);
      await expectRadarLabelsInside(page);
      const sidebarBox = await page.getByTestId("sidebar").boundingBox();
      expect(sidebarBox?.height ?? 0).toBeGreaterThanOrEqual(size.height - 40);
      await expandForFullPage(page);
      await page.screenshot({ path: `docs/parity/screenshots/${theme}/effectiveness-charts${suffix}.png`, fullPage: true });
      await page.goto("/skills/skill-research/tests");
      await expect(page.getByTestId("suite-matrix")).toBeVisible();
      await shot(page, theme, `suite${suffix}`);
      await page.getByRole("button", { name: "Run suite" }).click();
      await expect(page.getByTestId("suite-progress")).toBeVisible();
      await shot(page, theme, `suite-progress${suffix}`);
      await page.goto("/compare/fixture");
      await expect(page.getByLabel("Output A")).toBeVisible();
      await shot(page, theme, `compare${suffix}`);
      await page.goto("/skills/skill-research/history");
      await expect(page.getByTestId("history-v1")).toBeVisible();
      await shot(page, theme, `history${suffix}`);
      await page.goto("/runs/run-fixture");
      await expect(page.getByTestId("heatmap")).toBeVisible();
      await shot(page, theme, `results-board${suffix}`);
    });

    test(`measure states ${theme} ${size.width}x${size.height}`, async ({ page }) => {
      await prepare(page, theme, size.width, size.height);
      const states = ["empty", "loading", "error", "offline"] as const;
      const routes = [
        ["effectiveness", "/effectiveness"],
        ["results", "/runs/run-fixture"],
        ["compare", "/compare/fixture"],
        ["history", "/skills/skill-research/history"],
      ] as const;
      for (const state of states) {
        for (const [name, path] of routes) {
          await page.goto(`${path}?state=${state}`);
          if (state === "loading") {
            await expect(page.getByTestId("loading-state")).toBeVisible();
            await expect(page.getByRole("heading", { level: 1, name: "Loading" })).toHaveCount(0);
          } else {
            const copy = state === "error" ? /could not be read/ : state === "offline" ? /offline/i : /No measured|No comparison|hasn't been tested/;
            await expect(page.getByText(copy).first()).toBeVisible();
          }
          await shot(page, theme, `${name}-${state}${suffix}`);
        }
      }
    });
  }
}
