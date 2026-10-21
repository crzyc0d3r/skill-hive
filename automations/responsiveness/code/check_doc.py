#!/usr/bin/env python3
"""The responsiveness note must keep the measured before and after numbers."""

from pathlib import Path
import sys

doc = Path("docs/responsiveness.md")
text = doc.read_text() if doc.exists() else ""
required = [
    "payload_bytes=2612160",
    "list_ms=7.0",
    "skills=200",
    "projects=4",
    "After",
]
missing = [item for item in required if item not in text]
if missing or "payload_bytes=" not in text.split("After", 1)[-1]:
    print("missing", ", ".join(missing) or "an after payload_bytes")
    sys.exit(1)
print("responsiveness.md has before and after timings")
