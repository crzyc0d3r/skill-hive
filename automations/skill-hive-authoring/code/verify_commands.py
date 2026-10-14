#!/usr/bin/env python3
"""Confirm dedicated authoring commands replaced record_effect."""

from pathlib import Path

root = Path(__file__).resolve().parents[3]
needed = [
    "save_skill_draft",
    "snapshot_skill_draft",
    "restore_skill_draft",
    "publish_skill_draft",
    "fix_skill_draft",
    "save_agent_draft",
    "export_agent_draft",
    "save_generated_draft",
    "apply_optimized_description",
    "record_compare_verdict",
    "write_history_baseline",
    "set_provider_secret",
    "store_backup_pat",
    "revoke_backup_token",
    "set_aside_backup",
    "pull_project_center",
    "export_diagnostics",
    "download_and_install_update",
    "cancel_market_install",
    "git_backup_restore_remote",
    "rebuild_tray_menu",
]
sources = []
for folder in ("src", "src-tauri"):
    for path in (root / folder).rglob("*"):
        if path.suffix in {".rs", ".ts", ".tsx"} and "target" not in path.parts:
            sources.append(path.read_text(encoding="utf-8", errors="replace"))
blob = "\n".join(sources)
if "record_effect" in blob:
    raise SystemExit("record_effect is still referenced")
if (root / "src/lib/ipc/persist.ts").exists():
    raise SystemExit("persist.ts is still present")
missing = [name for name in needed if name not in blob]
if missing:
    raise SystemExit("missing commands: " + ", ".join(missing))
out = Path(__file__).resolve().parents[1] / "out" / "n1_verify.md"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(f"verified {len(needed)} authoring commands and no record_effect\n", encoding="utf-8")
print(out.read_text(encoding="utf-8").strip())
