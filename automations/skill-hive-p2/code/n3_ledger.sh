#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
python3 - <<'PY' >"${NODE_OUTPUT}"
from pathlib import Path
ledger = Path("docs/parity/m2-ledger.md").read_text()
limits = Path("docs/parity/m2-limits.md").read_text()
cli = Path("docs/cli/manage-skills.md").read_text()
required = [
    "Skill-aware merge conflicts",
    "Optional libgit2 engine",
    "Let your agents manage skills",
    "Lobster Agents",
    "Linked workspaces",
    "GitHub device-flow",
    "zh-CN and zh-TW",
]
missing = [row for row in required if row not in ledger]
print("# n3 ledger")
print()
print(f"ledger_bytes {len(ledger)}")
print(f"limits_bytes {len(limits)}")
print(f"cli_bytes {len(cli)}")
print("missing " + (", ".join(missing) if missing else "none"))
raise SystemExit(1 if missing or "TARGET_CONFLICT" not in cli else 0)
PY
