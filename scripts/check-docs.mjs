import fs from "node:fs";

const guide = fs.readFileSync("docs/user-guide.md", "utf8");
const headings = [
  "## Dashboard",
  "## Library",
  "## Install Skills",
  "## Global Workspace",
  "## Lobster Workspace",
  "## Project Workspace",
  "## Backup",
  "## Settings",
  "## Agents",
  "## Skill editor",
  "## Agent editor",
  "## Endpoints",
  "## Test suite",
  "## Run results",
  "## Generate wizard",
  "## Compare",
  "## History",
  "## Tray",
  "## Updates",
  "## CLI",
  "## Workflow D.2",
  "MIT License",
];
const missing = headings.filter((heading) => !guide.includes(heading));
if (missing.length) {
  console.error(missing.join("\n"));
  process.exit(1);
}

const secrets = fs.readFileSync("docs/release/secrets.md", "utf8");
for (const name of [
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
  "APPLE_API_ISSUER",
  "APPLE_API_KEY",
  "APPLE_API_PRIVATE_KEY",
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
  "WINDOWS_CERTIFICATE",
  "WINDOWS_CERTIFICATE_PASSWORD",
  "AZURE_TENANT_ID",
  "AZURE_CLIENT_ID",
  "AZURE_CLIENT_SECRET",
  "AZURE_TRUSTED_SIGNING_ENDPOINT",
  "AZURE_CODE_SIGNING_ACCOUNT_NAME",
  "AZURE_CERTIFICATE_PROFILE_NAME",
  "TAURI_SIGNING_PRIVATE_KEY",
  "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
]) {
  if (!secrets.includes(`\`${name}\``)) {
    console.error(`secrets.md missing ${name}`);
    process.exit(1);
  }
}

const cli = fs.readFileSync("crates/hive-cli/src/lib.rs", "utf8");
const command = cli.split("enum Command {")[1].split("enum ")[0];
const variants = [...command.matchAll(/^\s{4}([A-Z][A-Za-z0-9]+)\s*\{/gm)].map((match) => match[1].toLowerCase());
const reference = fs.readFileSync("docs/cli/reference.md", "utf8");
for (const variant of variants) {
  if (!reference.includes(`## ${variant}`)) {
    console.error(`CLI reference missing ${variant}`);
    process.exit(1);
  }
}

for (const file of [
  "docs/dev/developer-guide.md",
  "docs/release/v1.0.0.md",
  "docs/release/macos-smoke.md",
  "docs/release/accessibility.md",
  "docs/release/performance.md",
  "docs/release/security.md",
  "docs/release/dod.md",
]) {
  if (!fs.existsSync(file) || fs.statSync(file).size < 200) {
    console.error(`short or missing ${file}`);
    process.exit(1);
  }
}

console.log(`docs cover ${headings.length} guide anchors and ${variants.length} CLI groups`);
