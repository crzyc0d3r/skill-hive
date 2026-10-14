import { spawn, spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";

const require = createRequire(import.meta.url);

const CMD_TOOLS = new Set(["npx", "npm", "yarn", "pnpm", "corepack"]);

export function commandLaunch(command) {
  if (process.platform !== "win32") return { file: command, shell: false };
  const base = command.replace(/^.*[\\/]/, "").toLowerCase();
  if (base === "node" || base === "nodejs") return { file: process.execPath, shell: false };
  if (CMD_TOOLS.has(base)) return { file: `${base}.cmd`, shell: true };
  if (base.endsWith(".cmd") || base.endsWith(".bat")) return { file: command, shell: true };
  return { file: command, shell: false };
}

export function packageBin(packageName, binName) {
  const manifestPath = require.resolve(`${packageName}/package.json`);
  const manifest = require(manifestPath);
  const bins = manifest.bin;
  const relative = typeof bins === "string" ? bins : bins?.[binName];
  if (!relative) throw new Error(`${packageName} has no bin named ${binName}`);
  return resolve(dirname(manifestPath), relative);
}

export function spawnCommand(command, args, options = {}) {
  const launch = commandLaunch(command);
  return spawn(launch.file, args, { ...options, shell: options.shell ?? launch.shell });
}

export function spawnCommandSync(command, args, options = {}) {
  const launch = commandLaunch(command);
  return spawnSync(launch.file, args, { ...options, shell: options.shell ?? launch.shell });
}

export function spawnNodeScript(scriptPath, args, options = {}) {
  return spawn(process.execPath, [scriptPath, ...args], { ...options, shell: false });
}

export function killProcessTree(child) {
  if (!child.pid) return;
  if (process.platform === "win32") {
    spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"], { stdio: "ignore" });
    return;
  }
  try {
    process.kill(-child.pid, "SIGTERM");
  } catch {
    child.kill("SIGTERM");
  }
}
