#!/usr/bin/env python3
"""Confirm Phase 0 CI, attribution, and fixture files exist."""

from __future__ import annotations

import os
from pathlib import Path

ROOT = Path(os.environ.get("PROJECT_ROOT", Path(__file__).resolve().parents[3]))
OUT = Path(os.environ.get("NODE_OUTPUT", ROOT / "automations/skill-studio-p0/out/n7_ci.md"))

REQUIRED = [
    ".github/workflows/test.yml",
    ".github/workflows/frontend.yml",
    ".github/workflows/release.yml",
    "THIRD_PARTY_LICENSES/skills-manager.MIT.txt",
    "NOTICE",
    "deny.toml",
    "docs/dev/toolchain.md",
    "docs/dev/first-hour.md",
    "docs/procurement/apple-developer-id.md",
    "docs/procurement/app-store-connect-api-key.md",
    "docs/procurement/windows-signing-account.md",
    "fixtures/skills/valid/SKILL.md",
    "fixtures/skills/warnings/SKILL.md",
    "fixtures/skills/errors/SKILL.md",
    "fixtures/skills/errors/missing-name/SKILL.md",
    "scripts/prepare-release.mjs",
    "scripts/check-headers.mjs",
    "scripts/gen-ipc-types.mjs",
]

missing = [path for path in REQUIRED if not (ROOT / path).is_file()]
release = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
test_yml = (ROOT / ".github/workflows/test.yml").read_text(encoding="utf-8")
problems = list(missing)
for needle in (
    "ubuntu-22.04",
    "ubuntu-24.04-arm",
    "macos-latest",
    "windows-latest",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "releaseDraft: true",
    "timeout-minutes: 60",
):
    if needle not in release:
        problems.append(f"release.yml missing {needle}")
for needle in ("ubuntu-22.04", "macos-latest", "windows-latest", "save-if:"):
    if needle not in test_yml:
        problems.append(f"test.yml missing {needle}")
notice = (ROOT / "NOTICE").read_text(encoding="utf-8")
if "73d6b8f77bba6719924885c1f3be56b828d5a192" not in notice:
    problems.append("NOTICE commit")
if "com.agentskills.desktop" in (ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"):
    problems.append("upstream identifier reused")

lines = ["# CI and attribution", ""]
OUT.parent.mkdir(parents=True, exist_ok=True)
if problems:
    lines.append("fail: " + ", ".join(problems))
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    raise SystemExit(1)
lines.append("pass: test matrix on ubuntu, macos, and windows; unsigned five-target release workflow; MIT attribution; fixtures; procurement notes")
OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(OUT)
