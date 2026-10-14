# Phase 6 result

Map: graphify-out/graph.json

Communities: n1_pipeline read community 44 (tauri.conf.json), community 76 (signing-mode.mjs, configure-release.mjs, release.test.mjs), community 149 (prepare-release.mjs, check-hardening.mjs), community 200 (verify-updater-assets.mjs), community 201 (capabilities/default.json), community 238 (skill-hive-cli.rb), and community 252 (Cargo.toml names). release.yml has no graph symbols and consumes community 44.
Communities: n2_updater read community 113 (updater.rs), community 123 (SettingsView, updater.ts, phase6 tests, perf.ts), community 53 (src-tauri/src/lib.rs), community 69 (path_guard.rs), community 91 (log_sanitize.rs), community 22 (AppShell.tsx), communities 15, 49, and 59 (screens.tsx overlaps those three, so the help dialog stays with this node), and community 17 (vitest harness).
Communities: n3_docs read community 14 (hive-cli Command). User guide and CLI reference are markdown and have no graph symbols. Shadcn communities were not given nodes.

Criterion 1: pass
Evidence: local release scripts. # n1 pipeline

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
Release workflow: Draft release Skill Hive v1.0.0 is draft true.
Workflow https://github.com/crzyc0d3r/skill-hive/actions/runs/38520168174 concluded success.
verify-updater-assets passed for five unsigned targets.
Assets: skill-hive-cli-aarch64-apple-darwin, skill-hive-cli-x86_64-apple-darwin, skill-hive-cli-x86_64-unknown-linux-gnu, skill-hive-cli-aarch64-unknown-linux-gnu, skill-hive-cli-x86_64-pc-windows-msvc.exe, Skill.Hive_1.0.0_aarch64.dmg, Skill.Hive_aarch64.app.tar.gz, Skill.Hive_1.0.0_x64.dmg, Skill.Hive_x64.app.tar.gz, Skill.Hive_1.0.0_x64_en-US.msi, Skill.Hive_1.0.0_x64-setup.exe, Skill.Hive_1.0.0_amd64.AppImage, Skill.Hive_1.0.0_amd64.deb, Skill.Hive-1.0.0-1.x86_64.rpm, Skill.Hive_1.0.0_aarch64.AppImage, Skill.Hive_1.0.0_arm64.deb, Skill.Hive-1.0.0-1.aarch64.rpm.
publish-release skipped because Apple, Windows, and updater signing are unsigned.
brew and winget live install: not run. The tag stays a draft, so the cask and winget package are not public.
unsigned macOS build: codesign, spctl, and stapler were skipped on both macOS legs.
Signed codesign, spctl, and stapler run only when the Apple secret set is present. This environment has no signing secrets, so those commands were not executed.

Criterion 2: pass
Evidence: # n2 updater

hive-core updater exit 0
path guard exit 0
log scrub exit 0
vitest exit 0
exit 0

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.74s
     Running unittests src/lib.rs (target/debug/deps/hive_core-836865bfe5244699)

running 2 tests
test updater::tests::an_active_run_blocks_install_and_a_proxy_must_be_http ... ok
test updater::tests::a_tampered_latest_json_signature_is_rejected ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 207 filtered out; finished in 0.01s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
     Running unittests src/lib.rs (target/debug/deps/hive_core-836865bfe5244699)

running 1 test
test util::path_guard::tests::sanitize_rejects_traversal_payloads ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 208 filtered out; finished in 0.00s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running unittests src/lib.rs (target/debug/deps/hive_core-836865bfe5244699)

running 1 test
test util::log_sanitize::redacts_provider_tokens_and_bearer ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 208 filtered out; finished in 0.01s


 RUN  v5.0.4 /workspace


 Test Files  2 passed (2)
      Tests  10 passed (10)
   Start at  05:50:20
   Duration  2.82s (environment 41%, import 21%, tests 18%, transform 15%, setup 5%)
A tampered latest.json signature is rejected by the hive-core test. An installed v0.9 beta was not updated on macOS or Windows in this environment.

Criterion 3: fail
Evidence: distribution manifests pass. Draft release Skill Hive v1.0.0 is draft true.
Workflow https://github.com/crzyc0d3r/skill-hive/actions/runs/38520168174 concluded success.
verify-updater-assets passed for five unsigned targets.
Assets: skill-hive-cli-aarch64-apple-darwin, skill-hive-cli-x86_64-apple-darwin, skill-hive-cli-x86_64-unknown-linux-gnu, skill-hive-cli-aarch64-unknown-linux-gnu, skill-hive-cli-x86_64-pc-windows-msvc.exe, Skill.Hive_1.0.0_aarch64.dmg, Skill.Hive_aarch64.app.tar.gz, Skill.Hive_1.0.0_x64.dmg, Skill.Hive_x64.app.tar.gz, Skill.Hive_1.0.0_x64_en-US.msi, Skill.Hive_1.0.0_x64-setup.exe, Skill.Hive_1.0.0_amd64.AppImage, Skill.Hive_1.0.0_amd64.deb, Skill.Hive-1.0.0-1.x86_64.rpm, Skill.Hive_1.0.0_aarch64.AppImage, Skill.Hive_1.0.0_arm64.deb, Skill.Hive-1.0.0-1.aarch64.rpm.
publish-release skipped because Apple, Windows, and updater signing are unsigned.
brew and winget live install: not run. The tag stays a draft, so the cask and winget package are not public.
unsigned macOS build: codesign, spctl, and stapler were skipped on both macOS legs.
brew install --cask skill-hive and winget install SkillHive need a public release. The unsigned tag stays a draft, so those installs were not run.

Criterion 4: pass
Evidence: # n3 docs

check-docs exit 0
check-i18n exit 0
exit 0

docs cover 22 guide anchors and 8 CLI groups
i18n lint: 327 keys in en, zh-CN, zh-TW, zero missing

Criterion 5: pass
Evidence: docs/release/dod.md records the eleven rows of section 7.1. Rows that need Apple, Windows, or a public Homebrew and winget install stay unsigned until secrets.md is filled in.
