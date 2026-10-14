import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { spawnCommand } from "./spawn-tool.mjs";
import { tmpdir } from "node:os";
import path from "node:path";

const application = process.argv[2];
if (!application) {
  console.error("usage: node scripts/tauri-driver-suite.mjs <skill-hive-binary>");
  process.exit(2);
}

const base = mkdtempSync(path.join(tmpdir(), "skill-hive-driver-"));
const home = path.join(base, "home");
process.env.SKILL_HIVE_BASE = base;
process.env.HOME = home;
process.env.USERPROFILE = home;
process.env.XDG_CONFIG_HOME = path.join(base, "xdg-config");
process.env.XDG_DATA_HOME = path.join(base, "xdg-data");
process.env.XDG_CACHE_HOME = path.join(base, "xdg-cache");
for (const dir of [process.env.XDG_CONFIG_HOME, process.env.XDG_DATA_HOME, process.env.XDG_CACHE_HOME]) {
  mkdirSync(dir, { recursive: true });
}
const seededClaude = path.join(home, ".claude", "skills", "seeded-claude");
mkdirSync(seededClaude, { recursive: true });
writeFileSync(path.join(seededClaude, "SKILL.md"), "---\nname: seeded-claude\ndescription: seeded\n---\nSeeded body\n");
const localSkill = path.join(base, "local-skill");
mkdirSync(localSkill, { recursive: true });
writeFileSync(path.join(localSkill, "SKILL.md"), "---\nname: local-skill\ndescription: local\n---\nLocal body\n");
const gitWork = path.join(base, "git-work");
const gitSkill = path.join(gitWork, "seeded-git");
mkdirSync(gitSkill, { recursive: true });
writeFileSync(path.join(gitSkill, "SKILL.md"), "---\nname: seeded-git\ndescription: from git\n---\nGit body\n");
function git(dir, args) {
  execFileSync("git", args, { cwd: dir, stdio: "inherit" });
}
git(gitWork, ["init", "-b", "main"]);
git(gitWork, ["config", "user.email", "hive@localhost"]);
git(gitWork, ["config", "user.name", "Skill Hive"]);
git(gitWork, ["add", "."]);
git(gitWork, ["commit", "-m", "seed"]);
const bare = path.join(base, "seeded.git");
execFileSync("git", ["clone", "--bare", gitWork, bare], { stdio: "inherit" });
process.env.SKILL_HIVE_CONFIG_DIR = path.join(base, "config");
const webviewProfile = path.join(base, "webview-profile");
mkdirSync(webviewProfile, { recursive: true });
process.env.SKILL_HIVE_WEBVIEW_DATA_DIR = webviewProfile;
process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS =
  "--remote-debugging-port=9222 --remote-allow-origins=*";

const driverArgs = process.env.TAURI_NATIVE_DRIVER
  ? ["--native-driver", process.env.TAURI_NATIVE_DRIVER]
  : [];
const driver = spawnCommand("tauri-driver", driverArgs, { stdio: "inherit" });
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
  let prefix = `/session/${session}`;

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

  async function find(selector, using = "css selector") {
    const deadline = Date.now() + 10000;
    let last = `missing ${selector}`;
    while (Date.now() < deadline) {
      try {
        const found = await call("POST", `${prefix}/element`, { using, value: selector });
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

  function skillNames() {
    const db = path.join(base, "hive.db");
    const script = `import sqlite3; c=sqlite3.connect(${JSON.stringify(db)}); print("\\n".join(row[0] for row in c.execute("select name from skills")))`;
    return execFileSync("python3", ["-c", script], { encoding: "utf8" });
  }
  function setting(key) {
    const db = path.join(base, "hive.db");
    const script = `import sqlite3; c=sqlite3.connect(${JSON.stringify(db)}); row=c.execute("select value from settings where key=?", (${JSON.stringify(key)},)).fetchone(); print(row[0] if row else "")`;
    return execFileSync("python3", ["-c", script], { encoding: "utf8" }).trim();
  }
  async function clickText(text, last = false) {
    const quoted = JSON.stringify(text);
    const expression = last ? `(//button[normalize-space()=${quoted}])[last()]` : `//button[normalize-space()=${quoted}]`;
    const id = await find(expression, "xpath");
    await call("POST", `${prefix}/element/${id}/click`, {});
  }
  async function typeInto(selector, value) {
    const id = await find(selector);
    await call("POST", `${prefix}/element/${id}/clear`, {});
    await call("POST", `${prefix}/element/${id}/value`, { text: value, value: [...value] });
  }
  async function restart() {
    await fetch(`http://127.0.0.1:4444/session/${session}`, { method: "DELETE" }).catch(() => {});
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
    prefix = `/session/${session}`;
    await find("[data-testid=sidebar]");
  }

  await click("a[href='/install']");
  await click("[data-testid=tab-local]");
  await typeInto('input[aria-label="Select folder"]', localSkill);
  await clickText("Install local");
  const installedDeadline = Date.now() + 15000;
  while (!skillNames().includes("local-skill")) {
    if (Date.now() > installedDeadline) throw new Error(`local-skill missing from hive.db\n${skillNames()}`);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const copied = path.join(base, "skills");
  if (!existsSync(copied)) throw new Error(`skills directory missing at ${copied}`);
  await restart();
  await click("a[href='/my-skills']");
  const library = await call("GET", `${prefix}/source`);
  if (!String(library.value ?? "").includes("local-skill")) throw new Error("local-skill was not listed after restart");

  await click("a[href='/install']");
  await click("[data-testid=tab-scan]");
  await clickText("Import seeded-claude");
  const scanDeadline = Date.now() + 15000;
  while (!skillNames().includes("seeded-claude")) {
    if (Date.now() > scanDeadline) throw new Error(`seeded-claude missing from hive.db\n${skillNames()}`);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  await click("a[href='/my-skills']");
  await click('input[aria-label="Select local-skill"]');
  await clickText("Delete selected");
  const deleteDeadline = Date.now() + 15000;
  while (skillNames().includes("local-skill")) {
    if (Date.now() > deleteDeadline) throw new Error("local-skill was not deleted");
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  await click("a[href='/settings']");
  await find("[data-testid=settings]");
  const agentTabs = await call("POST", `${prefix}/elements`, { using: "css selector", value: 'button[role="tab"]' });
  const tabIds = agentTabs.value ?? [];
  let agentsTab = null;
  for (const entry of tabIds) {
    const id = entry["element-6066-11e4-a52e-4f735466cecf"] ?? Object.values(entry)[0];
    const text = await call("GET", `${prefix}/element/${id}/text`);
    if (String(text.value ?? "").trim() === "Agents") agentsTab = id;
  }
  if (!agentsTab) throw new Error("Agents settings tab was not found");
  await call("POST", `${prefix}/element/${agentsTab}/click`, {});
  await typeInto('input[aria-label="Agent name"]', "Field Notes");
  await typeInto('input[aria-label="Agent path"]', path.join(home, ".field-notes", "skills"));
  await clickText("New Agent");
  const agentDeadline = Date.now() + 15000;
  while (!setting("custom_tools").includes("Field Notes")) {
    if (Date.now() > agentDeadline) throw new Error(`custom agent missing\n${setting("custom_tools")}`);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  await click("a[href='/install']");
  await click("[data-testid=tab-git]");
  await typeInto('input[aria-label="Git URL"]', bare);
  await clickText("Clone & Preview");
  await find("[data-testid=git-preview]");
  await clickText("Import Selected");
  const gitDeadline = Date.now() + 20000;
  while (!skillNames().includes("seeded-git")) {
    if (Date.now() > gitDeadline) throw new Error(`seeded-git missing from hive.db\n${skillNames()}`);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  const skillFile = readdirSync(path.join(base, "skills"), { recursive: true })
    .map((entry) => path.join(base, "skills", String(entry)))
    .find((entry) => entry.endsWith(`${path.sep}SKILL.md`) && entry.includes("seeded-git"));
  if (!skillFile) throw new Error("seeded-git SKILL.md was not copied");
  const original = readFileSync(skillFile, "utf8");
  await click("a[href='/backup']");
  await click("[data-testid=backup-now]");
  await find("[data-testid=backup-restore]");
  writeFileSync(skillFile, `${original}\nchanged\n`);
  await click("[data-testid=backup-restore]");
  const restoreDeadline = Date.now() + 15000;
  while (readFileSync(skillFile, "utf8") !== original) {
    if (Date.now() > restoreDeadline) throw new Error("backup restore did not return the skill file");
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  await click("a[href='/my-skills']");
  await click('input[aria-label="Select seeded-git"]');
  await clickText("Sync to agents");
  await click('input[name="codex"]');
  await clickText("Sync to agents", true);
  const synced = path.join(home, ".codex", "skills", "seeded-git");
  const syncDeadline = Date.now() + 15000;
  while (!existsSync(synced)) {
    if (Date.now() > syncDeadline) throw new Error(`synced skill missing at ${synced}`);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  console.log("tauri-driver suite: shell routes plus install, scan, delete, sync, agent, git, and backup");
} finally {
  if (promoter) clearInterval(promoter);
  if (session) {
    await fetch(`http://127.0.0.1:4444/session/${session}`, { method: "DELETE" }).catch(() => {});
  }
  driver.kill("SIGTERM");
}
