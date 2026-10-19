# checks: effectiveness-verify

Run `r1` · updated 2026-10-18T15:51:44+00:00 · verdicts: pass, fail, unresolved

## Node gates

| node | requirement | verdict | evidence |
|---|---|---|---|
| n1_counts | a1 check.cmd (superseded) | pass | `python3 automations/effectiveness-verify/code/check_audit.py` → exit 0: real=36 stand-in=16 missing=11 total=63 · log `automations/effectiveness-verify/runs/r1/gates/n1_counts.a1.md` |
| n1_counts | a1 node.report (superseded) | pass | exit 0: · log `automations/effectiveness-verify/runs/r1/gates/n1_counts.a1.md` |
| n1_counts | a1 check.cmd | pass | `python3 automations/effectiveness-verify/code/check_audit.py` → exit 0: real=36 stand-in=16 missing=11 total=63 · log `automations/effectiveness-verify/runs/r1/gates/n1_counts.a1.md` |
| n1_counts | a1 node.report | pass | exit 0: · log `automations/effectiveness-verify/runs/r1/gates/n1_counts.a1.md` |
| n2_result | a1 check.cmd (superseded) | pass | `python3 automations/effectiveness-verify/code/check_result.py` → exit 0: real=36<br>stand-in=16<br>missing=11<br>total=63<br>wire_test=True<br>blind_command=True<br>screenshots=4/4 · log `automations/effectiveness-verify/runs/r1/gates/n2_result.a1.md` |
| n2_result | a1 node.report (superseded) | pass | exit 0: real=36<br>stand-in=16<br>missing=11<br>total=63<br>wire_test=True<br>blind_command=True<br>screenshots=4/4 · log `automations/effectiveness-verify/runs/r1/gates/n2_result.a1.md` |
| n2_result | a1 check.cmd | pass | `python3 automations/effectiveness-verify/code/check_result.py` → exit 0: real=36<br>stand-in=16<br>missing=11<br>total=63<br>wire_test=True<br>blind_command=True<br>screenshots=4/4 · log `automations/effectiveness-verify/runs/r1/gates/n2_result.a1.md` |
| n2_result | a1 node.report | pass | exit 0: real=36<br>stand-in=16<br>missing=11<br>total=63<br>wire_test=True<br>blind_command=True<br>screenshots=4/4 · log `automations/effectiveness-verify/runs/r1/gates/n2_result.a1.md` |

## Acceptance checks (the deliverable)

| check | verdict | evidence |
|---|---|---|
| 0. audit counts match the status rows | pass | `python3 automations/effectiveness-verify/code/check_audit.py` → exit 0: real=36 stand-in=16 missing=11 total=63 |
| 1. measured wire test and both theme screenshots exist | pass | result.md records wire_test=True, blind_command=True, and screenshots=4/4 for light and dark grade and chart views. |

## Baseline before the run (2026-10-18T15:51:38+00:00)

| item | verdict | evidence |
|---|---|---|
| deliverable exists before the run | fail | automations/effectiveness-verify/out/result.md absent |
| acceptance 0: audit counts match the status rows | pass | `python3 automations/effectiveness-verify/code/check_audit.py` → exit 0: real=36 stand-in=16 missing=11 total=63 |
| acceptance 1: measured wire test and both theme screenshots exist | unresolved | no cmd; judged at completion |

Observed before the run:
- audit already scored 36 real, 16 stand-in, 11 missing

## Completion (2026-10-18T15:51:44+00:00) — COMPLETE

| item | verdict | evidence |
|---|---|---|
| graph.md is a valid spec | pass | validate ok |
| spec approved before execution | pass | hash 8292b2d92ecce1e1 |
| n1_counts: output exists and can be opened | pass | automations/effectiveness-verify/out/counts.txt (40 bytes) |
| n1_counts: gate ran against the saved version | pass | gate pass at 2026-10-18T15:51:38+00:00 |
| n2_result: output exists and can be opened | pass | automations/effectiveness-verify/out/result.md (90 bytes) |
| n2_result: gate ran against the saved version | pass | gate pass at 2026-10-18T15:51:39+00:00 |
| graph.md matches the nodes that actually ran | pass | ran 2 of 2 declared nodes |
| acceptance 0: audit counts match the status rows | pass | `python3 automations/effectiveness-verify/code/check_audit.py` → exit 0: real=36 stand-in=16 missing=11 total=63 |
| acceptance 1: measured wire test and both theme screenshots exist | pass | result.md records wire_test=True, blind_command=True, and screenshots=4/4 for light and dark grade and chart views. |
| deliverable exists | pass | automations/effectiveness-verify/out/result.md |
| run limit respected | pass | 0/3 model dispatches |
