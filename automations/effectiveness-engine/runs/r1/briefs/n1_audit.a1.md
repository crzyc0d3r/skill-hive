# Node brief: n1_audit (attempt 1 of 3) — automation `effectiveness-engine`

You are running ONE node of a graph of bounded nodes. Build this node's artifact and nothing else.
Task context (do not try to complete all of it here): Score PLAN.md effectiveness requirements as real, stand-in, or missing, and do not count stand-ins as done.
Project root (all paths below are relative to it): `/workspace`

## Scope (your whole job)
Write docs/plan-conformance-audit.md. Score PLAN.md Part 2 section 3 (3.1-3.9), Part 3 A-D, Part 1 sections 4 and 5, and Part 4 section 5.3. Each row is real, stand-in, or missing, with a file reference. Do not count a stand-in as done.

## Inputs — read these; do not go looking for more unless the scope requires it
- `PLAN.md` — exists
- `crates/hive-core/src/testing/effectiveness.rs` — exists

## Output — exactly one file
`docs/plan-conformance-audit.md` — write the complete artifact here (create parent directories; overwriting is fine).

## Check — the gate will run this against the saved file; exit code 0 means pass
```bash
python3 automations/effectiveness-engine/code/check_audit.py
```

## Rules
- Allowed changes: `docs/plan-conformance-audit.md`, `automations/effectiveness-engine/**`. Nothing else; another node may own any other file.
- This is a judgment node: write the artifact directly. Only if a sub-step inside it has exactly one correct answer and would be error-prone by hand (counting, sorting or merging many entries) write a small helper under `automations/effectiveness-engine/code/`; never generate prose from a script.
- Do not publish, send, spend, or ask the user anything; if something blocks you, return verdict fail with the blocker as evidence.
- Run the check yourself before returning. If it fails, fix and re-run; return fail honestly if it still fails.

## Return format — your final message must be exactly this JSON and nothing else
```json
{
  "node": "n1_audit",
  "output_path": "docs/plan-conformance-audit.md",
  "verdict": "pass|fail",
  "evidence": "<the check's actual output, or why it failed>",
  "notes": "<optional, one line>"
}
```
