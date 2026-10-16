#!/usr/bin/env python3
"""Write the graph deliverable after the shot check passes."""
import subprocess
from pathlib import Path

check = subprocess.run(["python3", "automations/skill-hive-ui/code/check_shots.py"], capture_output=True, text=True)
if check.returncode != 0:
    print(check.stdout)
    print(check.stderr)
    raise SystemExit(check.returncode)

out = Path("automations/skill-hive-ui/out/result.md")
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(
    """# Skill Hive UI

The screen layer now uses the shadcn controls and the Part 4 tokens.

- Before shots: `docs/ui-audit/before/{light,dark}/{1100x640,1440x860}/`
- After shots: `docs/ui-audit/after/{light,dark}/{1100x640,1440x860}/`
- Deviation list: `docs/ui-audit/DEVIATIONS.md`
- Visual suite: `e2e/ui-regression.spec.ts` (no horizontal scroll at 1100x640 and 1440x860, heading contrast at least 4.5:1, reduced motion)

"""
    + check.stdout,
    encoding="utf-8",
)
print(check.stdout.strip())
