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
replaceVersion("src/components/shell/AppShell.tsx", /(Skill Hive )(\d+\.\d+\.\d+)()/);
replaceVersion("src/test/shell.test.tsx", /(Skill Hive )(\d+\.\d+\.\d+)()/);
replaceVersion("e2e/phase1-visual.spec.ts", /(Skill Hive )(\d+\.\d+\.\d+)()/);
replaceVersion("e2e/phase3-flows.spec.ts", /(Skill Hive )(\d+\.\d+\.\d+)()/);

const members = ["crates/hive-core/Cargo.toml", "crates/hive-cli/Cargo.toml", "src-tauri/Cargo.toml"];
for (const file of members) {
  const text = fs.readFileSync(file, "utf8");
  if (!text.includes("version.workspace = true")) {
    console.error(`${file} must inherit version.workspace = true`);
    process.exit(1);
  }
  console.log(`${file} inherits workspace version ${version}`);
}

const packaging = [
  "packaging/homebrew/Casks/skill-hive.rb",
  "packaging/homebrew/Formula/skill-hive-cli.rb",
  "packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.yaml",
  "packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.locale.en-US.yaml",
  "packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.installer.yaml",
];
for (const file of packaging) {
  const text = fs.readFileSync(file, "utf8");
  const next = text
    .replace(/version "[0-9]+\.[0-9]+\.[0-9]+"/, `version "${version}"`)
    .replace(/PackageVersion: [0-9]+\.[0-9]+\.[0-9]+/g, `PackageVersion: ${version}`)
    .replace(/download\/v[0-9]+\.[0-9]+\.[0-9]+\//g, `download/v${version}/`)
    .replace(/Hive_[0-9]+\.[0-9]+\.[0-9]+_/g, `Hive_${version}_`)
    .replace(/hive-cli-[0-9]+\.[0-9]+\.[0-9]+/g, `hive-cli-${version}`);
  fs.writeFileSync(file, next);
  console.log(`${file} -> ${version}`);
}
