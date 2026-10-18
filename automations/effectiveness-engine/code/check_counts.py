#!/usr/bin/env python3
import re
import sys
from pathlib import Path

text = Path("automations/effectiveness-engine/out/counts.txt").read_text()
audit = Path("docs/plan-conformance-audit.md").read_text()
problems = []
for status in ("real", "stand-in", "missing"):
    got = re.search(rf"^{status}=(\d+)$", text, re.M)
    expected = re.search(rf"\| {status} \| (\d+) \|", audit)
    if not got or not expected or got.group(1) != expected.group(1):
        problems.append(f"{status} counts.txt does not match the audit summary")
if problems:
    print("\n".join(problems))
    sys.exit(1)
print(text.strip())
