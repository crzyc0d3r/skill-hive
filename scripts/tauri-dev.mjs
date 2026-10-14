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

export const DEV_PROBE_HOLD_MS = 25_000;
export const DEV_PROBE_LIMIT_MS = 8 * 60 * 1000;

export function createDevProbeState(startedAt) {
  return {
    busy: false,
    compiling: false,
    devCommand: false,
    finished: false,
    compileAt: 0,
    devAt: 0,
    startedAt,
  };
}

export function absorbDevLog(state, text, now) {
  if (text.includes("EBUSY")) state.busy = true;
  if (text.includes("Running DevCommand")) {
    state.devCommand = true;
    if (!state.devAt) state.devAt = now;
  }
  if (text.includes("Compiling")) {
    state.compiling = true;
    if (!state.compileAt) state.compileAt = now;
  }
  if (text.includes("Finished")) state.finished = true;
  return state;
}

export function devProbeDecision(state, now) {
  if (state.busy) return "fail";
  if (now - state.startedAt >= DEV_PROBE_LIMIT_MS) return "fail";
  if (state.compiling && now - state.compileAt >= DEV_PROBE_HOLD_MS) return "ok";
  if (state.devCommand && state.finished && !state.compiling && now - state.devAt >= 10_000) return "ok";
  return "wait";
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
    stdio: probe ? ["ignore", "pipe", "pipe"] : "inherit",
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
    const state = createDevProbeState(Date.now());
    const onChunk = (chunk) => {
      const text = chunk.toString();
      process.stdout.write(text);
      absorbDevLog(state, text, Date.now());
    };
    child.stdout.on("data", onChunk);
    child.stderr.on("data", onChunk);
    const timer = setInterval(() => {
      const decision = devProbeDecision(state, Date.now());
      if (!spawned) {
        if (decision === "fail") {
          clearInterval(timer);
          console.error("tauri dev failed to spawn");
          finish(1);
        }
        return;
      }
      if (decision === "wait") return;
      clearInterval(timer);
      if (decision === "fail") {
        console.error(state.busy ? "vite watcher failed with EBUSY while cargo was compiling" : "tauri dev did not reach cargo compile");
        killProcessTree(child);
        finish(1);
        return;
      }
      console.log("tauri dev stayed up while cargo was compiling");
      killProcessTree(child);
      finish(0);
    }, 500);
    child.on("exit", (code) => {
      clearInterval(timer);
      if (!spawned || code !== 0 || devProbeDecision(state, Date.now()) === "fail") finish(code === 0 ? 1 : (code ?? 1));
      else finish(0);
    });
  }
}
