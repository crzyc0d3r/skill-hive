#!/usr/bin/env python3
"""Turn synchronous Tauri commands that do I/O into async commands.

The command body runs inside tauri::async_runtime::spawn_blocking via
crate::offload::offload, so the UI thread is not blocked.
"""

from __future__ import annotations

import pathlib
import re
import sys

IO_MARKERS = (
    "store",
    "std::fs",
    "fs::",
    "File::",
    "read_to_string",
    "read_dir",
    "write(",
    "OpenOptions",
    "remove_file",
    "remove_dir",
    "create_dir",
    "Command::",
    "std::process",
    "process::",
    "git_",
    "reqwest",
    "ureq",
    "fetch_https",
    "central_repo",
    "installer::",
    "scanner::",
    "SkillStore",
    ".emit(",
    "metadata(",
    "canonicalize",
    "sqlite",
    "rusqlite",
    "git_backup::",
    "updater::",
    "hive_core::testing",
)

ROOT = pathlib.Path(__file__).resolve().parents[1]
FILES = [
    ROOT / "src-tauri/src/commands.rs",
    ROOT / "src-tauri/src/lib.rs",
    ROOT / "src-tauri/src/parity_backup.rs",
    ROOT / "src-tauri/src/parity_presets.rs",
    ROOT / "src-tauri/src/parity_projects.rs",
    ROOT / "src-tauri/src/parity_shell.rs",
    ROOT / "src-tauri/src/parity_skills.rs",
    ROOT / "src-tauri/src/parity_workspace.rs",
]


def find_commands(text: str) -> list[tuple[int, int]]:
    spans = []
    for match in re.finditer(r"#\[tauri::command[^\]]*\]\n", text):
        start = match.start()
        # function starts at the next fn
        fn = re.search(r"\b(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\b", text[match.end() : match.end() + 400])
        if not fn:
            continue
        brace = text.find("{", match.end() + fn.end())
        if brace < 0:
            continue
        depth = 0
        i = brace
        while i < len(text):
            char = text[i]
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    spans.append((start, i + 1))
                    break
            i += 1
    return spans


def transform_command(block: str) -> str:
    if re.search(r"\basync\s+fn\b", block.split("{", 1)[0]):
        return block
    header, body = block.split("{", 1)
    body = body.rsplit("}", 1)[0]
    if not any(marker in body or marker in header for marker in IO_MARKERS):
        return block
    header = re.sub(r"\bfn\b", "async fn", header, count=1)
    clones = []
    if re.search(r"\bstore\s*:", header):
        clones.append("    let store = store.inner().clone();")
    if re.search(r"\bapp\s*:", header):
        clones.append("    let app = app.clone();")
    rewritten = body.replace("store.inner().clone()", "store.clone()")
    rewritten = rewritten.replace("store.inner()", "store.as_ref()")
    indented = "\n".join(("    " + line) if line.strip() else line for line in rewritten.splitlines())
    clone_block = ("\n".join(clones) + "\n") if clones else ""
    return (
        f"{header}{{\n"
        f"{clone_block}"
        f"    crate::offload::offload(move || {{{indented}\n    }}).await\n"
        f"}}"
    )


def process(path: pathlib.Path) -> int:
    text = path.read_text()
    spans = find_commands(text)
    changed = 0
    pieces = []
    cursor = 0
    for start, end in spans:
        pieces.append(text[cursor:start])
        original = text[start:end]
        updated = transform_command(original)
        if updated != original:
            changed += 1
        pieces.append(updated)
        cursor = end
    pieces.append(text[cursor:])
    path.write_text("".join(pieces))
    return changed


def main() -> int:
    total = 0
    for path in FILES:
        count = process(path)
        print(f"{path.relative_to(ROOT)}: {count}")
        total += count
    print(f"converted {total}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
