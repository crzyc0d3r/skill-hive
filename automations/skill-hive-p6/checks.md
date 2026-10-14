# checks: skill-hive-p6

Run `r1` · updated 2026-10-14T05:50:23+00:00 · verdicts: pass, fail, unresolved

## Node gates

| node | requirement | verdict | evidence |
|---|---|---|---|
| n1_pipeline | a1 check.cmd | pass | `grep -q 'exit 0' automations/skill-hive-p6/out/n1_pipeline.md` → exit 0: (no output) · log `automations/skill-hive-p6/runs/r1/gates/n1_pipeline.a1.md` |
| n1_pipeline | a1 node.report | pass | exit 0: · log `automations/skill-hive-p6/runs/r1/gates/n1_pipeline.a1.md` |
| n2_updater | a1 check.cmd | pass | `grep -q 'exit 0' automations/skill-hive-p6/out/n2_updater.md` → exit 0: (no output) · log `automations/skill-hive-p6/runs/r1/gates/n2_updater.a1.md` |
| n2_updater | a1 node.report | pass | exit 0: · log `automations/skill-hive-p6/runs/r1/gates/n2_updater.a1.md` |
| n3_docs | a1 check.cmd | pass | `grep -q 'check-docs exit 0' automations/skill-hive-p6/out/n3_docs.md` → exit 0: (no output) · log `automations/skill-hive-p6/runs/r1/gates/n3_docs.a1.md` |
| n3_docs | a1 node.report | pass | exit 0: · log `automations/skill-hive-p6/runs/r1/gates/n3_docs.a1.md` |
| n4_result | (not run yet) | unresolved | — |

## Acceptance checks (the deliverable)

| check | verdict | evidence |
|---|---|---|
| 0. Phase 6 result records five exit criteria and the communities that fed each node | unresolved | no evidence recorded |

## Baseline before the run (2026-10-14T05:49:52+00:00)

| item | verdict | evidence |
|---|---|---|
| deliverable exists before the run | fail | automations/skill-hive-p6/out/result.md absent |
| acceptance 0: Phase 6 result records five exit criteria and the communities that fed each node | fail | `python3 automations/skill-hive-p6/code/check_result.py` → exit 1: missing automations/skill-hive-p6/out/result.md |

Observed before the run:
- Before the nodes: hive-core 209 tests, workspace cargo test, clippy, cargo deny, vitest 68, and the node release checks had already passed locally. The acceptance file result.md does not exist yet.
