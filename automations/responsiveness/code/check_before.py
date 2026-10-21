#!/usr/bin/env python3
import json
from pathlib import Path
import sys

data = json.loads(Path("automations/responsiveness/out/before.json").read_text())
if data.get("payload_bytes") != 2612160 or data.get("list_ms") != 7.0 or data.get("skills") != 200:
    print("before.json does not match the measured baseline")
    sys.exit(1)
print("before.json matches the measured baseline")
