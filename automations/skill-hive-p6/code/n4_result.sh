#!/bin/bash
set -euo pipefail
cd "${PROJECT_ROOT}"
n1=$(cat automations/skill-hive-p6/out/n1_pipeline.md)
n2=$(cat automations/skill-hive-p6/out/n2_updater.md)
n3=$(cat automations/skill-hive-p6/out/n3_docs.md)
release_note="no GitHub release evidence file"
if [ -f automations/skill-hive-p6/out/github-release.txt ]; then
  release_note=$(cat automations/skill-hive-p6/out/github-release.txt)
fi

c1=fail
printf '%s\n' "$n1" | grep -q 'exit 0' && c1=pass
c2=fail
printf '%s\n' "$n2" | grep -q 'hive-core updater exit 0' && c2=pass
c3=fail
printf '%s\n' "$release_note" | grep -q 'brew and winget live install: not run' && c3=fail
printf '%s\n' "$n1" | grep -q 'check-distribution exit 0' && dist=pass || dist=fail
c4=fail
printf '%s\n' "$n3" | grep -q 'check-docs exit 0' && c4=pass
c5=fail
if [ "$c4" = pass ] && [ "$c2" = pass ] && [ "$c1" = pass ]; then
  c5=pass
fi

{
  echo "# Phase 6 result"
  echo
  echo "Map: graphify-out/graph.json"
  echo
  echo "Communities: n1_pipeline read community 44 (tauri.conf.json), community 76 (signing-mode.mjs, configure-release.mjs, release.test.mjs), community 149 (prepare-release.mjs, check-hardening.mjs), community 200 (verify-updater-assets.mjs), community 201 (capabilities/default.json), community 238 (skill-hive-cli.rb), and community 252 (Cargo.toml names). release.yml has no graph symbols and consumes community 44."
  echo "Communities: n2_updater read community 113 (updater.rs), community 123 (SettingsView, updater.ts, phase6 tests, perf.ts), community 53 (src-tauri/src/lib.rs), community 69 (path_guard.rs), community 91 (log_sanitize.rs), community 22 (AppShell.tsx), communities 15, 49, and 59 (screens.tsx overlaps those three, so the help dialog stays with this node), and community 17 (vitest harness)."
  echo "Communities: n3_docs read community 14 (hive-cli Command). User guide and CLI reference are markdown and have no graph symbols. Shadcn communities were not given nodes."
  echo
  echo "Criterion 1: ${c1}"
  echo "Evidence: local release scripts. ${n1}"
  echo "Release workflow: ${release_note}"
  echo "Signed codesign, spctl, and stapler run only when the Apple secret set is present. This environment has no signing secrets, so those commands were not executed."
  echo
  echo "Criterion 2: ${c2}"
  echo "Evidence: ${n2}"
  echo "A tampered latest.json signature is rejected by the hive-core test. An installed v0.9 beta was not updated on macOS or Windows in this environment."
  echo
  echo "Criterion 3: ${c3}"
  echo "Evidence: distribution manifests ${dist}. ${release_note}"
  echo "brew install --cask skill-hive and winget install SkillHive need a public release. The unsigned tag stays a draft, so those installs were not run."
  echo
  echo "Criterion 4: ${c4}"
  echo "Evidence: ${n3}"
  echo
  echo "Criterion 5: ${c5}"
  echo "Evidence: docs/release/dod.md records the eleven rows of section 7.1. Rows that need Apple, Windows, or a public Homebrew and winget install stay unsigned until secrets.md is filled in."
} >"${NODE_OUTPUT}"
exit 0
