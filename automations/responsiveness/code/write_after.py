#!/usr/bin/env python3
"""Pull the latest PROFILE line out of the library profile log."""

import json
import re
from pathlib import Path
import sys

logs = [
    Path("automations/responsiveness/out/profile.log"),
    Path("/tmp/profile.log"),
    Path("/tmp/cargo-lib.log"),
]
text = ""
for path in logs:
    if path.exists():
        text += path.read_text(errors="replace")
matches = re.findall(r"PROFILE seed_ms=(\S+) list_ms=(\S+) skills=(\d+) payload_bytes=(\d+) projects_ms=(\S+) projects=(\d+) presets_ms=(\S+) presets=(\d+)", text)
if not matches:
    print("no PROFILE line in the profile log")
    sys.exit(1)
seed_ms, list_ms, skills, payload, projects_ms, projects, presets_ms, presets = matches[-1]
out = Path("automations/responsiveness/out/after.json")
out.write_text(
    json.dumps(
        {
            "seed_ms": float(seed_ms),
            "list_ms": float(list_ms),
            "skills": int(skills),
            "payload_bytes": int(payload),
            "projects_ms": float(projects_ms),
            "projects": int(projects),
            "presets_ms": float(presets_ms),
            "presets": int(presets),
            "line": (
                f"PROFILE seed_ms={seed_ms} list_ms={list_ms} skills={skills} "
                f"payload_bytes={payload} projects_ms={projects_ms} projects={projects} "
                f"presets_ms={presets_ms} presets={presets}"
            ),
        },
        indent=2,
    )
    + "\n"
)
print(out.read_text())
