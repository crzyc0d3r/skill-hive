#!/usr/bin/env python3
import re
from collections import Counter
from pathlib import Path

text = Path("docs/plan-conformance-audit.md").read_text()
rows = re.findall(r"^\| [^|\n]+ \| [^|\n]+ \| (real|stand-in|missing) \|", text, re.M)
counts = Counter(rows)
out = Path("automations/effectiveness-engine/out/counts.txt")
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(
    f"real={counts['real']}\nstand-in={counts['stand-in']}\nmissing={counts['missing']}\ntotal={len(rows)}\n"
)
print(out.read_text())
