// Capture screenshots of every screen using the in-browser mock backend.
// Usage: npm run build && node scripts/screenshots.mjs [outDir] [route-filter]
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");

const outDir = process.argv[2] ?? "screenshots";
const filter = process.argv[3] ?? "";
const port = 4173 + Math.floor(Math.random() * 500);
mkdirSync(outDir, { recursive: true });

const routes = [
  ["home", "/"],
  ["library", "/library"],
  ["discover", "/discover"],
  ["discover-online", "/discover?tab=online"],
  ["discover-catalog", "/discover?tab=catalog"],
  ["find", "/find"],
  ["agents", "/agents/claude-code"],
  ["loadouts", "/loadouts/lo-1"],
  ["evals", "/evals"],
  ["run", "/runs/run-1"],
  ["settings", "/settings"],
  ["settings-providers", "/settings/providers"],
].filter(([name]) => !filter || name.includes(filter));

const vite = resolve("node_modules/vite/bin/vite.js");
const preview = spawn(process.execPath, [vite, "preview", "--port", String(port), "--strictPort"], { stdio: ["ignore", "pipe", "pipe"] });
preview.stderr.on("data", (d) => process.stderr.write(d));

async function waitForServer() {
  for (let i = 0; i < 100; i++) {
    try {
      const res = await fetch(`http://localhost:${port}/`);
      if (res.ok) return;
    } catch {
      // not up yet
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error("preview server did not start");
}

const browser = await chromium.launch();
try {
  await waitForServer();
  for (const theme of ["dark", "light"]) {
    const context = await browser.newContext({ viewport: { width: 1400, height: 880 }, deviceScaleFactor: 1, colorScheme: theme });
    const page = await context.newPage();
    page.on("pageerror", (e) => console.error(`[${theme}] page error:`, e.message));
    page.on("console", (m) => {
      if (m.type() === "error") console.error(`[${theme}] console:`, m.text());
    });
    await page.goto(`http://localhost:${port}/#/`);
    await page.evaluate((t) => localStorage.setItem("hive.theme", t), theme);
    await page.reload();
    for (const [name, route] of routes) {
      await page.goto(`http://localhost:${port}/#${route}`);
      await page.waitForTimeout(1100);
      await page.screenshot({ path: join(outDir, `${name}-${theme}.png`) });
      console.log(`captured ${name}-${theme}`);
    }
    if (!filter || "item-detail".includes(filter)) {
      await page.goto(`http://localhost:${port}/#/library`);
      await page.waitForTimeout(800);
      const card = page.getByRole("button", { name: /Select pdf/ }).first();
      if (await card.count()) {
        await card.click();
        await page.waitForTimeout(800);
        await page.screenshot({ path: join(outDir, `item-detail-${theme}.png`) });
        console.log(`captured item-detail-${theme}`);
      }
    }
    await context.close();
  }
} finally {
  await browser.close();
  preview.kill();
}
