#!/usr/bin/env python3
import json
from pathlib import Path
import sys

data = json.loads(Path("automations/responsiveness/out/after.json").read_text())
if data.get("skills") != 200 or data.get("projects") != 4:
    print("after profile did not load 200 skills and 4 projects")
    sys.exit(1)
if not isinstance(data.get("payload_bytes"), int) or not isinstance(data.get("list_ms"), float):
    print("after profile is missing timings")
    sys.exit(1)
print(f"after payload_bytes={data['payload_bytes']} list_ms={data['list_ms']}")
