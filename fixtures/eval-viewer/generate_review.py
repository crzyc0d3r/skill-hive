#!/usr/bin/env python3
"""Static review for a skill-creator workspace exported by Skill Hive.

Usage: generate_review.py WORKSPACE --static OUTPUT
Exits 0 when every eval directory has metadata and benchmark.json.
Exits 1 when the layout is missing a required file.
"""

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("workspace")
    parser.add_argument("--static", dest="static_path", required=True)
    args = parser.parse_args()
    root = Path(args.workspace)
    sections = []
    iterations = sorted(path for path in root.glob("iteration-*") if path.is_dir())
    if not iterations:
        print("no iteration directory", file=sys.stderr)
        return 1
    for iteration in iterations:
        benchmark = iteration / "benchmark.json"
        if not benchmark.is_file():
            print(f"missing {benchmark}", file=sys.stderr)
            return 1
        evals = sorted(
            path
            for path in iteration.iterdir()
            if path.is_dir() and path.name.startswith("eval-")
        )
        if not evals:
            print(f"no evals under {iteration}", file=sys.stderr)
            return 1
        for eval_dir in evals:
            meta_path = eval_dir / "eval_metadata.json"
            if not meta_path.is_file():
                print(f"missing {meta_path}", file=sys.stderr)
                return 1
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
            name = str(meta.get("eval_name") or eval_dir.name)
            sections.append(name)
            for config in ("with_skill", "without_skill"):
                grading = eval_dir / config / "grading.json"
                if grading.is_file():
                    sections.append(f"{name}:{config}")
    html = "<html><body>\n" + "\n".join(
        f"<section>{name}</section>" for name in sections
    ) + "\n</body></html>\n"
    Path(args.static_path).write_text(html, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
