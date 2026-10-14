#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
node scripts/gen-ipc-types.mjs --check
grep -q 'commands::install_local_skill' src-tauri/src/lib.rs
grep -q 'fn install_local_skill' src-tauri/src/commands.rs
mkdir -p automations/skill-hive-wiring/out
printf 'ipc-ok\n' > automations/skill-hive-wiring/out/n1_commands.md
