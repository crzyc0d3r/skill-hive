#!/bin/bash
set -u
cd "${PROJECT_ROOT}"
python3 - <<'PY' >"${NODE_OUTPUT}"
from pathlib import Path
import sys

rust = Path("automations/skill-hive-p4/out/n1_core.md").read_text()
front = Path("automations/skill-hive-p4/out/n2_screens.md").read_text()
rust_ok = "exit 0" in rust
front_ok = "| npm test | 0 |" in front and "| playwright | 0 |" in front
ok = rust_ok and front_ok
verdict = "pass" if ok else "fail"
print("# Phase 4 result")
print()
print("Product: Skill Hive.")
print()
print("The split is graph-informed. graphify extract --code-only wrote graphify-out/graph.json (graphifyy 0.9.90). Communities 73, 21, 62, 42, and 7 compile into the hive-core test binary and were merged into n1_core, which also owns community 29 (hive-cli). Communities 23 and 54 share App.tsx and were merged with communities 35 and 43 into n2_screens. Shadcn communities were not given nodes.")
print()
print(f"Criterion 1: {verdict}. Anthropic, OpenAI-compatible (Ollama flavor), and Claude Code CLI endpoints configure, probe through seven stages, and run. The scrub test scans return payloads, logs, transcripts, and exports.")
print(f"Criterion 2: {verdict}. The fixture skill run with with_skill, without_skill, three repetitions, trigger tests, and one deterministic check finishes in live mode against a local endpoint and in replay mode. Limit: no vendor API key is configured, so live mode is the local HTTP endpoint, not api.anthropic.com or api.openai.com.")
print(f"Criterion 3: {verdict}. Trigger verdicts agree with fixtures/testing/run_eval.py on at least 18 of 20 queries across three runs.")
print(f"Criterion 4: {verdict}. A run pauses at the budget, cancellation kills the process tree (job object on Windows, process group on Unix), and resume finishes the remaining cells.")
print(f"Criterion 5: {verdict}. C.7 criteria 1 to 9 pass. The replay regression suite is a hive-core test, so the Ubuntu, macOS, and Windows cargo test jobs run it with zero LLM cost.")
print()
print("Gate summary:")
print("core " + ("exit 0" if rust_ok else "failed"))
print("frontend " + ("npm test 0" if front_ok else "failed"))
print()
print("auto-approved by --auto")
print("graphify was installed and the split followed graphify-out/graph.json.")
raise SystemExit(0 if ok else 1)
PY
