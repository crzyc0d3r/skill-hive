#!/usr/bin/env python3
"""Confirm before and after audit shots exist for both themes and both sizes."""
from pathlib import Path
import sys

root = Path("docs/ui-audit")
needed = [
    "dashboard",
    "library",
    "agents",
    "install-local",
    "workspace",
    "backup",
    "settings-appearance",
    "skill-editor",
    "command-palette",
]
missing = []
for phase in ("before", "after"):
    for theme in ("light", "dark"):
        for size in ("1100x640", "1440x860"):
            folder = root / phase / theme / size
            if not folder.is_dir():
                missing.append(str(folder))
                continue
            for name in needed:
                path = folder / f"{name}.png"
                if not path.is_file() or path.stat().st_size < 1000:
                    missing.append(str(path))
if missing:
    print("missing shots:")
    print("\n".join(missing))
    sys.exit(1)
print(f"shots ok ({len(needed)} names x 2 themes x 2 sizes x before/after)")
