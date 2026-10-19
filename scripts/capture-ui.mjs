import { mkdirSync } from "node:fs";
import { chromium } from "@playwright/test";

const outRoot = process.argv[2] ?? "docs/ui-audit/before";
const base = process.env.UI_BASE ?? "http://127.0.0.1:1420";

const screens = [
  ["dashboard", "/"],
  ["library", "/my-skills"],
  ["agents", "/agents"],
  ["install-local", "/install?tab=local"],
  ["install-git", "/install?tab=git"],
  ["install-scan", "/install?tab=scan"],
  ["install-leaderboard", "/install?tab=leaderboard"],
  ["workspace", "/global-workspace"],
  ["workspace-agent", "/global-workspace/claude_code"],
  ["lobster", "/lobster-workspace"],
  ["project", "/project/fixture"],
  ["backup", "/backup"],
  ["settings-appearance", "/settings?tab=appearance"],
  ["settings-general", "/settings?tab=general"],
  ["settings-agents", "/settings?tab=agents"],
  ["settings-keyboard", "/settings?tab=keyboard"],
  ["settings-about", "/settings?tab=about"],
  ["endpoints", "/settings/endpoints"],
  ["skill-editor", "/skills/skill-research/edit"],
  ["agent-editor", "/agents/reviewer/edit"],
  ["generate", "/generate/skill"],
  ["suite", "/skills/skill-research/tests"],
  ["results", "/runs/run-fixture"],
  ["history", "/skills/skill-research/history"],
  ["compare", "/compare/fixture"],
  ["effectiveness", "/effectiveness"],
];

const sizes = [
  ["1100x640", 1100, 640],
  ["1440x860", 1440, 860],
];

const browser = await chromium.launch();
for (const theme of ["light", "dark"]) {
  for (const [sizeName, width, height] of sizes) {
    const dir = `${outRoot}/${theme}/${sizeName}`;
    mkdirSync(dir, { recursive: true });
    const page = await browser.newPage({ viewport: { width, height } });
    await page.addInitScript((choice) => {
      localStorage.setItem("skill-hive-demo", "1");
      localStorage.setItem("demo-setting:theme", choice);
    }, theme);
    for (const [name, path] of screens) {
      await page.goto(`${base}${path}`, { waitUntil: "networkidle" });
      await page.waitForTimeout(250);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      await page.screenshot({ path: `${dir}/${name}.png` });
      console.log(`${theme} ${sizeName} ${name} overflow=${overflow}`);
    }
    await page.goto(`${base}/`, { waitUntil: "networkidle" });
    await page.keyboard.press("Control+k");
    await page.waitForTimeout(200);
    await page.screenshot({ path: `${dir}/command-palette.png` });
    console.log(`${theme} ${sizeName} command-palette`);
    await page.close();
  }
}
await browser.close();
