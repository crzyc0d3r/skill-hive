// Readability and layout audit of every screen against the mock backend.
//
//   npm run build && node scripts/ui-audit.mjs [--out <dir>] [--quick] [--filter <text>]
//
// Loads each route (and a set of interactive states: sheets, dialogs, selects, menus) in both
// themes at several window widths, runs scripts/ui-audit.page.js inside the page and fails when
// any visible text falls under WCAG AA contrast, is set in a fallback font, is smaller than
// 11px, or when a layout overflows the window horizontally. Screenshots and report.json are
// written to --out (default: ui-audit/). --quick skips the middle viewport (used in CI).
import { spawn } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");
const here = dirname(fileURLToPath(import.meta.url));
const pageAudit = readFileSync(join(here, "ui-audit.page.js"), "utf8");

const argv = process.argv.slice(2);
const flag = (name) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
};
const outDir = resolve(flag("--out") ?? "ui-audit");
const quick = argv.includes("--quick");
const filter = flag("--filter") ?? "";
mkdirSync(outDir, { recursive: true });

const port = 5100 + Math.floor(Math.random() * 300);
const vite = resolve(here, "../node_modules/vite/bin/vite.js");
const preview = spawn(process.execPath, [vite, "preview", "--port", String(port), "--strictPort"], { cwd: resolve(here, ".."), stdio: ["ignore", "pipe", "pipe"] });
preview.stderr.on("data", (d) => process.stderr.write(d));
async function waitForServer() {
  for (let i = 0; i < 150; i++) {
    try {
      if ((await fetch(`http://localhost:${port}/`)).ok) return;
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error("preview server did not start");
}

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
  ["settings-agents", "/settings/agents"],
  ["settings-projects", "/settings/projects"],
  ["settings-about", "/settings/about"],
];

const openSkill = async (page) => {
  await page.getByRole("button", { name: /^pdf/ }).first().click();
  await page.waitForTimeout(500);
};
const openRunDialog = async (page) => {
  await openSkill(page);
  await page.getByRole("button", { name: /Run evaluation/ }).first().click();
  await page.waitForTimeout(700);
};

// Interactive states: [name, route, interaction]
const states = [
  ["item-detail", "/library", (page) => page.getByRole("button", { name: /Select pdf/ }).first().click()],
  ["item-detail-instructions", "/library", async (page) => {
    await page.getByRole("button", { name: /Select pdf/ }).first().click();
    await page.waitForTimeout(500);
    await page.getByRole("tab", { name: /Instructions/ }).click();
  }],
  ["item-detail-agent", "/library", (page) => page.getByRole("button", { name: /Select code-reviewer/ }).first().click()],
  ["evals-skill", "/evals", openSkill],
  ["evals-runs", "/evals", async (page) => {
    await openSkill(page);
    await page.getByRole("tab", { name: /^Runs/ }).click();
  }],
  ["run-dialog", "/evals", openRunDialog],
  ["run-dialog-select", "/evals", async (page) => {
    await openRunDialog(page);
    const trigger = page.getByRole("dialog").getByRole("combobox").first();
    await trigger.scrollIntoViewIfNeeded();
    await trigger.click();
  }],
  ["command-palette", "/", (page) => page.keyboard.press("Control+k")],
  ["dropdown-menu", "/library", async (page) => {
    const btn = page.locator('[aria-haspopup="menu"]').first();
    if (await btn.count()) await btn.click();
  }],
  ["add-provider", "/settings/providers", (page) => page.getByRole("button", { name: /Add provider/ }).first().click()],
  ["add-source", "/discover?tab=online", async (page) => {
    await page.getByRole("textbox").first().fill("anthropics/skills");
    await page.keyboard.press("Enter");
  }],
  ["loadout-edit", "/loadouts/lo-1", async (page) => {
    const btn = page.getByRole("button", { name: /Edit|Rename|Settings/ }).first();
    if (await btn.count()) await btn.click();
  }],
  ["agents-deploy", "/agents/claude-code", async (page) => {
    const btn = page.getByRole("button", { name: /Deploy|Install|Add/ }).first();
    if (await btn.count()) await btn.click();
  }],
  ["find-query", "/find", async (page) => {
    await page.getByRole("textbox").first().fill("extract tables from a pdf and build a spreadsheet");
    await page.keyboard.press("Enter");
  }],
];

// The desktop window never goes below 980px wide; 800 is a deliberately stricter check.
const viewports = quick ? [[1400, 880], [980, 640]] : [[1400, 880], [1024, 700], [980, 640]];

const report = [];
const browser = await chromium.launch();
try {
  await waitForServer();
  for (const theme of ["dark", "light"]) {
    for (const [w, h] of viewports) {
      const context = await browser.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: 1, colorScheme: theme });
      const page = await context.newPage();
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      page.on("console", (m) => {
        if (m.type() === "error") errors.push(m.text());
      });
      await page.goto(`http://localhost:${port}/#/`);
      await page.evaluate((t) => localStorage.setItem("hive.theme", t), theme);
      await page.reload();
      const run = async (name, route, interact) => {
        if (filter && !name.includes(filter)) return;
        const label = `${name}-${theme}-${w}`;
        try {
          await page.goto(`http://localhost:${port}/#${route}`);
          await page.waitForTimeout(700);
          if (interact) {
            await interact(page);
            await page.waitForTimeout(800);
          }
          const result = JSON.parse(await page.evaluate(pageAudit));
          result.state = label;
          result.errors = errors.splice(0);
          report.push(result);
          await page.screenshot({ path: join(outDir, `${label}.png`) });
          const flags = [
            result.lowContrast.length ? `contrast=${result.lowContrast.length}` : "",
            result.overflow.length ? `overflow=${result.overflow.length}` : "",
            result.clipped.length ? `clipped=${result.clipped.length}` : "",
            result.fallbackFonts.length ? `fallback-fonts=${result.fallbackFonts.length}` : "",
            result.tinyText.length ? `tiny=${result.tinyText.length}` : "",
            result.errors.length ? `errors=${result.errors.length}` : "",
          ].filter(Boolean);
          console.log(`${label}: ${flags.length ? flags.join(" ") : "ok"}`);
        } catch (e) {
          console.log(`${label}: FAILED ${String(e.message).split("\n")[0]}`);
          report.push({ state: label, failed: String(e.message).split("\n")[0] });
        }
        await page.keyboard.press("Escape").catch(() => {});
        await page.keyboard.press("Escape").catch(() => {});
      };
      for (const [name, route] of routes) await run(name, route);
      if (w !== 1024) for (const [name, route, interact] of states) await run(name, route, interact);
      await context.close();
    }
  }
} finally {
  await browser.close();
  preview.kill();
}
writeFileSync(join(outDir, "report.json"), JSON.stringify(report, null, 1));

// Summaries: distinct offending text styles, overflow, fonts, failures.
const styles = new Map();
for (const r of report) {
  for (const item of r.lowContrast ?? []) {
    const key = `${item.tag}|${item.cls}|${item.color}`;
    const cur = styles.get(key) ?? { ...item, states: new Set() };
    cur.states.add(r.state);
    cur.ratio = Math.min(cur.ratio, item.ratio);
    styles.set(key, cur);
  }
}
const overflow = report.filter((r) => r.overflow?.length);
const fonts = report.filter((r) => r.fallbackFonts?.length);
const tiny = report.filter((r) => r.tinyText?.length);
const failed = report.filter((r) => r.failed);
const errors = report.filter((r) => r.errors?.length);

console.log(`\n${report.length} states audited.`);
if (styles.size) {
  console.log(`Low-contrast text styles: ${styles.size}`);
  for (const v of [...styles.values()].sort((a, b) => a.ratio - b.ratio).slice(0, 40)) {
    console.log(`  ${v.ratio.toFixed(2)}  <${v.tag} class="${v.cls}"> "${v.text}" color=${v.color} bg=${v.bg} size=${v.size} [${[...v.states].slice(0, 3).join(", ")}${v.states.size > 3 ? ` +${v.states.size - 3}` : ""}]`);
  }
}
for (const r of overflow) console.log(`Overflow in ${r.state}: ${JSON.stringify(r.overflow.slice(0, 3))}`);
for (const r of fonts) console.log(`Fallback fonts in ${r.state}: ${JSON.stringify(r.fallbackFonts.slice(0, 2))}`);
for (const r of tiny) console.log(`Text under 11px in ${r.state}: ${JSON.stringify(r.tinyText.slice(0, 3))}`);
for (const r of errors) console.log(`Page errors in ${r.state}: ${r.errors.slice(0, 2).join(" | ")}`);
for (const r of failed) console.log(`State could not be reached: ${r.state}: ${r.failed}`);
const clipped = report.filter((r) => r.clipped?.length);
for (const r of clipped) console.log(`Clipped text (warning) in ${r.state}: ${JSON.stringify(r.clipped.slice(0, 2))}`);

const bad = styles.size + overflow.length + fonts.length + tiny.length + failed.length + errors.length;
console.log(bad ? `\nUI audit failed: ${bad} problem group(s). Screenshots and report.json are in ${outDir}` : `\nUI audit passed. Screenshots and report.json are in ${outDir}`);
process.exit(bad ? 1 : 0);
