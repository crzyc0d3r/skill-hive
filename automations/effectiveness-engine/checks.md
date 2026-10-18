# checks: effectiveness-engine

Run `r1` · updated 2026-10-18T14:45:15+00:00 · verdicts: pass, fail, unresolved

## Node gates

| node | requirement | verdict | evidence |
|---|---|---|---|
| n1_audit | a1 check.cmd | pass | `python3 automations/effectiveness-engine/code/check_audit.py` → exit 0: real=38 stand-in=12 missing=13 total=63 · log `automations/effectiveness-engine/runs/r1/gates/n1_audit.a1.md` |
| n1_audit | a1 node.report | pass | docs/plan-conformance-audit.md has 63 rows: 38 real, 12 stand-in, 13 missing. Stand-in rows are listed separately and the summary says they are not done. · log `automations/effectiveness-engine/runs/r1/gates/n1_audit.a1.md` |
| n2_counts | a1 check.cmd | pass | `python3 automations/effectiveness-engine/code/check_counts.py` → exit 0: real=38<br>stand-in=12<br>missing=13<br>total=63 · log `automations/effectiveness-engine/runs/r1/gates/n2_counts.a1.md` |
| n2_counts | a1 node.report | pass | exit 0: real=38<br>stand-in=12<br>missing=13<br>total=63 · log `automations/effectiveness-engine/runs/r1/gates/n2_counts.a1.md` |

## Acceptance checks (the deliverable)

| check | verdict | evidence |
|---|---|---|
| 0. summary counts match the status rows | pass | `python3 automations/effectiveness-engine/code/check_audit.py` → exit 0: real=38 stand-in=12 missing=13 total=63 |
| 1. stand-in rows are not counted as done | pass | The summary says stand-in rows are not done, and the 12 stand-in rows name ScriptedProvider, frozen fixtures, the fake CLI binary, or a screen that does not call the engine. |

## Baseline before the run (2026-10-18T14:45:03+00:00)

| item | verdict | evidence |
|---|---|---|
| deliverable exists before the run | pass | docs/plan-conformance-audit.md (19036 bytes) |
| acceptance 0: summary counts match the status rows | pass | `python3 automations/effectiveness-engine/code/check_audit.py` → exit 0: real=38 stand-in=12 missing=13 total=63 |
| acceptance 1: stand-in rows are not counted as done | unresolved | no cmd; judged at completion |

Observed before the run:
- audit already drafted; engine tests and screen tests passed before the graph ran

## Completion (2026-10-18T14:45:15+00:00) — COMPLETE

| item | verdict | evidence |
|---|---|---|
| graph.md is a valid spec | pass | validate ok |
| spec approved before execution | pass | hash b9be045485d5a61a |
| n1_audit: output exists and can be opened | pass | docs/plan-conformance-audit.md (19036 bytes) |
| n1_audit: gate ran against the saved version | pass | gate pass at 2026-10-18T14:45:08+00:00 |
| n2_counts: output exists and can be opened | pass | automations/effectiveness-engine/out/counts.txt (40 bytes) |
| n2_counts: gate ran against the saved version | pass | gate pass at 2026-10-18T14:45:08+00:00 |
| graph.md matches the nodes that actually ran | pass | ran 2 of 2 declared nodes |
| acceptance 0: summary counts match the status rows | pass | `python3 automations/effectiveness-engine/code/check_audit.py` → exit 0: real=38 stand-in=12 missing=13 total=63 |
| acceptance 1: stand-in rows are not counted as done | pass | The summary says stand-in rows are not done, and the 12 stand-in rows name ScriptedProvider, frozen fixtures, the fake CLI binary, or a screen that does not call the engine. |
| deliverable exists | pass | docs/plan-conformance-audit.md |
| run limit respected | pass | 1/3 model dispatches |
