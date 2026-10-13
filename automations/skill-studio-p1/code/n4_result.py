#!/usr/bin/env python3
import os
from pathlib import Path

root = Path(os.environ["PROJECT_ROOT"])
out = root / "automations/skill-studio-p1/out"
chunks = [(out / name).read_text(encoding="utf-8") for name in ("n1_rust.md", "n2_frontend.md", "n3_visual.md")]
text = "\n".join(chunks)

def ok(label: str) -> bool:
    return f"| {label} | 0 |" in text

cargo = 0 if ok("cargo test") else 1
npm = 0 if ok("npm test") else 1
playwright = 0 if ok("playwright") else 1
body = f"""# Phase 1 result

| cargo test | {cargo} |
| npm test | {npm} |
| playwright | {playwright} |

## M1 exit criteria

1. fail. `docs/parity/m1-ledger.md` still lists partial must rows, so the ledger does not show every must row as done.
2. pass. Symlink deploy, the Windows junction fallback, UNC copy, and TARGET_CONFLICT are covered by `sync_engine` tests and `phase1_tests::foreign_directory_conflict_names_the_path`. Three-OS evidence is the Test workflow.
3. pass. `phase1_tests::skills_manager_import_reproduces_counts_and_order` reproduces 12 skills, 3 presets, tag assignments, and membership order.
4. pass. `fifty_four_adapters_have_global_and_project_directories`, `custom_agent_collision_is_skipped_and_order_persists`, and `path_overrides_round_trip_and_reset`.
5. pass. `phase1_tests::install_deploy_preset_undeploy_scan_and_import`.
6. fail. Playwright light and dark checks at 1100x640 passed (`e2e/phase1-visual.spec.ts`). The WebKitGTK and WKWebView review was not performed (`docs/parity/dx-review-m1.md`).

graphify is not installed, so this split was not graph-informed.
"""
Path(os.environ["NODE_OUTPUT"]).write_text(body, encoding="utf-8")
if cargo or npm or playwright:
    raise SystemExit(1)
