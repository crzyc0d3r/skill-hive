#!/usr/bin/env python3
"""Run the Phase 0 local checks and write the deliverable."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

ROOT = Path(os.environ.get("PROJECT_ROOT", Path(__file__).resolve().parents[3]))
OUT = Path(os.environ.get("NODE_OUTPUT", ROOT / "automations/skill-studio-p0/out/result.md"))


def run(cmd: str) -> tuple[int, str]:
    proc = subprocess.run(cmd, shell=True, cwd=ROOT, capture_output=True, text=True)
    text = (proc.stdout + proc.stderr).strip()
    tail = "\n".join(text.splitlines()[-30:])
    return proc.returncode, tail


checks = [
    ("cargo fmt", "cargo fmt --all"),
    ("cargo test", "cargo test --workspace -- --test-threads=1"),
    ("npm test", "npm test"),
    ("tsc", "npx tsc -b --pretty false"),
    ("contrast", "node scripts/contrast.mjs"),
    ("headers", "node scripts/check-headers.mjs"),
]
results = []
failed = False
for name, cmd in checks:
    code, tail = run(cmd)
    results.append((name, code, tail))
    if code != 0:
        failed = True

lines = [
    "# Skill Studio Phase 0",
    "",
    "Local verification for milestone M0. schema v8 is asserted by `studio_db_migrates_to_v8_and_layout_is_created`.",
    "",
    "| check | exit |",
    "|---|---|",
]
for name, code, _tail in results:
    lines.append(f"| {name} | {code} |")
lines.append("")
lines.append("## Evidence")
lines.append("")
for name, code, tail in results:
    lines.append(f"### {name} (exit {code})")
    lines.append("")
    lines.append("```")
    lines.append(tail or "(no output)")
    lines.append("```")
    lines.append("")
if failed:
    lines.append("fail: one or more local checks exited non-zero")
else:
    lines.append("pass: cargo test and npm test are green locally; schema v8 migration is in the studio-core suite.")
OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(OUT)
if failed:
    raise SystemExit(1)
