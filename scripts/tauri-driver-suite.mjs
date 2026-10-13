import { spawn } from "node:child_process";
import { mkdtempSync } from "node:fs";
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

const driver = spawn("tauri-driver", [], { stdio: "inherit" });
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

let session;
try {
  const created = await call("POST", "/session", {
    capabilities: {
      alwaysMatch: {
        browserName: "wry",
        "tauri:options": { application: path.resolve(application) },
      },
    },
  });
  session = created.value?.sessionId ?? created.sessionId;
  if (!session) throw new Error(`no session id in ${JSON.stringify(created)}`);
  const prefix = `/session/${session}`;

  async function find(selector) {
    const found = await call("POST", `${prefix}/element`, { using: "css selector", value: selector });
    const id = found.value?.["element-6066-11e4-a52e-4f735466cecf"] ?? Object.values(found.value ?? {})[0];
    if (!id) throw new Error(`missing ${selector}: ${JSON.stringify(found)}`);
    return id;
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
  console.log("tauri-driver suite: phase 1 routes plus project, marketplace, update, and backup");
} finally {
  if (session) {
    await fetch(`http://127.0.0.1:4444/session/${session}`, { method: "DELETE" }).catch(() => {});
  }
  driver.kill("SIGTERM");
}
