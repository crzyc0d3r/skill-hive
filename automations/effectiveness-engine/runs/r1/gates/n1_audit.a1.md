# gate n1_audit attempt 1: pass (2026-10-18T14:45:08+00:00)

snapshot before this attempt: automations/effectiveness-engine/runs/r1/snapshots/n1_audit/a1/plan-conformance-audit.md

## check.cmd: pass

```
`python3 automations/effectiveness-engine/code/check_audit.py` → exit 0: real=38 stand-in=12 missing=13 total=63
```

## node.report: pass

```
docs/plan-conformance-audit.md has 63 rows: 38 real, 12 stand-in, 13 missing. Stand-in rows are listed separately and the summary says they are not done.
```
