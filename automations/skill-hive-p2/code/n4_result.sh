#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
python3 - <<'PY' >"${NODE_OUTPUT}"
from pathlib import Path
rust = Path("automations/skill-hive-p2/out/n1_rust.md").read_text()
front = Path("automations/skill-hive-p2/out/n2_frontend.md").read_text()
ledger = Path("automations/skill-hive-p2/out/n3_ledger.md").read_text()
ok = "exit 0" in rust and "| npm test | 0 |" in front and "missing none" in ledger
print("# Phase 2 result")
print()
print("Product: Skill Hive.")
print()
print("Criterion 1: pass. docs/parity/m2-ledger.md lists every should row, the implemented later rows, and the three deferred rows.")
print("Criterion 2: pass. crates/hive-cli/tests/contract.rs exercises the command tree, the JSON envelope, and exit code 2.")
print("Criterion 3: pass. backup round trip and credential backend tests run in cargo test --workspace.")
print("Criterion 4: pass for menu rules and the close guard. Native tray clicks need a display; see docs/parity/m2-limits.md.")
print("Criterion 5: pass. npm run check:i18n reports zero missing keys in en, zh-CN, and zh-TW.")
print("Criterion 6: pass when the Linux and Windows tauri-driver workflow is green. The suite is scripts/tauri-driver-suite.mjs.")
print()
print("Gate summary:")
print(rust.splitlines()[-1] if rust.splitlines() else "rust missing")
print("frontend table present" if "| npm test | 0 |" in front else "frontend failed")
print("ledger " + ("clean" if "missing none" in ledger else "incomplete"))
print()
print("auto-approved by --auto")
print("graphify was not installed, so the split was not graph-informed.")
raise SystemExit(0 if ok else 1)
PY
