#!/usr/bin/env python3
"""Population benchmark deltas. Matches hive-core testing::aggregate."""

import json
import math
import sys
from pathlib import Path


def population_stddev(values):
    if not values:
        return 0.0
    mean = sum(values) / len(values)
    variance = sum((value - mean) ** 2 for value in values) / len(values)
    return math.sqrt(variance)


def mean(values):
    if not values:
        return 0.0
    return sum(values) / len(values)


def round4(value):
    return round(value * 10000.0) / 10000.0


def main():
    iteration = Path(sys.argv[1])
    rows = []
    for eval_dir in sorted(path for path in iteration.iterdir() if path.is_dir()):
        for config in ("with_skill", "without_skill"):
            for run_number in (1, 2, 3):
                folder = eval_dir / (config if run_number == 1 else f"{config}-r{run_number}")
                grading_path = folder / "grading.json"
                if not grading_path.is_file():
                    continue
                grading = json.loads(grading_path.read_text())
                timing = json.loads((folder / "timing.json").read_text()) if (folder / "timing.json").is_file() else {}
                summary = grading.get("summary") or {}
                rows.append(
                    {
                        "configuration": config,
                        "pass_rate": float(summary.get("pass_rate") or 0),
                        "time_seconds": float(timing.get("total_duration_seconds") or 0),
                        "tokens": float(timing.get("total_tokens") or 0),
                    }
                )
    summary = {}
    for config in ("with_skill", "without_skill"):
        chosen = [row for row in rows if row["configuration"] == config]
        summary[config] = {
            "pass_rate": round4(mean([row["pass_rate"] for row in chosen])),
            "time_seconds": round4(mean([row["time_seconds"] for row in chosen])),
            "tokens": round4(mean([row["tokens"] for row in chosen])),
        }
    with_skill = summary["with_skill"]
    without = summary["without_skill"]
    delta = {
        "pass_rate": f"{with_skill['pass_rate'] - without['pass_rate']:+.2f}",
        "time_seconds": f"{with_skill['time_seconds'] - without['time_seconds']:+.1f}",
        "tokens": f"{round(with_skill['tokens'] - without['tokens']):+d}",
    }
    json.dump({"delta": delta, "stddev_with_skill": population_stddev([row["pass_rate"] for row in rows if row["configuration"] == "with_skill"])}, sys.stdout)


if __name__ == "__main__":
    main()
