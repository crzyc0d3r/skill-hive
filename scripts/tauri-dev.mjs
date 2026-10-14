import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { killProcessTree, packageBin, spawnNodeScript } from "./spawn-tool.mjs";

export function tauriDevCommand() {
  return {
    file: process.execPath,
    args: [packageBin("@tauri-apps/cli", "tauri"), "dev", "--config", "src-tauri/tauri.dev.conf.json"],
    shell: false,
  };
}

function isDirectRun() {
  const entry = process.argv[1];
  if (!entry) return false;
  return resolve(entry) === resolve(fileURLToPath(import.meta.url));
}

function finish(code) {
  if (finish.settled) return;
  finish.settled = true;
  process.exit(code);
}
finish.settled = false;

if (isDirectRun()) {
  const base = resolve(process.env.SKILL_HIVE_DEV_BASE ?? ".dev-base");
  mkdirSync(base, { recursive: true });
  const command = tauriDevCommand();
  const probe = process.env.SKILL_HIVE_TAURI_DEV_PROBE === "1";
  const child = spawnNodeScript(command.args[0], command.args.slice(1), {
    stdio: "inherit",
    env: { ...process.env, SKILL_HIVE_BASE: base },
    detached: probe && process.platform !== "win32",
  });
  let spawned = false;
  child.on("spawn", () => {
    spawned = true;
  });
  child.on("error", (error) => {
    console.error(error.message);
    finish(1);
  });
  if (!probe) {
    child.on("exit", (code) => finish(code ?? 1));
  } else {
    const timer = setTimeout(() => {
      if (!spawned) {
        console.error("tauri dev failed to spawn");
        finish(1);
        return;
      }
      console.log("tauri dev spawned");
      killProcessTree(child);
      finish(0);
    }, 15000);
    child.on("exit", (code) => {
      clearTimeout(timer);
      if (!spawned) finish(code ?? 1);
      else finish(code === 0 ? 0 : 1);
    });
  }
}
