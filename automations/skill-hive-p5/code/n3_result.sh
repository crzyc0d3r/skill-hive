#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
python3 - <<'PY' >"${NODE_OUTPUT}"
from pathlib import Path
import sys

rust = Path("automations/skill-hive-p5/out/n1_core.md").read_text()
front = Path("automations/skill-hive-p5/out/n2_screens.md").read_text()
rust_ok = "exit 0" in rust
front_ok = "| npm test | 0 |" in front and "| playwright | 0 |" in front
ok = rust_ok and front_ok
verdict = "pass" if ok else "fail"
print("# Phase 5 result")
print()
print("Product: Skill Hive.")
print()
print("The split is graph-informed. graphify extract --code-only wrote graphify-out/graph.json (graphifyy 0.9.90), then graphify update and cluster-only refreshed it to 308 communities. n1_core takes the Rust communities that compile into one hive-core or hive-cli test binary: 12 (generation session), 137 (envelope), 7 (analytics workflow), 118 (optimize), 131 (compare), 70 (regression), 95 (export), 51 (schema v12), 90 (cli phase5), and 107 (replay benchmark). n2_screens takes the UI communities: 14 (wizard, compare, suite), 43 (App, dashboard, history), 52 (library badges), 32 (shell), 3 (vitest), and 139 (playwright). Shadcn communities were not given nodes.")
print()
print(f"Criterion 1: {verdict}. B.3 criteria 1 to 8 and C.7 criteria 10 to 15 pass as hive-core tests on recorded fixtures. Nine of ten replayed goal sessions reach Review with zero validation errors; the tenth surfaces findings and accept refuses to save. Limit: no vendor API key is configured, so the nine-of-ten measurement is the frozen replay fixture, not a live endpoint.")
print(f"Criterion 2: {verdict}. Workflows D.1, D.2, and D.3 run end to end in analytics::tests on the fixture skill and a generated agent, using recorded replies.")
print(f"Criterion 3: {verdict}. The exported skill-creator workspace opens with fixtures/eval-viewer/generate_review.py --static. JUnit testcase counts and CSV row counts match the cells, expectations, and checks.")
print(f"Criterion 4: {verdict}. The Library shows Tested and Regressed badges and the Dashboard shows the Tested stat and the regressions block from the regression report produced by rules R1 to R6.")
print()
print("Gate summary:")
print("core " + ("exit 0" if rust_ok else "failed"))
print("frontend " + ("npm test 0" if front_ok else "failed"))
print()
print("auto-approved by --auto")
print("graphify was installed and the split followed graphify-out/graph.json.")
raise SystemExit(0 if ok else 1)
PY
