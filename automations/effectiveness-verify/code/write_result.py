#!/usr/bin/env python3
"""Write the verification result from the audit rows and the files the build must leave behind."""

import re
from collections import Counter
from pathlib import Path

root = Path(".")
text = (root / "docs/plan-conformance-audit.md").read_text()
rows = re.findall(r"^\| [^|\n]+ \| [^|\n]+ \| (real|stand-in|missing) \|", text, re.M)
counts = Counter(rows)
engine = (root / "crates/hive-core/src/testing/effectiveness.rs").read_text()
shots = [
    root / "docs/parity/screenshots/light/effectiveness.png",
    root / "docs/parity/screenshots/dark/effectiveness.png",
    root / "docs/parity/screenshots/light/effectiveness-charts.png",
    root / "docs/parity/screenshots/dark/effectiveness-charts.png",
]
missing = [str(path) for path in shots if not path.is_file() or path.stat().st_size < 1000]
wire = "measured_run_talks_to_the_anthropic_wire_and_writes_disk_and_rows" in engine
blind = "fn test_blind(" in (root / "crates/hive-cli/src/eval.rs").read_text()
lines = [
    f"real={counts['real']}",
    f"stand-in={counts['stand-in']}",
    f"missing={counts['missing']}",
    f"total={len(rows)}",
    f"wire_test={wire}",
    f"blind_command={blind}",
    f"screenshots={len(shots) - len(missing)}/{len(shots)}",
]
if missing:
    lines.append("missing_screenshots=" + ",".join(missing))
out = root / "automations/effectiveness-verify/out/result.md"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text("\n".join(lines) + "\n")
print(out.read_text())
