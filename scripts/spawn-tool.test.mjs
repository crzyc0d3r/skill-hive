import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import test from "node:test";
import { commandLaunch, packageBin, spawnCommandSync } from "./spawn-tool.mjs";
import { tauriDevCommand } from "./tauri-dev.mjs";

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
