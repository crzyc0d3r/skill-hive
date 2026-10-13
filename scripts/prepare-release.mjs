import fs from "node:fs";

const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
const version = pkg.version;
if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error(`package.json version is not semver: ${version}`);
  process.exit(1);
}

function replaceVersion(file, pattern) {
  const text = fs.readFileSync(file, "utf8");
  if (!pattern.test(text)) {
    console.error(`version field not found in ${file}`);
    process.exit(1);
  }
  const next = text.replace(pattern, `$1${version}$3`);
  fs.writeFileSync(file, next);
  console.log(`${file} -> ${version}`);
}

replaceVersion("src-tauri/tauri.conf.json", /("version": ")([^"]+)(")/);
replaceVersion("Cargo.toml", /(\nversion = ")([^"]+)(")/);

const members = ["crates/studio-core/Cargo.toml", "crates/studio-cli/Cargo.toml", "src-tauri/Cargo.toml"];
for (const file of members) {
  const text = fs.readFileSync(file, "utf8");
  if (!text.includes("version.workspace = true")) {
    console.error(`${file} must inherit version.workspace = true`);
    process.exit(1);
  }
  console.log(`${file} inherits workspace version ${version}`);
}
