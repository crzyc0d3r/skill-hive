import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const dir = process.argv[2];
if (!dir) {
  console.error("usage: bump-distribution.mjs <asset-dir>");
  process.exit(2);
}

const hashes = new Map();
for (const name of fs.readdirSync(dir)) {
  const full = path.join(dir, name);
  if (!fs.statSync(full).isFile()) continue;
  const sha = crypto.createHash("sha256").update(fs.readFileSync(full)).digest("hex");
  hashes.set(name, sha);
  console.log(`${sha}  ${name}`);
}

const files = [
  "packaging/homebrew/Casks/skill-hive.rb",
  "packaging/homebrew/Formula/skill-hive-cli.rb",
  "packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.installer.yaml",
];
const version = JSON.parse(fs.readFileSync("package.json", "utf8")).version;

function replaceNextPending(text, needle, sha) {
  const index = text.indexOf(needle);
  if (index === -1) return text;
  const pending = text.indexOf("PENDING", index);
  if (pending === -1) return text;
  return `${text.slice(0, pending)}${sha}${text.slice(pending + "PENDING".length)}`;
}

for (const file of files) {
  let text = fs.readFileSync(file, "utf8");
  for (const [name, sha] of hashes) {
    const needle = text.includes(name) ? name : name.replaceAll(version, "#{version}");
    text = replaceNextPending(text, needle, sha);
  }
  fs.writeFileSync(file, text);
}
console.log("distribution checksums updated from release assets");
