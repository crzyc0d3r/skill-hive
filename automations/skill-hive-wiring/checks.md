# checks: skill-hive-wiring

Run `r1` · updated 2026-10-14T17:06:53+00:00 · verdicts: pass, fail, unresolved

## Node gates

| node | requirement | verdict | evidence |
|---|---|---|---|
| n1_commands | a1 check.cmd | pass | `grep -q 'ipc-ok' automations/skill-hive-wiring/out/n1_commands.md` → exit 0: (no output) · log `automations/skill-hive-wiring/runs/r1/gates/n1_commands.a1.md` |
| n1_commands | a1 node.report | pass | exit 0: src/lib/ipc/ipc.d.ts matches get_settings, set_settings, log_startup_event, check_app_update, update_install_blocker, updater_proxy, list_base_entries, get_managed_skills, list_presets, get_tool_status, scan_local_skills, install_local_skill, install_archive_skill, import_existing_skill, im… · log `automations/skill-hive-wiring/runs/r1/gates/n1_commands.a1.md` |
| n2_audit | a1 check.cmd | pass | `grep -q 'audit-ok' automations/skill-hive-wiring/out/n2_audit.md` → exit 0: (no output) · log `automations/skill-hive-wiring/runs/r1/gates/n2_audit.a1.md` |
| n2_audit | a1 node.report | pass | exit 0: · log `automations/skill-hive-wiring/runs/r1/gates/n2_audit.a1.md` |
| n3_result | a1 check.cmd | pass | `python3 automations/skill-hive-wiring/code/check_result.py` → exit 0: result-ok · log `automations/skill-hive-wiring/runs/r1/gates/n3_result.a1.md` |
| n3_result | a1 node.report | pass | exit 0: · log `automations/skill-hive-wiring/runs/r1/gates/n3_result.a1.md` |

## Acceptance checks (the deliverable)

| check | verdict | evidence |
|---|---|---|
| 0. audit exists | pass | `test -s docs/wiring-audit.md` → exit 0: (no output) |
| 1. ipc types match commands | pass | `node scripts/gen-ipc-types.mjs --check` → exit 0: src/lib/ipc/ipc.d.ts matches get_settings, set_settings, log_startup_event, check_app_update, update_install_blocker, updater_proxy, list_base_entries, get_managed_skills, list_presets, get_tool_status, scan_local_skills, install_local_skill, insta… |

## Baseline before the run (2026-10-14T17:06:31+00:00)

| item | verdict | evidence |
|---|---|---|
| deliverable exists before the run | pass | docs/wiring-audit.md (8711 bytes) |
| acceptance 0: audit exists | pass | `test -s docs/wiring-audit.md` → exit 0: (no output) |
| acceptance 1: ipc types match commands | pass | `node scripts/gen-ipc-types.mjs --check` → exit 0: src/lib/ipc/ipc.d.ts matches get_settings, set_settings, log_startup_event, check_app_update, update_install_blocker, updater_proxy, list_base_entries, get_managed_skills, list_presets, get_tool_status, scan_local_skills, install_local_skill, insta… |

Observed before the run:
- unit tests 68 passed before the verification nodes

## Completion (2026-10-14T17:06:53+00:00) — COMPLETE

| item | verdict | evidence |
|---|---|---|
| graph.md is a valid spec | pass | validate ok |
| spec approved before execution | pass | hash 335e6271a3621d66 |
| n1_commands: output exists and can be opened | pass | automations/skill-hive-wiring/out/n1_commands.md (7 bytes) |
| n1_commands: gate ran against the saved version | pass | gate pass at 2026-10-14T17:06:47+00:00 |
| n2_audit: output exists and can be opened | pass | automations/skill-hive-wiring/out/n2_audit.md (9 bytes) |
| n2_audit: gate ran against the saved version | pass | gate pass at 2026-10-14T17:06:47+00:00 |
| n3_result: output exists and can be opened | pass | automations/skill-hive-wiring/out/result.md (84 bytes) |
| n3_result: gate ran against the saved version | pass | gate pass at 2026-10-14T17:06:47+00:00 |
| graph.md matches the nodes that actually ran | pass | ran 3 of 3 declared nodes |
| acceptance 0: audit exists | pass | `test -s docs/wiring-audit.md` → exit 0: (no output) |
| acceptance 1: ipc types match commands | pass | `node scripts/gen-ipc-types.mjs --check` → exit 0: src/lib/ipc/ipc.d.ts matches get_settings, set_settings, log_startup_event, check_app_update, update_install_blocker, updater_proxy, list_base_entries, get_managed_skills, list_presets, get_tool_status, scan_local_skills, install_local_skill, insta… |
| deliverable exists | pass | docs/wiring-audit.md |
| run limit respected | pass | 0/6 model dispatches |
