import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { commandLaunch, packageBin, spawnCommandSync } from "./spawn-tool.mjs";
import { absorbDevLog, createDevProbeState, DEV_PROBE_HOLD_MS, DEV_PROBE_LIMIT_MS, devProbeDecision, tauriDevCommand } from "./tauri-dev.mjs";

test("tauri dev launches the local CLI with node, not npx", () => {
  const command = tauriDevCommand();
  assert.equal(command.shell, false);
  assert.equal(command.file, process.execPath);
  assert.equal(command.args[1], "dev");
  assert.equal(command.args[2], "--config");
  assert.equal(command.args[3], "src-tauri/tauri.dev.conf.json");
  assert.ok(command.args[0].endsWith("tauri.js"));
  assert.equal(existsSync(command.args[0]), true);
  assert.equal(command.args.includes("npx"), false);
});

test("the local tauri CLI answers --help", () => {
  const bin = packageBin("@tauri-apps/cli", "tauri");
  const result = spawnCommandSync(process.execPath, [bin, "--help"], { encoding: "utf8" });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0);
  assert.match(`${result.stdout}`, /Usage/i);
});

test("windows cmd shims are spawned through the shell", () => {
  const npx = commandLaunch("npx");
  const npm = commandLaunch("npm");
  const node = commandLaunch("node");
  if (process.platform === "win32") {
    assert.equal(npx.file, "npx.cmd");
    assert.equal(npx.shell, true);
    assert.equal(npm.file, "npm.cmd");
    assert.equal(npm.shell, true);
    assert.equal(node.file, process.execPath);
    assert.equal(node.shell, false);
  } else {
    assert.equal(npx.file, "npx");
    assert.equal(npx.shell, false);
    assert.equal(npm.shell, false);
    assert.equal(node.file, "node");
    assert.equal(node.shell, false);
  }
});

test("vite ignores cargo output and other non-frontend directories", () => {
  const source = readFileSync(new URL("../vite.config.ts", import.meta.url), "utf8");
  for (const pattern of [
    "**/target/**",
    "**/src-tauri/target/**",
    "crates/**",
    "**/crates/**",
    "graphify-out/**",
    "**/graphify-out/**",
    "automations/**",
    "**/automations/**",
  ]) {
    assert.equal(source.includes(`"${pattern}"`), true, pattern);
  }
});

test("the dev probe stays up through cargo compile and fails on EBUSY", () => {
  const started = 1_000;
  const state = createDevProbeState(started);
  absorbDevLog(state, "Running DevCommand (`cargo run`)", started + 100);
  absorbDevLog(state, "\u001b[1m\u001b[92m   Compiling\u001b[0m hive-core", started + 200);
  assert.equal(devProbeDecision(state, started + 200 + DEV_PROBE_HOLD_MS - 1), "wait");
  assert.equal(devProbeDecision(state, started + 200 + DEV_PROBE_HOLD_MS), "ok");
  absorbDevLog(state, "EBUSY: resource busy or locked, watch 'E:\\\\code\\\\skill-hive\\\\target\\\\debug\\\\build\\\\build_script_build-1.exe'", started + 300);
  assert.equal(devProbeDecision(state, started + 200 + DEV_PROBE_HOLD_MS), "fail");

  const cached = createDevProbeState(started);
  absorbDevLog(cached, "Running DevCommand (`cargo run`)", started + 50);
  absorbDevLog(cached, "Finished `dev` profile", started + 80);
  assert.equal(devProbeDecision(cached, started + 50 + 9_000), "wait");
  assert.equal(devProbeDecision(cached, started + 50 + 10_000), "ok");

  const stalled = createDevProbeState(started);
  assert.equal(devProbeDecision(stalled, started + DEV_PROBE_LIMIT_MS), "fail");
});
