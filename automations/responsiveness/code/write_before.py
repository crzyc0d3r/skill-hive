#!/usr/bin/env python3
"""Baseline captured before managed() stopped embedding skill file bodies."""

import json
from pathlib import Path

out = Path("automations/responsiveness/out/before.json")
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(
    json.dumps(
        {
            "host": "linux",
            "disk": "warm, files just written",
            "skills": 200,
            "projects": 4,
            "presets": 4,
            "seed_ms": 66.4,
            "list_ms": 7.0,
            "payload_bytes": 2612160,
            "projects_ms": 0.1,
            "presets_ms": 0.0,
            "line": "PROFILE seed_ms=66.4 list_ms=7.0 skills=200 payload_bytes=2612160 projects_ms=0.1 projects=4 presets_ms=0.0 presets=4",
        },
        indent=2,
    )
    + "\n"
)
