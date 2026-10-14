#!/usr/bin/env python3
"""Deterministic pre-flight from PLAN.md section 3.8.

This is the quick_validate.py contract Skill Hive agrees with: it stops at the
first failure, while the app reports every rule. The rule id is the portable
check that failed, so the fixture suite can compare error sets.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ALLOWED = {"name", "description", "license", "allowed-tools", "metadata", "compatibility"}
EXCLUDED_ANYWHERE = {"__pycache__", "node_modules"}
ROOT_EXCLUDED = {"evals"}


def fail(rule: str, message: str) -> int:
    print(json.dumps({"ok": False, "rule": rule, "message": message}))
    return 1


def ok() -> int:
    print(json.dumps({"ok": True, "rule": None, "message": "Skill is valid!"}))
    return 0


def packaged_skill_mds(root: Path) -> list[Path]:
    found: list[Path] = []

    def walk(directory: Path, at_root: bool) -> None:
        try:
            entries = list(directory.iterdir())
        except OSError:
            return
        for entry in entries:
            if entry.is_dir():
                name = entry.name
                if name in EXCLUDED_ANYWHERE:
                    continue
                if at_root and name in ROOT_EXCLUDED:
                    continue
                walk(entry, False)
            elif entry.is_file() and entry.name == "SKILL.md":
                found.append(entry)

    walk(root, True)
    return found


def split_lines(text: str) -> list[tuple[str, str]]:
    lines: list[tuple[str, str]] = []
    i = 0
    while i < len(text):
        nxt = text.find("\n", i)
        if nxt < 0:
            lines.append((text[i:], ""))
            break
        end = nxt
        nl = "\n"
        if end > i and text[end - 1] == "\r":
            end -= 1
            nl = "\r\n"
        lines.append((text[i:end], nl))
        i = nxt + 1
    return lines


def parse_scalar(raw: str) -> tuple[object, str | None]:
    value = raw.strip()
    if value == "" or value in {"~", "null", "Null", "NULL"}:
        return None, None
    if (value.startswith('"') and value.endswith('"') and len(value) >= 2) or (
        value.startswith("'") and value.endswith("'") and len(value) >= 2
    ):
        return value[1:-1], None
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        if not inner:
            return [], None
        return [part.strip().strip("'\"") for part in inner.split(",")], None
    if value in {"true", "True", "TRUE", "false", "False", "FALSE", "yes", "no", "on", "off"}:
        return value.lower() in {"true", "yes", "on"}, None
    if re.fullmatch(r"-?\d+", value):
        return int(value), None
    if value[0] in {">", "|"}:
        return value, "block"
    return value, None


def parse_frontmatter(inner: str) -> tuple[dict | None, str | None]:
    lines = split_lines(inner)
    if lines and lines[-1][0] == "" and lines[-1][1] == "":
        lines = lines[:-1]
    mapping: dict[str, object] = {}
    i = 0
    while i < len(lines):
        text, _nl = lines[i]
        if text.strip() == "":
            i += 1
            continue
        if text.startswith("- ") or text.startswith("-"):
            return None, "Frontmatter must be a YAML dictionary"
        match = re.match(r"^([A-Za-z_][A-Za-z0-9_-]*):(.*)$", text)
        if not match or text[0].isspace():
            return None, "Invalid YAML in frontmatter"
        key, rest = match.group(1), match.group(2)
        indicator = rest.strip()
        if indicator[:1] in {">", "|"} and re.fullmatch(r"[>|][+-]?\d*", indicator):
            block: list[str] = []
            i += 1
            while i < len(lines):
                nxt, _nl = lines[i]
                if nxt.strip() == "":
                    block.append("")
                    i += 1
                    continue
                if re.match(r"^[A-Za-z_][A-Za-z0-9_-]*:", nxt):
                    break
                if nxt.startswith(" ") or nxt.startswith("\t"):
                    block.append(nxt.strip())
                    i += 1
                    continue
                return None, "Invalid YAML in frontmatter"
            sep = "\n" if indicator.startswith("|") else " "
            text_value = sep.join(part for part in block if not (sep == " " and part == "")).strip()
            if indicator.startswith(">"):
                text_value = re.sub(r"\s+", " ", " ".join(block)).strip()
            mapping[key] = text_value
            continue
        if rest.strip() == "":
            nested: dict[str, str] = {}
            seq: list[str] = []
            i += 1
            while i < len(lines):
                nxt, _nl = lines[i]
                if nxt.strip() == "":
                    i += 1
                    continue
                if re.match(r"^[A-Za-z_][A-Za-z0-9_-]*:", nxt):
                    break
                item = re.match(r"^\s+-\s+(.*)$", nxt)
                child = re.match(r"^\s+([A-Za-z_][A-Za-z0-9_-]*):(.*)$", nxt)
                if item:
                    seq.append(item.group(1).strip().strip("'\""))
                elif child:
                    nested[child.group(1)] = child.group(2).strip().strip("'\"")
                else:
                    return None, "Invalid YAML in frontmatter"
                i += 1
            mapping[key] = seq if seq and not nested else nested
            continue
        parsed, err = parse_scalar(rest)
        if err == "block":
            return None, "Invalid YAML in frontmatter"
        mapping[key] = parsed
        i += 1
    return mapping, None


def validate(root: Path) -> int:
    skill_md = root / "SKILL.md"
    if not skill_md.is_file():
        return fail("S-01", "SKILL.md not found")
    found = packaged_skill_mds(root)
    if len(found) != 1:
        return fail("S-03", f"expected exactly one SKILL.md, found {len(found)}")
    content = skill_md.read_text(encoding="utf-8")
    if content.startswith("\ufeff"):
        content = content[1:]
    if not content.startswith("---"):
        return fail("S-04", "SKILL.md must start with YAML frontmatter (---)")
    match = re.match(r"^---\r?\n(.*?)\r?\n---", content, re.DOTALL)
    if not match:
        return fail("S-05", "Invalid frontmatter format")
    front, err = parse_frontmatter(match.group(1))
    if err or not isinstance(front, dict):
        return fail("S-06", err or "Frontmatter must be a YAML dictionary")
    unexpected = sorted(set(front) - ALLOWED)
    if unexpected:
        return fail("S-23", f"Unexpected key(s) in SKILL.md frontmatter: {', '.join(unexpected)}")
    if "name" not in front:
        return fail("S-07", "Missing 'name' in frontmatter")
    if "description" not in front:
        return fail("S-08", "Missing 'description' in frontmatter")
    name = front["name"]
    if not isinstance(name, str):
        return fail("S-09", "name must be a string")
    if not re.fullmatch(r"[a-z0-9-]+", name):
        return fail("S-10", "name must match ^[a-z0-9-]+$")
    if name.startswith("-") or name.endswith("-") or "--" in name:
        return fail("S-11", "name must not have leading, trailing, or doubled hyphens")
    if not 1 <= len(name) <= 64:
        return fail("S-12", "name must be 1 to 64 characters")
    description = front["description"]
    if not isinstance(description, str):
        return fail("S-16", "description must be a string")
    if "<" in description or ">" in description:
        return fail("S-18", "description must not contain angle brackets")
    if len(description) > 1024:
        return fail("S-17", "description must be at most 1024 characters")
    if "compatibility" in front:
        compatibility = front["compatibility"]
        if not isinstance(compatibility, str) or len(compatibility) > 500:
            return fail("S-20", "compatibility must be a string of at most 500 characters")
    return ok()


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: quick_validate.py <skill-directory>", file=sys.stderr)
        return 2
    return validate(Path(sys.argv[1]))


if __name__ == "__main__":
    sys.exit(main())
