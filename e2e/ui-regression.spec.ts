import { expect, test, type Page } from "@playwright/test";

const screens = [
  { name: "dashboard", path: "/", landmark: "dashboard" },
  { name: "library", path: "/my-skills", landmark: "library" },
  { name: "agents", path: "/agents", landmark: "agents" },
  { name: "install-local", path: "/install?tab=local", landmark: "install" },
  { name: "install-git", path: "/install?tab=git", landmark: "install" },
  { name: "install-scan", path: "/install?tab=scan", landmark: "install" },
  { name: "install-leaderboard", path: "/install?tab=leaderboard", landmark: "install" },
  { name: "workspace", path: "/global-workspace", landmark: "global-workspace" },
  { name: "workspace-agent", path: "/global-workspace/claude_code", landmark: "agent-page" },
  { name: "lobster", path: "/lobster-workspace", landmark: "lobster" },
  { name: "project", path: "/project/fixture", landmark: "project" },
  { name: "backup", path: "/backup", landmark: "backup" },
  { name: "settings-appearance", path: "/settings?tab=appearance", landmark: "settings" },
  { name: "settings-general", path: "/settings?tab=general", landmark: "settings" },
  { name: "settings-agents", path: "/settings?tab=agents", landmark: "settings" },
  { name: "settings-keyboard", path: "/settings?tab=keyboard", landmark: "settings" },
  { name: "settings-about", path: "/settings?tab=about", landmark: "settings" },
  { name: "endpoints", path: "/settings/endpoints", landmark: "endpoints" },
  { name: "skill-editor", path: "/skills/skill-research/edit", landmark: "skill-editor" },
  { name: "agent-editor", path: "/agents/reviewer/edit", landmark: "agent-editor" },
  { name: "generate", path: "/generate/skill", landmark: "generate-wizard" },
  { name: "suite", path: "/skills/skill-research/tests", landmark: "suite" },
  { name: "results", path: "/runs/run-fixture", landmark: "results" },
  { name: "history", path: "/skills/skill-research/history", landmark: "history" },
  { name: "compare", path: "/compare/fixture", landmark: "compare" },
] as const;

const sizes = [
  { name: "1100x640", width: 1100, height: 640 },
  { name: "1440x860", width: 1440, height: 860 },
] as const;

async function prepare(page: Page, theme: "light" | "dark", width: number, height: number) {
  await page.addInitScript((choice) => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem("demo-setting:theme", choice);
  }, theme);
  await page.setViewportSize({ width, height });
}

async function measure(page: Page) {
  return page.evaluate(() => {
    function parse(color: string) {
      const match = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/);
      if (!match) return null;
      const alpha = match[4] === undefined ? 1 : Number(match[4]);
      if (alpha < 0.9) return null;
      return [Number(match[1]), Number(match[2]), Number(match[3])];
    }
    function paint(node: Element | null) {
      let current: Element | null = node;
      while (current) {
        const channels = parse(getComputedStyle(current).backgroundColor);
        if (channels) return channels;
        current = current.parentElement;
      }
      return [255, 255, 255];
    }
    const heading = document.querySelector("h1");
    if (!heading) return { ratio: 0, sample: "missing h1" };
    const fg = parse(getComputedStyle(heading).color) ?? [0, 0, 0];
    const bg = paint(heading);
    const lum = (channels: number[]) => {
      const linear = channels.map((value) => {
        const c = value / 255;
        return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
    };
    const [hi, lo] = [lum(fg), lum(bg)].sort((a, b) => b - a);
    return { ratio: (hi + 0.05) / (lo + 0.05), sample: "h1" };
  });
}

for (const theme of ["light", "dark"] as const) {
  for (const size of sizes) {
    test(`${theme} ${size.name} has no page scroll and readable headings`, async ({ page }) => {
      await prepare(page, theme, size.width, size.height);
      for (const screen of screens) {
        await page.goto(screen.path);
        await expect(page.getByTestId(screen.landmark), screen.name).toBeVisible();
        const overflow = await page.evaluate(
          () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
        );
        expect(overflow, `${screen.name} horizontal overflow`).toBeLessThanOrEqual(1);
        const contrast = await measure(page);
        expect(contrast.ratio, `${screen.name} heading contrast`).toBeGreaterThanOrEqual(4.5);
        await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      }
    });
  }
}

test("reduced motion collapses transitions", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await prepare(page, "light", 1100, 640);
  await page.goto("/");
  await expect(page.getByTestId("dashboard")).toBeVisible();
  const duration = await page.evaluate(() => getComputedStyle(document.body).getPropertyValue("--duration-base").trim());
  expect(duration).toBe("0.01ms");
});

test("command palette opens inside 1100x640", async ({ page }) => {
  await prepare(page, "dark", 1100, 640);
  await page.goto("/");
  await page.getByTestId("dashboard").waitFor();
  await page.keyboard.press("Control+k");
  await expect(page.getByTestId("command-palette")).toBeVisible();
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(1);
});
