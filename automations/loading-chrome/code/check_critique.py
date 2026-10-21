#!/usr/bin/env python3
"""Acceptance: the critique names the layouts and the screenshots exist."""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[3]
text = (root / "automations/loading-chrome/out/critique.md").read_text(encoding="utf-8")
needles = [
    "Library",
    "Dashboard",
    "Settings",
    "Install",
    "Effectiveness",
    "Syncing skills",
    "prefers-reduced-motion",
]
missing = [item for item in needles if item not in text]
shots = [
    "docs/parity/screenshots/light/loading-library.png",
    "docs/parity/screenshots/dark/loading-library.png",
    "docs/parity/screenshots/light/loading-dashboard.png",
    "docs/parity/screenshots/dark/loading-dashboard.png",
    "docs/parity/screenshots/light/loading-job.png",
    "docs/parity/screenshots/dark/loading-job.png",
    "docs/parity/screenshots/light/loading-settings.png",
    "docs/parity/screenshots/light/loading-effectiveness.png",
]
absent = [item for item in shots if not (root / item).is_file()]
css = (root / "src/styles/tokens.css").read_text(encoding="utf-8")
if "skeleton-shimmer" not in css or "prefers-reduced-motion" not in css:
    missing.append("shimmer css")
if missing or absent:
    print("missing", missing, "absent", absent)
    sys.exit(1)
print("critique ok", len(shots), "shots")
