import fs from "node:fs";
import path from "node:path";
import { applyBundle, decideSigning } from "./signing-mode.mjs";

const planOnly = process.argv.includes("--plan-only");
const configPath = process.argv.includes("--config")
  ? process.argv[process.argv.indexOf("--config") + 1]
  : "src-tauri/tauri.conf.json";

const decision = decideSigning(process.env);
if (!decision.ok) {
  console.error("signing secrets are incomplete; refusing to build so tauri-action cannot ad-hoc sign");
  for (const item of decision.partials) console.error(`missing ${item}`);
  process.exit(1);
}

const lines = [
  `apple=${decision.apple.mode}`,
  `apple_path=${decision.apple.path}`,
  `windows=${decision.windows.mode}`,
  `windows_path=${decision.windows.path}`,
  `updater=${decision.updater.mode}`,
];

if (planOnly) {
  console.log(lines.join("\n"));
  if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `${lines.join("\n")}\n`);
  process.exit(0);
}

const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
const next = applyBundle(config, decision);
fs.writeFileSync(configPath, `${JSON.stringify(next, null, 2)}\n`);
console.log(`updater artifacts: ${next.bundle.createUpdaterArtifacts ? "signed" : "unsigned"}`);
console.log(`apple: ${decision.apple.mode} (${decision.apple.path})`);
console.log(`windows: ${decision.windows.mode} (${decision.windows.path})`);

if (decision.apple.mode === "sign" && decision.apple.path === "api-key") {
  const dir = process.env.RUNNER_TEMP || process.cwd();
  const keyPath = path.join(dir, "AuthKey.p8");
  fs.writeFileSync(keyPath, process.env.APPLE_API_PRIVATE_KEY.trim() + "\n", { mode: 0o600 });
  const envLine = `APPLE_API_KEY_PATH=${keyPath}`;
  console.log(envLine);
  if (process.env.GITHUB_ENV) fs.appendFileSync(process.env.GITHUB_ENV, `${envLine}\n`);
}

if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `${lines.join("\n")}\n`);
