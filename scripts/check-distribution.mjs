import fs from "node:fs";

const problems = [];
const cask = fs.readFileSync("packaging/homebrew/Casks/skill-hive.rb", "utf8");
const formula = fs.readFileSync("packaging/homebrew/Formula/skill-hive-cli.rb", "utf8");
const installer = fs.readFileSync("packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.installer.yaml", "utf8");
const locale = fs.readFileSync("packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.locale.en-US.yaml", "utf8");
const version = fs.readFileSync("packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.yaml", "utf8");

for (const [label, text, needles] of [
  ["cask", cask, ["cask \"skill-hive\"", "Skill Hive", "on_arm", "on_intel", "app \"Skill Hive.app\""]],
  ["formula", formula, ["class SkillHiveCli", "skill-hive-cli"]],
  ["winget installer", installer, ["PackageIdentifier: SkillHive.SkillHive", "InstallerType: wix", "Architecture: x64"]],
  ["winget locale", locale, ["PackageName: Skill Hive", "Publisher: Skill Hive", "Moniker: SkillHive"]],
  ["winget version", version, ["PackageIdentifier: SkillHive.SkillHive", "ManifestType: version"]],
]) {
  for (const needle of needles) {
    if (!text.includes(needle)) problems.push(`${label} missing ${needle}`);
  }
  if (/skills-manager|Skills Manager/i.test(text)) problems.push(`${label} reuses the upstream name`);
}

if (problems.length) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log("distribution manifests describe the Skill Hive cask, CLI formula, and winget package");
