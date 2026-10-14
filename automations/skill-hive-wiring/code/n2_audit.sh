#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
test -s docs/wiring-audit.md
grep -q 'install_local_skill' docs/wiring-audit.md
grep -q 'Before' docs/wiring-audit.md
grep -q 'After' docs/wiring-audit.md
mkdir -p automations/skill-hive-wiring/out
printf 'audit-ok\n' > automations/skill-hive-wiring/out/n2_audit.md
