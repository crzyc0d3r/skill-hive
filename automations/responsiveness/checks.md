# checks: responsiveness

Run `r1` · updated 2026-10-21T19:40:10+00:00 · verdicts: pass, fail, unresolved

## Node gates

| node | requirement | verdict | evidence |
|---|---|---|---|
| n1_before | a1 check.cmd (superseded) | pass | `python3 automations/responsiveness/code/check_before.py` → exit 0: before.json matches the measured baseline · log `automations/responsiveness/runs/r1/gates/n1_before.a1.md` |
| n1_before | a1 node.report (superseded) | pass | exit 0: · log `automations/responsiveness/runs/r1/gates/n1_before.a1.md` |
| n1_before | a1 check.cmd | pass | `python3 automations/responsiveness/code/check_before.py` → exit 0: before.json matches the measured baseline · log `automations/responsiveness/runs/r1/gates/n1_before.a1.md` |
| n1_before | a1 node.report | pass | exit 0: · log `automations/responsiveness/runs/r1/gates/n1_before.a1.md` |
| n2_after | a1 check.cmd (superseded) | pass | `python3 automations/responsiveness/code/check_after.py` → exit 0: after payload_bytes=0 list_ms=0.7 · log `automations/responsiveness/runs/r1/gates/n2_after.a1.md` |
| n2_after | a1 node.report (superseded) | pass | exit 0: {<br>"seed_ms": 67.0,<br>"list_ms": 0.7,<br>"skills": 200,<br>"payload_bytes": 0,<br>"projects_ms": 0.0,<br>"projects": 4,<br>"presets_ms": 0.0,<br>"presets": 4,<br>"line": "PROFILE seed_ms=67.0 list_ms=0.7 skills=200 payload_bytes=0 projects_ms=0.0 projects=4 presets_ms=0.0 presets=4"<br>} · log `automations/responsiveness/runs/r1/gates/n2_after.a1.md` |
| n2_after | a1 check.cmd | pass | `python3 automations/responsiveness/code/check_after.py` → exit 0: after payload_bytes=0 list_ms=0.7 · log `automations/responsiveness/runs/r1/gates/n2_after.a1.md` |
| n2_after | a1 node.report | pass | exit 0: {<br>"seed_ms": 67.0,<br>"list_ms": 0.7,<br>"skills": 200,<br>"payload_bytes": 0,<br>"projects_ms": 0.0,<br>"projects": 4,<br>"presets_ms": 0.0,<br>"presets": 4,<br>"line": "PROFILE seed_ms=67.0 list_ms=0.7 skills=200 payload_bytes=0 projects_ms=0.0 projects=4 presets_ms=0.0 presets=4"<br>} · log `automations/responsiveness/runs/r1/gates/n2_after.a1.md` |
| n3_doc | a1 check.cmd (superseded) | pass | `python3 automations/responsiveness/code/check_doc.py` → exit 0: responsiveness.md has before and after timings · log `automations/responsiveness/runs/r1/gates/n3_doc.a1.md` |
| n3_doc | a1 node.report (superseded) | pass | exit 0: wrote docs/responsiveness.md · log `automations/responsiveness/runs/r1/gates/n3_doc.a1.md` |
| n3_doc | a1 check.cmd | pass | `python3 automations/responsiveness/code/check_doc.py` → exit 0: responsiveness.md has before and after timings · log `automations/responsiveness/runs/r1/gates/n3_doc.a1.md` |
| n3_doc | a1 node.report | pass | exit 0: wrote docs/responsiveness.md · log `automations/responsiveness/runs/r1/gates/n3_doc.a1.md` |

## Acceptance checks (the deliverable)

| check | verdict | evidence |
|---|---|---|
| 0. docs/responsiveness.md records before and after library timings | pass | `python3 automations/responsiveness/code/check_doc.py` → exit 0: responsiveness.md has before and after timings |
| 1. every data screen renders a loading state | pass | `test -s src/features/loading-states.test.tsx` → exit 0: (no output) |

## Baseline before the run (2026-10-21T19:22:29+00:00)

| item | verdict | evidence |
|---|---|---|
| deliverable exists before the run | fail | docs/responsiveness.md absent |
| acceptance 0: docs/responsiveness.md records before and after library timings | fail | `python3 automations/responsiveness/code/check_doc.py` → exit 1: missing payload_bytes=2612160, list_ms=7.0, skills=200, projects=4, After |
| acceptance 1: every data screen renders a loading state | pass | `test -s src/features/loading-states.test.tsx` → exit 0: (no output) |

Observed before the run:
- before profile payload_bytes=2612160 list_ms=7.0; after not written yet

## Completion (2026-10-21T19:40:10+00:00) — COMPLETE

| item | verdict | evidence |
|---|---|---|
| graph.md is a valid spec | pass | validate ok |
| spec approved before execution | pass | hash 6a4fd01f49d2602c |
| n1_before: output exists and can be opened | pass | automations/responsiveness/out/before.json (349 bytes) |
| n1_before: gate ran against the saved version | pass | gate pass at 2026-10-21T19:22:34+00:00 |
| n2_after: output exists and can be opened | pass | automations/responsiveness/out/after.json (280 bytes) |
| n2_after: gate ran against the saved version | pass | gate pass at 2026-10-21T19:22:34+00:00 |
| n3_doc: output exists and can be opened | pass | docs/responsiveness.md (4822 bytes) |
| n3_doc: gate ran against the saved version | pass | gate pass at 2026-10-21T19:22:35+00:00 |
| graph.md matches the nodes that actually ran | pass | ran 3 of 3 declared nodes |
| acceptance 0: docs/responsiveness.md records before and after library timings | pass | `python3 automations/responsiveness/code/check_doc.py` → exit 0: responsiveness.md has before and after timings |
| acceptance 1: every data screen renders a loading state | pass | `test -s src/features/loading-states.test.tsx` → exit 0: (no output) |
| deliverable exists | pass | docs/responsiveness.md |
| run limit respected | pass | 0/6 model dispatches |
