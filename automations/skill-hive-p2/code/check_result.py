#!/usr/bin/env python3
from pathlib import Path
import sys

text = Path("automations/skill-hive-p2/out/result.md").read_text()
required = [f"Criterion {n}: pass" for n in range(1, 7)]
missing = [line for line in required if line not in text]
if missing:
    print("missing:", ", ".join(missing))
    sys.exit(1)
print("result.md records six passing Phase 2 criteria")
