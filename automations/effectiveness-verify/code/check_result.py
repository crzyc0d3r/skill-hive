#!/usr/bin/env python3
"""The result file must record the audit counts, the wire test, and the reviewed screenshots."""

import re
import sys
from collections import Counter
from pathlib import Path

audit = Path("docs/plan-conformance-audit.md").read_text()
rows = re.findall(r"^\| [^|\n]+ \| [^|\n]+ \| (real|stand-in|missing) \|", audit, re.M)
counts = Counter(rows)
text = Path("automations/effectiveness-verify/out/result.md").read_text()
required = [
    f"real={counts['real']}",
    f"stand-in={counts['stand-in']}",
    f"missing={counts['missing']}",
    f"total={len(rows)}",
    "wire_test=True",
    "blind_command=True",
]
problems = [line for line in required if line not in text]
shots = [
    "suite",
    "compare",
    "effectiveness-charts",
    "results",
    "history",
    "endpoints",
    "agents",
    "skill-editor",
]
present = 0
for name in shots:
    for theme in ("light", "dark"):
        path = Path("docs/parity/screenshots") / theme / f"{name}.png"
        if path.is_file() and path.stat().st_size > 1000:
            present += 1
        else:
            problems.append(f"missing screenshot {theme}/{name}.png")
if f"screenshots={present}/{len(shots) * 2}" not in text and f"screenshots={len(shots)}/{len(shots)}" not in text:
    problems.append(f"screenshot count is not recorded ({present} files)")
if problems:
    print("result missing:\n" + "\n".join(problems))
    sys.exit(1)
print(text)
