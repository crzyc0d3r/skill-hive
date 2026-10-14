#!/usr/bin/env python3
"""Publish the authoring before/after table."""

from pathlib import Path

bundle = Path(__file__).resolve().parents[1]
root = bundle.parents[1]
source = Path(__file__).with_name("authoring-wiring.md")
dest = root / "docs" / "authoring-wiring.md"
text = source.read_text(encoding="utf-8")
if "keyring" not in text or "download_and_install_update" not in text:
    raise SystemExit("authoring table is incomplete")
dest.write_text(text, encoding="utf-8")
print(f"wrote {dest} ({dest.stat().st_size} bytes)")
