#!/usr/bin/env python3
"""Acceptance check: result.md records five Phase 6 criteria and the graphify communities."""
from pathlib import Path
import sys

path = Path("automations/skill-hive-p6/out/result.md")
if not path.is_file() or path.stat().st_size == 0:
    print("missing automations/skill-hive-p6/out/result.md")
    sys.exit(1)
text = path.read_text()
required = [f"Criterion {n}:" for n in range(1, 6)]
required.extend(
    [
        "graphify-out/graph.json",
        "community 44",
        "community 113",
        "community 14",
        "pass",
        "fail",
    ]
)
missing = [line for line in required if line not in text]
if missing:
    print("missing:", ", ".join(missing))
    sys.exit(1)
print("result.md records five Phase 6 criteria and the graphify communities")
