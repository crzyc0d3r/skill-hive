#!/usr/bin/env python3
"""Acceptance check: result.md records five passing Phase 4 criteria and a graphify map."""
from pathlib import Path
import sys

path = Path("automations/skill-hive-p4/out/result.md")
if not path.is_file():
    print("missing automations/skill-hive-p4/out/result.md")
    sys.exit(1)
text = path.read_text()
required = [f"Criterion {n}: pass" for n in range(1, 6)]
required.append("graphify-out/graph.json")
missing = [line for line in required if line not in text]
if missing:
    print("missing:", ", ".join(missing))
    sys.exit(1)
print("result.md records five passing Phase 4 criteria and the graphify map")
