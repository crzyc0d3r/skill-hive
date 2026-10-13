#!/usr/bin/env python3
"""Confirm the Phase 0 shell files exist and record that in the node output."""

from __future__ import annotations

import os
from pathlib import Path

ROOT = Path(os.environ.get("PROJECT_ROOT", Path(__file__).resolve().parents[3]))
OUT = Path(os.environ.get("NODE_OUTPUT", ROOT / "automations/skill-hive-p0/out/n6_shell.md"))

REQUIRED = [
    "src/components/shell/AppShell.tsx",
    "src/styles/tokens.css",
    "src/components/ui/button.tsx",
    "src/components/ui/sidebar.tsx",
    "src/components/ui/command.tsx",
    "src/lib/ipc/settings.ts",
    "src/lib/events/files.ts",
    "src/test/shell.test.tsx",
    "src/test/mock-ipc.test.ts",
    "src-tauri/tauri.conf.json",
    "src-tauri/tauri.macos.conf.json",
    "src-tauri/capabilities/default.json",
]

missing = [path for path in REQUIRED if not (ROOT / path).is_file()]
conf = (ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8")
shell = (ROOT / "src/components/shell/AppShell.tsx").read_text(encoding="utf-8")
problems = list(missing)
if "com.skillhive.desktop" not in conf:
    problems.append("identifier")
if "1440" not in conf or "1100" not in conf:
    problems.append("window size")
if "No matches for" not in shell:
    problems.append("palette empty state")
if 'role="status"' not in shell:
    problems.append("status bar")
if "data-testid=\"sidebar\"" not in shell:
    problems.append("sidebar")

lines = ["# Shell", ""]
if problems:
    lines.append("fail: " + ", ".join(problems))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    raise SystemExit(1)

lines.append("pass: sidebar, header, status bar, command palette, tokens, shadcn primitives, mockIPC harness")
lines.append("window 1440x860 min 1100x640, identifier com.skillhive.desktop, macOS overlay title bar in tauri.macos.conf.json")
OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(OUT)
