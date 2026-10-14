# n1 pipeline

node release tests exit 0
check-hardening exit 0
check-distribution exit 0
prepare-release exit 0
exit 0

# Subtest: Azure Trusted Signing sets the Windows signCommand hook
ok 4 - Azure Trusted Signing sets the Windows signCommand hook
  ---
  duration_ms: 0.217781
  ...
# Subtest: an updater password without the private key is partial
ok 5 - an updater password without the private key is partial
  ---
  duration_ms: 0.186661
  ...
# Subtest: the updater private key enables updater artifacts only
ok 6 - the updater private key enables updater artifacts only
  ---
  duration_ms: 0.180366
  ...
# Subtest: unsigned verification accepts the five targets without signatures
ok 7 - unsigned verification accepts the five targets without signatures
  ---
  duration_ms: 0.846434
  ...
# Subtest: a missing signature blocks publication
ok 8 - a missing signature blocks publication
  ---
  duration_ms: 0.351009
  ...
# Subtest: signed verification passes when every signature is present
ok 9 - signed verification passes when every signature is present
  ---
  duration_ms: 0.306579
  ...
# Subtest: the release workflow keeps unsigned builds and names the secret gates
ok 10 - the release workflow keeps unsigned builds and names the secret gates
  ---
  duration_ms: 0.381908
  ...
1..10
# tests 10
# suites 0
# pass 10
# fail 0
# cancelled 0
# skipped 0
# todo 0
# duration_ms 51.210421
hardening checks passed for CSP, capabilities, identity, and Linux dependencies
distribution manifests describe the Skill Hive cask, CLI formula, and winget package
src-tauri/tauri.conf.json -> 1.0.0
Cargo.toml -> 1.0.0
src/components/shell/AppShell.tsx -> 1.0.0
src/test/shell.test.tsx -> 1.0.0
e2e/phase1-visual.spec.ts -> 1.0.0
e2e/phase3-flows.spec.ts -> 1.0.0
crates/hive-core/Cargo.toml inherits workspace version 1.0.0
crates/hive-cli/Cargo.toml inherits workspace version 1.0.0
src-tauri/Cargo.toml inherits workspace version 1.0.0
packaging/homebrew/Casks/skill-hive.rb -> 1.0.0
packaging/homebrew/Formula/skill-hive-cli.rb -> 1.0.0
packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.yaml -> 1.0.0
packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.locale.en-US.yaml -> 1.0.0
packaging/winget/manifests/s/SkillHive/SkillHive/SkillHive.SkillHive.installer.yaml -> 1.0.0
