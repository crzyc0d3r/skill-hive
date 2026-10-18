#!/usr/bin/env python3
"""The result file must record the audit counts, the wire test, and four screenshots."""

import sys
from pathlib import Path

text = Path("automations/effectiveness-verify/out/result.md").read_text()
required = [
    "real=36",
    "stand-in=16",
    "missing=11",
    "total=63",
    "wire_test=True",
    "blind_command=True",
    "screenshots=4/4",
]
problems = [line for line in required if line not in text]
if problems:
    print("result missing:\n" + "\n".join(problems))
    sys.exit(1)
print(text)
