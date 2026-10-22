// Same page-landmark check the Windows driver runs, against a Vite dev server.
// Exits 1 when the library/settings/install top moves after ?job=sync, which is
// what main@10d960b does because the job banner sits inside the page.
import { chromium } from "@playwright/test";

const base = (process.argv[2] ?? "http://127.0.0.1:1420").replace(/\/$/, "");

async function sample(page, path, landmark) {
  await page.goto(`${base}${path}`);
  await page.waitForSelector(`[data-testid=${landmark}]`);
  const samples = [];
  const started = Date.now();
  while (Date.now() - started < 2000) {
    samples.push(
      await page.evaluate((id) => {
        const node = document.querySelector(`[data-testid="${id}"]`);
        const content = document.querySelector('[data-testid="content"]');
        return {
          top: node ? node.getBoundingClientRect().top : null,
          content: content ? content.getBoundingClientRect().top : null,
          dark: document.documentElement.classList.contains("dark"),
        };
      }, landmark),
    );
    await page.waitForTimeout(100);
  }
  return samples;
}

function span(samples, key) {
  const values = samples.map((row) => row[key]).filter((value) => typeof value === "number");
  return { min: Math.min(...values), max: Math.max(...values), first: values[0], n: values.length };
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1100, height: 640 } });
await page.addInitScript(() => {
  localStorage.setItem("skill-hive-demo", "1");
  localStorage.setItem("demo-setting:theme", "dark");
});
const plain = await sample(page, "/my-skills", "library");
const job = await sample(page, "/my-skills?job=sync", "library");
const settings = await sample(page, "/settings?job=sync", "settings");
const install = await sample(page, "/install?tab=scan&job=sync", "install");
await browser.close();

const report = {
  base,
  themeDark: plain.every((row) => row.dark) && job.every((row) => row.dark),
  plain: span(plain, "top"),
  plainContent: span(plain, "content"),
  job: span(job, "top"),
  jobContent: span(job, "content"),
  settings: span(settings, "top"),
  install: span(install, "top"),
};
const moved =
  !report.themeDark ||
  Math.abs(report.job.first - report.plain.first) > 1 ||
  report.job.max - report.job.min > 1 ||
  Math.abs(report.settings.first - report.plain.first) > 1 ||
  Math.abs(report.install.first - report.plain.first) > 1;
report.moved = moved;
console.log(JSON.stringify(report, null, 2));
process.exit(moved ? 1 : 0);
