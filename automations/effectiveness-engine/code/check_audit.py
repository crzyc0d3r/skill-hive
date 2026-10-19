#!/usr/bin/env python3
"""The audit's summary counts must match its rows, and stand-ins are not called done."""

import re
import sys
from collections import Counter
from pathlib import Path

text = Path("docs/plan-conformance-audit.md").read_text()
rows = re.findall(r"^\| [^|\n]+ \| [^|\n]+ \| (real|stand-in|missing) \|", text, re.M)
counts = Counter(rows)
required = ("Part 2", "Part 3", "Part 1 §4", "Part 1 §5", "Part 4")
missing_sections = [name for name in required if name not in text]
problems = []
if missing_sections:
    problems.append("missing sections: " + ", ".join(missing_sections))
if not rows:
    problems.append("no status rows")
for status in ("real", "stand-in", "missing"):
    match = re.search(rf"\| {status} \| (\d+) \|", text)
    if not match:
        problems.append(f"summary missing {status}")
        continue
    if int(match.group(1)) != counts[status]:
        problems.append(f"{status} summary {match.group(1)} != table {counts[status]}")
if "Stand-in rows are not done." not in text:
    problems.append("stand-in rows are not explicitly excluded")
if "ScriptedProvider" not in text:
    problems.append("scripted stand-in is not named")
if problems:
    print("\n".join(problems))
    sys.exit(1)
print(f"real={counts['real']} stand-in={counts['stand-in']} missing={counts['missing']} total={len(rows)}")
