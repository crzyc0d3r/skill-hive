#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
mkdir -p automations/skill-hive-wiring/out
{
  echo "# Wiring result"
  echo
  echo "Commands: $(cat automations/skill-hive-wiring/out/n1_commands.md)"
  echo "Audit: $(cat automations/skill-hive-wiring/out/n2_audit.md)"
  echo "Deliverable: docs/wiring-audit.md"
} > automations/skill-hive-wiring/out/result.md
