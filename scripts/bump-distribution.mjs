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
for (const file of files) {
  let text = fs.readFileSync(file, "utf8");
  for (const [name, sha] of hashes) {
    if (!text.includes(name)) continue;
    text = text.replaceAll("PENDING", sha);
  }
  fs.writeFileSync(file, text);
}
console.log("distribution checksums updated from release assets");
