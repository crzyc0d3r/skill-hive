import { expect, test, type Page } from "@playwright/test";

async function prepare(page: Page, theme: "light" | "dark", width: number, flags: Record<string, string>) {
  await page.addInitScript(({ theme: choice, flags: extra }) => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", choice);
    for (const [key, value] of Object.entries(extra)) localStorage.setItem(key, value);
  }, { theme, flags });
  await page.setViewportSize({ width, height: 800 });
}

test("a crowded library stays one card per name and resizes its columns", async ({ page }) => {
  await prepare(page, "light", 1100, { "skill-hive-crowd": "1", "skill-hive-preview-agents": "1" });
  await page.goto("/my-skills");
  await expect(page.getByTestId("library")).toBeVisible();
  const cards = page.getByTestId("library-cards");
  await expect(cards.getByText("autopilot", { exact: true })).toHaveCount(1);
  await expect(cards.getByText("azure-outage", { exact: true })).toHaveCount(1);
  const bounded = async () => {
    const count = await cards.locator("[data-virtual-card]").count();
    expect(count).toBeGreaterThan(0);
    expect(count).toBeLessThan(80);
  };
  await bounded();

  const columnsAt = async (width: number, expected: number) => {
    await page.setViewportSize({ width, height: 800 });
    await expect(cards).toHaveAttribute("data-columns", String(expected));
    const overflow = await cards.evaluate((node) => node.scrollWidth - node.clientWidth);
    expect(overflow).toBeLessThanOrEqual(1);
    const template = await cards.evaluate((node) => getComputedStyle(node).gridTemplateColumns);
    expect(template.split(" ").filter(Boolean)).toHaveLength(expected);
  };
  await columnsAt(1100, 2);
  await columnsAt(1440, 3);
  await columnsAt(800, 1);

  await page.locator("a[href='/global-workspace/claude_code']").first().click();
  await expect(page.getByTestId("global-workspace")).toBeVisible();
  const workspace = page.getByTestId("workspace-cards");
  await expect(workspace.getByText("autopilot", { exact: true })).toHaveCount(1);
  await expect(page.getByText("Installed in 4 locations").first()).toBeVisible();
  await expect(workspace).toHaveAttribute("data-columns", "1");
  await page.setViewportSize({ width: 1440, height: 800 });
  await expect(workspace).toHaveAttribute("data-columns", "3");
  const workspaceOverflow = await workspace.evaluate((node) => node.scrollWidth - node.clientWidth);
  expect(workspaceOverflow).toBeLessThanOrEqual(1);
  await page.locator("a[href='/settings']").first().click();
  await expect(page.getByTestId("settings")).toBeVisible();
  await page.goto("/my-skills");
  await expect(page.locator("body")).not.toBeEmpty();
  await expect(page.getByTestId("route-error")).toHaveCount(0);
  await bounded();
});

for (const theme of ["light", "dark"] as const) {
  for (const width of [800, 1100, 1440, 1920] as const) {
    test(`library grid ${theme} ${width}`, async ({ page }) => {
      const columns = width < 1100 ? 1 : width < 1440 ? 2 : width < 1920 ? 3 : 4;
      await prepare(page, theme, width, { "skill-hive-crowd": "1", "skill-hive-preview-agents": "1" });
      await page.goto("/my-skills");
      await expect(page.getByTestId("library-cards")).toHaveAttribute("data-columns", String(columns));
      await expect(page.getByTestId("route-error")).toHaveCount(0);
      await page.screenshot({
        path: `docs/parity/screenshots/${theme}/library-grid-${width}.png`,
        fullPage: false,
      });
    });
  }

  test(`agent placement ${theme}`, async ({ page }) => {
    await prepare(page, theme, 1100, { "skill-hive-preview-agents": "1" });
    await page.goto("/my-skills");
    await page.getByRole("button", { name: /Autopilot/ }).click();
    const placement = page.getByTestId("agent-placement");
    await placement.scrollIntoViewIfNeeded();
    await expect(placement).toBeVisible();
    await expect(page.getByRole("button", { name: "claude_code" })).toHaveCount(0);
    await expect(placement.getByText("Claude Code")).toBeVisible();
    await expect(page.getByText("AdaL")).toHaveCount(0);
    await placement.screenshot({
      path: `docs/parity/screenshots/${theme}/agent-placement-after.png`,
    });
    await page.screenshot({
      path: `docs/parity/screenshots/${theme}/agent-placement-after.png`,
      fullPage: false,
    });
    await page.getByRole("button", { name: "Add to agent" }).click();
    await page.getByRole("button", { name: "Show all agents" }).click();
    await expect(page.getByRole("button", { name: "AdaL" })).toBeVisible();
    await page.getByRole("button", { name: "AdaL" }).scrollIntoViewIfNeeded();
    await page.screenshot({
      path: `docs/parity/screenshots/${theme}/agent-placement-show-all.png`,
      fullPage: false,
    });
  });

  test(`settings agents ${theme}`, async ({ page }) => {
    await prepare(page, theme, 1100, { "skill-hive-preview-agents": "1" });
    await page.goto("/settings?tab=agents");
    await expect(page.getByText("Agents on this machine")).toBeVisible();
    await expect(page.getByText("AdaL")).toHaveCount(0);
    await page.screenshot({
      path: `docs/parity/screenshots/${theme}/settings-agents-detected.png`,
      fullPage: false,
    });
    await page.getByRole("button", { name: "Show all agents" }).click();
    const adal = page.getByText("AdaL").first();
    await adal.scrollIntoViewIfNeeded();
    await expect(adal).toBeVisible();
    await page.screenshot({
      path: `docs/parity/screenshots/${theme}/settings-agents-all.png`,
      fullPage: false,
    });
  });

  test(`error boundary ${theme}`, async ({ page }) => {
    await prepare(page, theme, 1100, {});
    await page.goto("/my-skills?crash=route");
    await expect(page.getByTestId("sidebar")).toBeVisible();
    await expect(page.getByTestId("route-error")).toBeVisible();
    await expect(page.getByText("Route render failed")).toBeVisible();
    if (await page.locator("vite-error-overlay").count()) {
      await page.locator("vite-error-overlay").evaluate((node) => node.remove());
    }
    await page.screenshot({
      path: `docs/parity/screenshots/${theme}/error-route.png`,
      fullPage: false,
    });
    await page.goto("/?crash=app");
    await expect(page.getByTestId("route-error")).toBeVisible();
    await expect(page.getByText("Top-level render failed")).toBeVisible();
    await expect(page.getByTestId("sidebar")).toHaveCount(0);
    const dark = await page.evaluate(() => document.documentElement.classList.contains("dark"));
    expect(dark).toBe(theme === "dark");
    if (await page.locator("vite-error-overlay").count()) {
      await page.locator("vite-error-overlay").evaluate((node) => node.remove());
    }
    await page.screenshot({
      path: `docs/parity/screenshots/${theme}/error-app.png`,
      fullPage: false,
    });
  });

  test(`install scan ${theme}`, async ({ page }) => {
    await prepare(page, theme, 1100, { "skill-hive-preview-agents": "1" });
    await page.goto("/install?tab=scan");
    const scan = page.getByTestId("install");
    await expect(scan.getByText("Found in 3 locations")).toBeVisible();
    await expect(page.getByRole("button", { name: "claude_code" })).toHaveCount(0);
    await expect(scan.getByText("Claude Code")).toBeVisible();
    await expect(scan.getByText("Codex")).toBeVisible();
    await expect(scan.getByText("Cursor")).toBeVisible();
    await page.screenshot({
      path: `docs/parity/screenshots/${theme}/install-scan-grouped.png`,
      fullPage: false,
    });
  });
}
