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

async function samplePage(page: Page, landmark: string, ms = 2000) {
  const samples: { top: number; content: number; dark: boolean }[] = [];
  const started = Date.now();
  while (Date.now() - started < ms) {
    samples.push(
      await page.evaluate((id) => {
        const node = document.querySelector(`[data-testid="${id}"]`);
        const content = document.querySelector('[data-testid="content"]');
        return {
          top: node instanceof HTMLElement ? node.getBoundingClientRect().top : Number.NaN,
          content: content instanceof HTMLElement ? content.getBoundingClientRect().top : Number.NaN,
          dark: document.documentElement.classList.contains("dark"),
        };
      }, landmark),
    );
    await page.waitForTimeout(100);
  }
  return samples;
}

function assertStable(samples: { top: number; dark: boolean }[], dark: boolean) {
  expect(samples.length).toBeGreaterThan(5);
  for (const sample of samples) {
    expect(sample.dark).toBe(dark);
    expect(Math.abs(sample.top - samples[0].top)).toBeLessThanOrEqual(1);
  }
}

test("page landmark stays put for about 2s while a job is showing", async ({ page }) => {
  test.setTimeout(30_000);
  await page.addInitScript(() => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", "dark");
  });
  await page.setViewportSize({ width: 1100, height: 640 });
  await page.goto("/my-skills");
  await expect(page.getByTestId("library")).toBeVisible();
  await expect(page.locator("html")).toHaveClass(/dark/);
  const before = await samplePage(page, "library");
  assertStable(before, true);
  await page.goto("/my-skills?job=sync");
  await expect(page.getByTestId("job-progress")).toBeVisible();
  const during = await samplePage(page, "library");
  assertStable(during, true);
  expect(Math.abs(during[0].top - before[0].top)).toBeLessThanOrEqual(1);
  expect(during[0].content).toBe(before[0].content);
  await page.goto("/settings?job=sync");
  await expect(page.getByTestId("settings")).toBeVisible();
  const settings = await samplePage(page, "settings");
  assertStable(settings, true);
  expect(Math.abs(settings[0].top - before[0].top)).toBeLessThanOrEqual(1);
  await page.goto("/install?tab=scan&job=sync");
  await expect(page.getByTestId("install")).toBeVisible();
  const install = await samplePage(page, "install");
  assertStable(install, true);
  expect(Math.abs(install[0].top - before[0].top)).toBeLessThanOrEqual(1);
});

function contrastRatio(foreground: string, background: string): number {
  const parse = (value: string) => {
    const match = value.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
    if (!match) return null;
    return [Number(match[1]), Number(match[2]), Number(match[3])] as const;
  };
  const channel = (value: number) => {
    const s = value / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  const lum = (rgb: readonly [number, number, number]) => 0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2]);
  const fg = parse(foreground);
  const bg = parse(background);
  expect(fg).not.toBeNull();
  expect(bg).not.toBeNull();
  const lighter = Math.max(lum(fg!), lum(bg!));
  const darker = Math.min(lum(fg!), lum(bg!));
  return (lighter + 0.05) / (darker + 0.05);
}

test("the retry banner is in normal flow and keeps its text readable", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", "dark");
  });
  await page.setViewportSize({ width: 1100, height: 640 });
  await page.goto("/my-skills?banner=load");
  const banner = page.getByTestId("status-banner");
  await expect(banner).toBeVisible();
  await expect(banner).toContainText("Could not load settings.");
  const box = await page.evaluate(() => {
    const bannerNode = document.querySelector("[data-testid=status-banner]");
    const header = document.querySelector("[data-testid=header]")?.getBoundingClientRect();
    const search = [...document.querySelectorAll("button")].find((node) => node.textContent?.includes("Search"))?.getBoundingClientRect();
    const alert = bannerNode?.getBoundingClientRect();
    const heading = document.querySelector("[data-testid=library] h1")?.getBoundingClientRect();
    const pageButtons = [...document.querySelectorAll("[data-testid=library] button, [data-testid=library] a")].map((node) => node.getBoundingClientRect());
    const message = bannerNode?.querySelector("span");
    const retry = [...(bannerNode?.querySelectorAll("button") ?? [])].find((node) => node.textContent?.includes("Try again"));
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 1;
    const ctx = canvas.getContext("2d");
    const rgb = (value: string) => {
      if (!ctx || !value) return "";
      ctx.clearRect(0, 0, 1, 1);
      ctx.fillStyle = "#000";
      ctx.fillStyle = value;
      ctx.fillRect(0, 0, 1, 1);
      const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data;
      if (a === 0) return "";
      return `rgb(${r}, ${g}, ${b})`;
    };
    const paint = (node: Element | null | undefined, fallback: string) => {
      if (!node) return { color: "", background: "" };
      const style = getComputedStyle(node);
      const background = rgb(style.backgroundColor) || rgb(fallback);
      return { color: rgb(style.color), background };
    };
    const bannerBg = bannerNode ? getComputedStyle(bannerNode).backgroundColor : "";
    return {
      position: bannerNode ? getComputedStyle(bannerNode).position : "",
      headerBottom: header?.bottom ?? 0,
      searchBottom: search?.bottom ?? 0,
      searchTop: search?.top ?? 0,
      bannerTop: alert?.top ?? 0,
      bannerBottom: alert?.bottom ?? 0,
      headingTop: heading?.top ?? 0,
      pageButtons,
      message: paint(message, bannerBg),
      retry: paint(retry, bannerBg),
    };
  });
  expect(box.position).toBe("static");
  expect(box.bannerTop).toBeGreaterThanOrEqual(box.headerBottom - 1);
  expect(box.bannerTop).toBeGreaterThanOrEqual(box.searchBottom - 1);
  expect(box.headingTop).toBeGreaterThanOrEqual(box.bannerBottom - 1);
  for (const button of box.pageButtons) {
    const overlaps = button.bottom > box.bannerTop + 1 && button.top < box.bannerBottom - 1;
    expect(overlaps).toBe(false);
  }
  expect(contrastRatio(box.message.color, box.message.background)).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(box.retry.color, box.retry.background)).toBeGreaterThanOrEqual(4.5);
});
