#!/usr/bin/env node
// Launch the built desktop binary against an empty home directory and fail unless the interface
// reports that it rendered and the backend answered. This is what tells us that a build starts
// on each platform, since nothing else ever runs the real app.
//
//   node scripts/smoke-desktop.mjs [--bin <path> | --dir <dir>] [--timeout <seconds>] [--out <dir>]
//                                  [--port <n>] [--require-devtools]
//
// Defaults: the newest of "Skill Hive[.exe]" / "skill-hive[.exe]" under target/release (the
// Tauri CLI renames the binary to the product name on macOS and Windows), 90 seconds,
// ./smoke-desktop, DevTools port 9333. Build with `npx tauri build --no-bundle`: a plain
// `cargo build` leaves out tauri/custom-protocol and the binary would load the dev server URL.
// On Linux without a display wrap it in `xvfb-run -a`.
//
// How readiness is detected: the app logs `ui: ready v<version>` (src/app/shell/AppShell.tsx
// calls the `ui_report` command once the version arrived from Rust) to <home>/logs/skill-hive.log;
// a boot failure in the interface logs `ui: boot failed: ...`; a fatal error before the window
// writes <home>/logs/skill-hive-crash.log. SKILL_HIVE_HEADLESS suppresses the native error dialog.
// The log is the verdict. Everything else is evidence for a human: a screenshot of the desktop
// taken with the OS's own tools, and on Windows, when WebView2 exposes its DevTools protocol on
// the requested port, the page text, console errors and a page screenshot (best effort unless
// --require-devtools is given).

import { execSync, spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const args = parseArgs(process.argv.slice(2));
const platform = process.platform;
const timeoutMs = Number(args.timeout ?? 90) * 1000;
const out = path.resolve(args.out ?? "smoke-desktop");
const cdpPort = Number(args.port ?? 9333);
const requireDevtools = args["require-devtools"] === "true";
mkdirSync(out, { recursive: true });

const bin = resolveBinary();
if (!bin) {
  console.error(`smoke: no desktop binary found (looked in ${path.resolve(args.dir ?? path.join("target", "release"))})`);
  process.exit(2);
}

const home = mkdtempSync(path.join(tmpdir(), "skill-hive-smoke-"));
const logFile = path.join(home, "logs", "skill-hive.log");
const crashFile = path.join(home, "logs", "skill-hive-crash.log");
const env = { ...process.env, HIVE_HOME: home, SKILL_HIVE_HEADLESS: "1", RUST_LOG: "info,hive_core=debug" };
if (platform === "win32") env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = `--remote-debugging-port=${cdpPort}`;

console.log(`smoke: launching ${bin}`);
console.log(`smoke: home ${home}`);
const child = spawn(bin, [], { env, stdio: ["ignore", "pipe", "pipe"] });
let output = "";
child.stdout.on("data", (d) => (output += d));
child.stderr.on("data", (d) => (output += d));
let exited = null;
child.on("exit", (code, signal) => (exited = { code, signal }));
child.on("error", (err) => (exited = { error: String(err) }));

const report = {
  ok: false,
  platform,
  bin,
  home,
  ready: null,
  pageLoads: [],
  uiErrors: [],
  desktopScreenshot: null,
  devtools: null,
  console: [],
  page: null,
  exit: null,
  error: null,
};
const started = Date.now();
let devtools = null;
if (platform === "win32") {
  devtools = attachDevTools();
  devtools.catch(() => {}); // reported through the report, never unhandled
}

try {
  for (;;) {
    const log = readText(logFile);
    report.pageLoads = [...log.matchAll(/page load.*?url=(\S+).*?event=(\w+)/g)].map((m) => `${m[2]} ${m[1]}`);
    report.uiErrors = log.split("\n").filter((l) => /ERROR.*ui: /.test(l));
    if (report.uiErrors.length) throw new Error(`the interface reported a boot failure:\n${report.uiErrors.join("\n")}`);
    const ready = log.match(/ui: ready (v\S+)/);
    if (ready) {
      report.ready = ready[1];
      break;
    }
    if (exited) {
      const crash = readText(crashFile);
      throw new Error(`the process exited before the interface was ready: ${JSON.stringify(exited)}${crash ? `\ncrash file:\n${crash}` : ""}`);
    }
    if (Date.now() - started > timeoutMs) throw new Error(`the interface did not report ready within ${timeoutMs / 1000}s`);
    await sleep(500);
  }
  console.log(`smoke: interface ready (${report.ready}) after ${((Date.now() - started) / 1000).toFixed(1)}s`);
  await sleep(1500); // let the first paint settle before the pictures
  report.desktopScreenshot = captureDesktop();

  if (devtools) {
    try {
      const session = await withTimeout(devtools, 10_000, "WebView2 did not expose its DevTools port");
      report.devtools = "attached";
      report.page = await session.inspect();
      if (report.page.appReady === null) throw new Error("data-app-ready is missing on <html> although the log says ready");
      if (report.page.bootError) throw new Error(`boot error shown in the page:\n${report.page.bootError}`);
      if (!/Home/.test(report.page.text)) throw new Error(`the page text does not look like the app:\n${report.page.text.slice(0, 400)}`);
      console.log(`smoke: page shows "${report.page.text.split("\n").slice(0, 6).join(" | ")}"`);
    } catch (err) {
      if (report.devtools === "attached" || requireDevtools) throw err;
      report.devtools = `unavailable: ${String(err?.message ?? err)}`;
      console.log(`smoke: ${report.devtools}; page checks skipped, the log is the verdict`);
    }
  }
  report.ok = true;
} catch (err) {
  report.error = String(err?.message ?? err);
  if (!report.desktopScreenshot) report.desktopScreenshot = captureDesktop();
  if (devtools && report.page === null) {
    // Best effort: whatever the page looks like at the moment of failure.
    try {
      const session = await withTimeout(devtools, 3_000, "DevTools did not attach");
      report.page = await session.inspect();
    } catch {
      /* nothing more to learn from the page */
    }
  }
} finally {
  if (devtools) {
    try {
      report.console = (await withTimeout(devtools, 500, "")).messages;
    } catch {
      /* never attached */
    }
  }
  await stop(child);
  report.exit = exited;
  if (existsSync(logFile)) copyFileSync(logFile, path.join(out, "skill-hive.log"));
  if (existsSync(crashFile)) copyFileSync(crashFile, path.join(out, "skill-hive-crash.log"));
  if (output) writeFileSync(path.join(out, "process-output.txt"), output);
  writeFileSync(path.join(out, "report.json"), JSON.stringify(report, null, 2));
}

if (report.ok) {
  console.log(`smoke: OK (${report.pageLoads.length} page load events, report in ${out})`);
  process.exit(0);
}
console.error(`smoke: FAILED: ${report.error}`);
const log = readText(logFile);
if (log) console.error(`--- last log lines\n${log.trim().split("\n").slice(-40).join("\n")}`);
if (report.console.length) console.error(`--- console\n${report.console.slice(-20).join("\n")}`);
if (output) console.error(`--- process output\n${output.trim().split("\n").slice(-20).join("\n")}`);
console.error(`--- report and logs in ${out}`);
process.exit(1);

// ---- helpers ----------------------------------------------------------------------------------

function resolveBinary() {
  if (args.bin) return existsSync(args.bin) ? path.resolve(args.bin) : null;
  const dir = path.resolve(args.dir ?? path.join("target", "release"));
  const names = platform === "win32" ? ["Skill Hive.exe", "skill-hive.exe"] : ["Skill Hive", "skill-hive"];
  const found = names
    .map((n) => path.join(dir, n))
    .filter((p) => existsSync(p) && statSync(p).isFile())
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  return found[0] ?? null;
}

/** A picture of the whole desktop with the OS's own tools; returns the file or null. */
function captureDesktop() {
  const file = path.join(out, "desktop.png");
  try {
    if (platform === "win32") {
      const script = path.join(out, "capture.ps1");
      writeFileSync(
        script,
        [
          "Add-Type -AssemblyName System.Windows.Forms,System.Drawing",
          "$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds",
          "$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height",
          "$g = [System.Drawing.Graphics]::FromImage($bmp)",
          "$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)",
          `$bmp.Save('${file.replace(/'/g, "''")}')`,
        ].join("\n"),
      );
      execSync(`powershell -NoProfile -ExecutionPolicy Bypass -File "${script}"`, { stdio: "ignore", timeout: 30_000 });
    } else if (platform === "darwin") {
      execSync(`screencapture -x "${file}"`, { stdio: "ignore", timeout: 30_000 });
    } else {
      execSync(`import -window root "${file}"`, { stdio: "ignore", timeout: 30_000 });
    }
    return existsSync(file) ? file : null;
  } catch {
    return null;
  }
}

/** Connects to the WebView2 DevTools endpoint as soon as it answers and records console output. */
async function attachDevTools() {
  const targets = await pollJson(`http://127.0.0.1:${cdpPort}/json`, timeoutMs);
  const page = targets.find((t) => t.type === "page" && /tauri\.localhost/.test(t.url)) ?? targets.find((t) => t.type === "page");
  if (!page) throw new Error(`no page target on the DevTools port: ${JSON.stringify(targets)}`);
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve, { once: true });
    ws.addEventListener("error", () => reject(new Error("DevTools websocket failed")), { once: true });
  });
  const messages = [];
  let nextId = 1;
  const pending = new Map();
  ws.addEventListener("message", (event) => {
    const msg = JSON.parse(String(event.data));
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
      return;
    }
    if (msg.method === "Runtime.consoleAPICalled" && /error|warning|assert/.test(msg.params.type)) {
      messages.push(`${msg.params.type}: ${msg.params.args.map((a) => a.value ?? a.description ?? a.type).join(" ")}`);
    } else if (msg.method === "Runtime.exceptionThrown") {
      const d = msg.params.exceptionDetails;
      messages.push(`exception: ${d.exception?.description ?? d.text}`);
    } else if (msg.method === "Log.entryAdded" && /error|warning/.test(msg.params.entry.level)) {
      messages.push(`${msg.params.entry.source} ${msg.params.entry.level}: ${msg.params.entry.text} ${msg.params.entry.url ?? ""}`);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = nextId++;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
      setTimeout(() => {
        if (pending.delete(id)) reject(new Error(`${method} timed out`));
      }, 15_000);
    });
  await send("Runtime.enable");
  await send("Log.enable");
  await send("Page.enable");
  console.log(`smoke: DevTools attached to ${page.url}`);
  const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, returnByValue: true })).result?.value ?? null;
  return {
    messages,
    async inspect() {
      const result = {
        url: await evaluate("location.href"),
        appReady: await evaluate("document.documentElement.getAttribute('data-app-ready')"),
        bootPlaceholder: Boolean(await evaluate("Boolean(document.querySelector('[data-boot]'))")),
        bootError: (await evaluate("document.querySelector('[data-boot-error]')?.textContent ?? ''")) || "",
        text: String((await evaluate("document.body.innerText.slice(0, 2000)")) ?? ""),
      };
      try {
        const shot = await send("Page.captureScreenshot", { format: "png" });
        writeFileSync(path.join(out, "window.png"), Buffer.from(shot.data, "base64"));
      } catch (err) {
        result.screenshotError = String(err?.message ?? err);
      }
      return result;
    },
  };
}

async function pollJson(url, ms) {
  const until = Date.now() + ms;
  let lastError = "";
  while (Date.now() < until) {
    if (exited) throw new Error(`process exited before ${url} answered`);
    try {
      const res = await fetch(url);
      if (res.ok) return await res.json();
      lastError = `HTTP ${res.status}`;
    } catch (err) {
      lastError = String(err?.message ?? err);
    }
    await sleep(500);
  }
  throw new Error(`${url} did not answer within ${ms / 1000}s: ${lastError}`);
}

function withTimeout(promise, ms, message) {
  return Promise.race([promise, new Promise((_, reject) => setTimeout(() => reject(new Error(message)), ms))]);
}

async function stop(proc) {
  if (exited) return;
  if (platform === "win32") {
    try {
      execSync(`taskkill /PID ${proc.pid} /T /F`, { stdio: "ignore" });
    } catch {
      proc.kill();
    }
  } else {
    proc.kill("SIGTERM");
    await sleep(2000);
    if (!exited) proc.kill("SIGKILL");
  }
  await sleep(500);
}

function readText(file) {
  try {
    return readFileSync(file, "utf8");
  } catch {
    return "";
  }
}

function parseArgs(argv) {
  const parsed = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) parsed[a.slice(2)] = argv[i + 1] && !argv[i + 1].startsWith("--") ? argv[++i] : "true";
  }
  return parsed;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
