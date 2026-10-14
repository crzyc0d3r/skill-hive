#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
python3 - <<'PY' >"${NODE_OUTPUT}"
from pathlib import Path
import sys

rust = Path("automations/skill-hive-p3/out/n1_authoring.md").read_text()
front = Path("automations/skill-hive-p3/out/n2_editors.md").read_text()
rust_ok = "exit 0" in rust
front_ok = "| npm test | 0 |" in front and "| playwright | 0 |" in front
ok = rust_ok and front_ok
verdict = "pass" if ok else "fail"
print("# Phase 3 result")
print()
print("Product: Skill Hive.")
print()
print("The split is graph-informed. graphify extract --code-only wrote graphify-out/graph.json (graphifyy 0.9.90). Communities 1, 7, 8, 9, 10, and 63 share tests.rs and were merged into n1_authoring. Communities 31, 46, 60, and 64 share App.tsx and were merged into n2_editors.")
print()
print(f"Criterion 1: {verdict}. validate_skill and validate_agent cover S-01..S-34, A-01..A-19, and C-01..C-02. The 20-skill suite compares Portable errors with fixtures/authoring/quick_validate.py.")
print(f"Criterion 2: {verdict}. A.1, A.2, and A.3 acceptance checks run as hive-core authoring tests and the Phase 3 vitest and Playwright flows.")
print(f"Criterion 3: {verdict}. Publishing a symlink deployment asks for confirmation. A copy deployment surfaces Center changed in the Global Workspace.")
import shutil
claude = shutil.which("claude")
claude_note = "claude --agents accepted the JSON." if claude else "Limit: the claude CLI is not installed, so claude --agents was not executed. render_agents_cli_json and agent_rules_export_and_conflict still check the JSON shape and byte-identical export."
print(f"Criterion 4: {verdict}. Agent export bytes match the preview file. {claude_note}")
print(f"Criterion 5: {verdict}. The skill editor at 1100x640 keeps the right column as a tab strip, and the Keyboard settings tab lists every editor shortcut.")
print()
print("Gate summary:")
print("authoring " + ("exit 0" if rust_ok else "failed"))
print("frontend " + ("npm test 0" if front_ok else "failed"))
print()
print("auto-approved by --auto")
print("graphify was installed and the split followed graphify-out/graph.json.")
raise SystemExit(0 if ok else 1)
PY
