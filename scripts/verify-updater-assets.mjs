import fs from "node:fs";

/**
 * Require the five release targets. In signed mode also require latest.json
 * and one updater signature per target. A missing signature blocks publication.
 */

const TARGETS = [
  ["darwin-aarch64 dmg", (name) => name.endsWith(".dmg") && /aarch64/i.test(name)],
  ["darwin-aarch64 app.tar.gz", (name) => name.endsWith(".app.tar.gz") && /aarch64/i.test(name)],
  ["darwin-x86_64 dmg", (name) => name.endsWith(".dmg") && /x64|x86_64/i.test(name) && !/aarch64|arm64/i.test(name)],
  ["darwin-x86_64 app.tar.gz", (name) => name.endsWith(".app.tar.gz") && /x64|x86_64/i.test(name) && !/aarch64|arm64/i.test(name)],
  ["windows-x86_64 msi", (name) => name.endsWith(".msi")],
  ["windows-x86_64 setup.exe", (name) => /-setup\.exe$/i.test(name)],
  ["linux-x86_64 AppImage", (name) => name.endsWith(".AppImage") && /amd64|x86_64|x64/i.test(name)],
  ["linux-x86_64 deb", (name) => name.endsWith(".deb") && /amd64|x86_64/i.test(name)],
  ["linux-x86_64 rpm", (name) => name.endsWith(".rpm") && /x86_64|amd64/i.test(name)],
  ["linux-aarch64 AppImage", (name) => name.endsWith(".AppImage") && /aarch64|arm64/i.test(name)],
  ["linux-aarch64 deb", (name) => name.endsWith(".deb") && /aarch64|arm64/i.test(name)],
  ["linux-aarch64 rpm", (name) => name.endsWith(".rpm") && /aarch64|arm64/i.test(name)],
  ["cli darwin-aarch64", (name) => /skill-hive-cli/.test(name) && /aarch64-apple-darwin/.test(name)],
  ["cli darwin-x86_64", (name) => /skill-hive-cli/.test(name) && /x86_64-apple-darwin/.test(name)],
  ["cli linux-x86_64", (name) => /skill-hive-cli/.test(name) && /x86_64-unknown-linux-gnu/.test(name)],
  ["cli linux-aarch64", (name) => /skill-hive-cli/.test(name) && /aarch64-unknown-linux-gnu/.test(name)],
  ["cli windows-x86_64", (name) => /skill-hive-cli/.test(name) && /pc-windows/.test(name)],
];

const SIGNATURES = [
  ["darwin-aarch64 sig", (name) => name.endsWith(".sig") && name.includes(".app.tar.gz") && /aarch64/i.test(name)],
  ["darwin-x86_64 sig", (name) => name.endsWith(".sig") && name.includes(".app.tar.gz") && /x64|x86_64/i.test(name) && !/aarch64|arm64/i.test(name)],
  ["windows-x86_64 sig", (name) => name.endsWith(".sig") && /-setup\.exe/i.test(name)],
  ["linux-x86_64 sig", (name) => name.endsWith(".sig") && name.includes(".AppImage") && /amd64|x86_64|x64/i.test(name)],
  ["linux-aarch64 sig", (name) => name.endsWith(".sig") && name.includes(".AppImage") && /aarch64|arm64/i.test(name)],
  ["latest.json", (name) => name === "latest.json" || name.endsWith("/latest.json")],
];

export function verifyAssetNames(names, { signed }) {
  const missing = [];
  for (const [label, pred] of TARGETS) {
    if (!names.some(pred)) missing.push(label);
  }
  if (signed) {
    for (const [label, pred] of SIGNATURES) {
      if (!names.some(pred)) missing.push(label);
    }
  }
  return missing;
}

function readNames(file) {
  return fs
    .readFileSync(file, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function main() {
  const signed = process.argv.includes("--signed");
  const fileFlag = process.argv.indexOf("--names");
  if (fileFlag === -1) {
    console.error("usage: verify-updater-assets.mjs --names <file> [--signed]");
    process.exit(2);
  }
  const missing = verifyAssetNames(readNames(process.argv[fileFlag + 1]), { signed });
  if (missing.length) {
    console.error(signed ? "blocked before publication" : "release assets incomplete");
    for (const item of missing) console.error(`missing ${item}`);
    process.exit(1);
  }
  console.log(signed ? "verify-updater-assets passed for five signed targets" : "verify-updater-assets passed for five unsigned targets");
}

if (process.argv[1] && process.argv[1].endsWith("verify-updater-assets.mjs")) main();
