# checks: skill-hive-p6

Run `r1` · updated 2026-10-14T06:17:16+00:00 · verdicts: pass, fail, unresolved

## Node gates

| node | requirement | verdict | evidence |
|---|---|---|---|
| n1_pipeline | a1 check.cmd | pass | `grep -q 'exit 0' automations/skill-hive-p6/out/n1_pipeline.md` → exit 0: (no output) · log `automations/skill-hive-p6/runs/r1/gates/n1_pipeline.a1.md` |
| n1_pipeline | a1 node.report | pass | exit 0: · log `automations/skill-hive-p6/runs/r1/gates/n1_pipeline.a1.md` |
| n2_updater | a1 check.cmd | pass | `grep -q 'exit 0' automations/skill-hive-p6/out/n2_updater.md` → exit 0: (no output) · log `automations/skill-hive-p6/runs/r1/gates/n2_updater.a1.md` |
| n2_updater | a1 node.report | pass | exit 0: · log `automations/skill-hive-p6/runs/r1/gates/n2_updater.a1.md` |
| n3_docs | a1 check.cmd | pass | `grep -q 'check-docs exit 0' automations/skill-hive-p6/out/n3_docs.md` → exit 0: (no output) · log `automations/skill-hive-p6/runs/r1/gates/n3_docs.a1.md` |
| n3_docs | a1 node.report | pass | exit 0: · log `automations/skill-hive-p6/runs/r1/gates/n3_docs.a1.md` |
| n4_result | a1 check.cmd | pass | `python3 automations/skill-hive-p6/code/check_result.py` → exit 0: result.md records five Phase 6 criteria and the graphify communities · log `automations/skill-hive-p6/runs/r1/gates/n4_result.a1.md` |
| n4_result | a1 node.report | pass | exit 0: · log `automations/skill-hive-p6/runs/r1/gates/n4_result.a1.md` |

## Acceptance checks (the deliverable)

| check | verdict | evidence |
|---|---|---|
| 0. Phase 6 result records five exit criteria and the communities that fed each node | pass | `python3 automations/skill-hive-p6/code/check_result.py` → exit 0: result.md records five Phase 6 criteria and the graphify communities |

## Baseline before the run (2026-10-14T05:49:52+00:00)

| item | verdict | evidence |
|---|---|---|
| deliverable exists before the run | fail | automations/skill-hive-p6/out/result.md absent |
| acceptance 0: Phase 6 result records five exit criteria and the communities that fed each node | fail | `python3 automations/skill-hive-p6/code/check_result.py` → exit 1: missing automations/skill-hive-p6/out/result.md |

Observed before the run:
- Before the nodes: hive-core 209 tests, workspace cargo test, clippy, cargo deny, vitest 68, and the node release checks had already passed locally. The acceptance file result.md does not exist yet.

## Completion (2026-10-14T06:17:16+00:00) — COMPLETE

| item | verdict | evidence |
|---|---|---|
| graph.md is a valid spec | pass | validate ok |
| spec approved before execution | pass | hash 2be7e7acd22a8a24 |
| n1_pipeline: output exists and can be opened | pass | automations/skill-hive-p6/out/n1_pipeline.md (2320 bytes) |
| n1_pipeline: gate ran against the saved version | pass | gate pass at 2026-10-14T05:50:17+00:00 |
| n2_updater: output exists and can be opened | pass | automations/skill-hive-p6/out/n2_updater.md (1385 bytes) |
| n2_updater: gate ran against the saved version | pass | gate pass at 2026-10-14T05:50:23+00:00 |
| n3_docs: output exists and can be opened | pass | automations/skill-hive-p6/out/n3_docs.md (154 bytes) |
| n3_docs: gate ran against the saved version | pass | gate pass at 2026-10-14T05:50:23+00:00 |
| n4_result: output exists and can be opened | pass | automations/skill-hive-p6/out/result.md (7810 bytes) |
| n4_result: gate ran against the saved version | pass | gate pass at 2026-10-14T06:17:16+00:00 |
| graph.md matches the nodes that actually ran | pass | ran 4 of 4 declared nodes |
| acceptance 0: Phase 6 result records five exit criteria and the communities that fed each node | pass | `python3 automations/skill-hive-p6/code/check_result.py` → exit 0: result.md records five Phase 6 criteria and the graphify communities |
| deliverable exists | pass | automations/skill-hive-p6/out/result.md |
| run limit respected | pass | 0/3 model dispatches |
