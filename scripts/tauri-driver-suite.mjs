import { spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const application = process.argv[2];
if (!application) {
  console.error("usage: node scripts/tauri-driver-suite.mjs <skill-hive-binary>");
  process.exit(2);
}

const base = mkdtempSync(path.join(tmpdir(), "skill-hive-driver-"));
process.env.SKILL_HIVE_BASE = base;
process.env.SKILL_HIVE_CONFIG_DIR = path.join(base, "config");
const webviewProfile = path.join(base, "webview-profile");
mkdirSync(webviewProfile, { recursive: true });
process.env.SKILL_HIVE_WEBVIEW_DATA_DIR = webviewProfile;
process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS =
  "--remote-debugging-port=9222 --remote-allow-origins=*";

const driverArgs = process.env.TAURI_NATIVE_DRIVER
  ? ["--native-driver", process.env.TAURI_NATIVE_DRIVER]
  : [];
const driver = spawn("tauri-driver", driverArgs, { stdio: "inherit" });
await new Promise((resolve) => setTimeout(resolve, 1000));

async function call(method, urlPath, body) {
  const response = await fetch(`http://127.0.0.1:4444${urlPath}`, {
    method,
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  if (!response.ok) throw new Error(`${method} ${urlPath} ${response.status} ${text}`);
  return text ? JSON.parse(text) : {};
}

function promotePortFile(nested) {
  const parent = path.dirname(nested);
  if (path.basename(parent).toLowerCase() !== "ebwebview") return;
  const dest = path.join(path.dirname(parent), "DevToolsActivePort");
  if (existsSync(dest)) return;
  try {
    copyFileSync(nested, dest);
    console.log(`promoted DevToolsActivePort from ${nested}`);
  } catch (err) {
    console.log(`could not promote DevToolsActivePort: ${err}`);
  }
}

function walkForPort(dir, depth, found) {
  if (depth < 0) return;
  let entries = [];
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const full = path.join(dir, entry.name);
    if (entry.name.toLowerCase() === "ebwebview") {
      const nested = path.join(full, "DevToolsActivePort");
      if (existsSync(nested)) {
        found.push(nested);
        promotePortFile(nested);
      }
      continue;
    }
    walkForPort(full, depth - 1, found);
  }
}

function promoteDevToolsPort() {
  promoteDevToolsPort.scans += 1;
  const roots = [
    process.env.SKILL_HIVE_WEBVIEW_DATA_DIR,
    process.env.TEMP,
    process.env.TMP,
    process.env.LOCALAPPDATA && path.win32.join(process.env.LOCALAPPDATA, "Temp"),
    tmpdir(),
  ].filter(Boolean);
  const seen = new Set();
  const found = [];
  for (const root of roots) {
    if (seen.has(root)) continue;
    seen.add(root);
    walkForPort(root, 4, found);
  }
  if (promoteDevToolsPort.scans === 1 || promoteDevToolsPort.scans % 20 === 0 || found.length) {
    console.log(
      `devtools scan ${promoteDevToolsPort.scans} roots ${[...seen].join(" ; ")} ports ${found.join(" ; ") || "(none)"}`,
    );
  }
}
promoteDevToolsPort.scans = 0;

let session;
const promoter = process.platform === "win32" ? setInterval(promoteDevToolsPort, 200) : null;
try {
  const created = await call("POST", "/session", {
    capabilities: {
      alwaysMatch: {
        browserName: "wry",
        "tauri:options": {
          application: path.resolve(application),
          webviewOptions: {
            userDataFolder: webviewProfile,
            additionalBrowserArguments: ["--remote-debugging-port=9222", "--remote-allow-origins=*"],
          },
        },
      },
    },
  });
  session = created.value?.sessionId ?? created.sessionId;
  if (!session) throw new Error(`no session id in ${JSON.stringify(created)}`);
  const prefix = `/session/${session}`;

  const deadline = Date.now() + 30000;
  let last = "no window";
  let attached = false;
  while (Date.now() < deadline) {
    const handles = await call("GET", `${prefix}/window/handles`);
    const windows = handles.value ?? [];
    for (const handle of windows) {
      await call("POST", `${prefix}/window`, { handle });
      const url = await call("GET", `${prefix}/url`);
      const source = await call("GET", `${prefix}/source`);
      const html = String(source.value ?? "");
      last = `windows ${JSON.stringify(windows)} url ${url.value} source ${html.slice(0, 240).replace(/\s+/g, " ")}`;
      if (html.includes('data-testid="sidebar"') || html.includes("data-testid='sidebar'")) {
        attached = true;
        break;
      }
    }
    if (attached) break;
    console.log(last);
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  if (!attached) throw new Error(`sidebar did not appear\n${last}`);
  console.log(`attached ${last}`);

  async function find(selector) {
    const deadline = Date.now() + 10000;
    let last = `missing ${selector}`;
    while (Date.now() < deadline) {
      try {
        const found = await call("POST", `${prefix}/element`, { using: "css selector", value: selector });
        const id = found.value?.["element-6066-11e4-a52e-4f735466cecf"] ?? Object.values(found.value ?? {})[0];
        if (id) return id;
        last = `missing ${selector}: ${JSON.stringify(found)}`;
      } catch (err) {
        last = err.message;
      }
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    const url = await call("GET", `${prefix}/url`).catch(() => ({}));
    const source = await call("GET", `${prefix}/source`).catch(() => ({}));
    const html = String(source.value ?? "")
      .replace(/<style[\s\S]*?<\/style>/gi, " ")
      .replace(/<script[\s\S]*?<\/script>/gi, " ")
      .replace(/\s+/g, " ");
    const bodyAt = html.indexOf("<body");
    const slice = (bodyAt >= 0 ? html.slice(bodyAt) : html).slice(0, 1200);
    throw new Error(`${selector} not found\n${last}\nurl ${url.value}\nsource ${slice}`);
  }

  async function click(selector) {
    const id = await find(selector);
    await call("POST", `${prefix}/element/${id}/click`, {});
  }

  await find("[data-testid=sidebar]");
  await click("a[href='/']");
  await find("[data-testid=dashboard]");
  await click("a[href='/my-skills']");
  await find("[data-testid=library]");
  await click("[data-testid=check-all]");
  await click("[data-testid=update-anyway]");
  await click("a[href='/project/fixture']");
  await click("[data-testid=link-project]");
  await find("[data-testid=project-dot]");
  await click("a[href='/install']");
  await click("[data-testid=tab-leaderboard]");
  await find("[data-testid=marketplace]");
  await click("a[href='/backup']");
  await click("[data-testid=backup-now]");
  await click("[data-testid=backup-restore]");
  await find("[data-testid=backup]");
  console.log("tauri-driver suite: shell routes plus project, marketplace, update, and backup");
} finally {
  if (promoter) clearInterval(promoter);
  if (session) {
    await fetch(`http://127.0.0.1:4444/session/${session}`, { method: "DELETE" }).catch(() => {});
  }
  driver.kill("SIGTERM");
}
