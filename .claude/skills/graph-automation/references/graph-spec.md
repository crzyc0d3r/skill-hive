# graph.md and the spec

`graph.md` is prose plus exactly one ```json block: the spec. `graphctl` reads and rewrites that block and the tables between its `graphctl:auto` markers; everything else in the file (Decisions, Learned rules, your notes) is yours and is preserved.

## Spec

```json
{
  "slug": "release-notes",
  "version": 1,
  "approved": false,
  "approved_hash": null,
  "inputs": {
    "task": "Write release notes for everything merged since v2.3",
    "output": "docs/release-notes-v2.4.md",
    "checks": [
      {"text": "every entry cites a commit or PR", "cmd": "python3 automations/release-notes/code/check_cites.py"},
      "tone matches docs/release-notes-v2.3.md"
    ],
    "sources": ["src/", "CHANGELOG.md"],
    "scope": ["docs/release-notes-v2.4.md", "automations/release-notes/**"],
    "limit": 12
  },
  "map": {"graph_json": "graphify-out/graph.json", "lessons": "graphify-out/reflections/LESSONS.md"},
  "nodes": []
}
```

| field | meaning |
|---|---|
| `inputs.task` | `{{TASK}}` from the governing prompt |
| `inputs.output` | `{{OUTPUT}}`: one deliverable path, relative to the project root |
| `inputs.checks` | `{{CHECKS}}`: acceptance checks on the deliverable. An object with `cmd` is deterministic (exit 0 = pass, run by `baseline`, `accept --run`, `complete`). A bare string is judged at completion and needs evidence recorded with `accept --check N`. |
| `inputs.sources` | `{{SOURCES}}`: what graphify maps and nodes read |
| `inputs.scope` | `{{SCOPE}}`: globs; every node's `writes` must match one (`validate` refuses otherwise) |
| `inputs.limit` | `{{LIMIT}}`: max model-node attempts in one run. Code nodes do not count. `ready` holds model nodes once it is reached. |
| `map.graph_json` | the graphify graph for this corpus; `graphctl` appends `--graph` to bare `graphify …` context commands |
| `map.lessons` | graphify's `LESSONS.md` if it exists; briefs point subagents at it |

## Node

```json
{
  "id": "n2_summaries",
  "kind": "model",
  "scope": "Write one paragraph per community in the inventory, naming the files that changed and why.",
  "reads": ["automations/release-notes/out/n1_inventory.json"],
  "context": ["graphify query \"release changes\"", "graphify explain \"PaymentService\""],
  "writes": "automations/release-notes/out/n2_summaries.md",
  "check": {
    "cmd": "python3 automations/release-notes/code/check_summaries.py",
    "verify": "each paragraph's claims match the inventory it reads"
  },
  "deps": ["n1_inventory"],
  "risky": false,
  "timeout": 600
}
```

| field | rule |
|---|---|
| `id` | `^[a-z0-9][a-z0-9_-]*$`, unique. A numeric prefix in reading order helps humans. |
| `kind` | `model` (a subagent given only its brief) or `code` (a command graphctl runs itself). A step with exactly one correct answer is `code`. |
| `scope` | the node's whole job in one or two sentences. It becomes the subagent's instructions, so write it for a reader with no other context. |
| `reads` | every input, by path. If a path is another node's `writes`, that node must be in `deps` (`validate` enforces it: an undeclared consumer is a race). |
| `context` | model nodes only: graphify commands the subagent runs first (`query`, `explain`, `path`, `affected`) so it works from the graph instead of reading the corpus. |
| `run` | code nodes only: a shell command, cwd = project root, env `NODE_ID NODE_OUTPUT BUNDLE_DIR RUN_ID PROJECT_ROOT`. Put the script under `automations/<slug>/code/`. |
| `writes` | exactly one output path. Unique across nodes (two nodes never edit the same file) and inside `scope`. |
| `check.cmd` | deterministic check the gate runs against the saved file; exit 0 = pass; stdout/stderr become evidence. Default when absent: `test -s <writes>`. |
| `check.verify` | optional judgment requirement. A separate verifier subagent reports on it; the gate reads that report last. |
| `deps` | the edges. Nodes with no path between them are parallel. |
| `risky` | snapshot the current output into `runs/<run>/snapshots/` before each attempt; the gate log records the snapshot path or why it was skipped (target absent). Set it on every node that writes a real project file (usually the final code merge node). |

## What `validate` enforces

- ids unique and well-formed; `kind` is model or code; `scope` and `writes` non-empty; code nodes have `run`
- no cycles (it prints the parallel levels when valid)
- no two nodes write the same path; every `writes` matches `inputs.scope`
- a node that reads another node's output lists it in `deps`
- warnings (not errors): an edge whose output is not consumed, a model node with `run`, no node writing the deliverable, no `check`

## How a gate decides

Order matters: evidence first, confidence last.

1. `check.cmd` runs against the saved file (missing file = fail).
2. The node's own report (`graphctl report`, or the exit code of `run` for code nodes). Missing = unresolved.
3. `check.verify`, if present: the verifier's report. Missing = unresolved.

Any fail → `fail`; otherwise any unresolved → `unresolved`; otherwise `pass`. A fail on attempt 3 → `blocked`. Every gate writes `runs/<run>/gates/<node>.a<N>.md` and a row per requirement in `checks.md`; earlier attempts stay in the table marked superseded, because failed attempts are part of the record.

## Node statuses

`pending → running → reported → passed`, or `→ failed` (retry via correction edge, at most 3 attempts) `→ blocked`. `unresolved` means the gate ran but a report is missing: record it and gate again; do not dispatch again.

## Bundle layout

```
automations/<slug>/
  graph.md        spec + derived tables + Decisions + Learned rules   (the reusable template)
  checks.md       node | requirement | verdict | evidence, baseline, acceptance, completion
  progress.md     Graph / Outputs / Decisions / Open issues / Next action (handoff)
  state.json      machine state (do not hand-edit)
  out/            node artifacts
  code/           scripts for code nodes and checks
  runs/<run>/     prompt.md (filled governing prompt), briefs/<node>.a<N>.md, returns/<node>.a<N>.json and <node>.verify.a<N>.json, gates/<node>.a<N>.md, snapshots/
```

## graphctl cheat sheet

Run from the project root. `$G` below stands for `python3 ~/.claude/skills/graph-automation/scripts/graphctl.py`; type it in full every time (zsh does not word-split a variable holding a command, and shell state does not persist between tool calls).

| step | command |
|---|---|
| create | `$G init <slug> --task '…' --output P --checks 'text::cmd' 'text' --sources S… --scope G… --limit N --graph-json P --lessons P` (single-quote values containing `$(…)` or quotes) |
| write nodes | `$G spec <slug> --from spec.json` (`{"nodes": [...]}` replaces the nodes; an `"inputs"` object there is merged field by field; approval fields are ignored), then `$G validate <slug>` and `$G render <slug>` |
| approve | `$G approve <slug>` (after the user approved, or under `--auto`) |
| prompt | `$G prompt <slug>` writes and prints the filled governing prompt |
| baseline | `$G baseline <slug> [--note "…"]` before any node runs |
| loop | `$G ready <slug>` → `$G run-code <slug> <code-node>` / `$G dispatch <slug> <model-node>` → `$G report <slug> <node> --json-file F --tokens N` → (`$G brief <slug> <node> --verify` → `$G report … --role verify --json-file F`) → `$G gate <slug> <node>` |
| record | `$G decide <slug> "…"`, `$G learn <slug> "…" --node <id>`, `$G progress <slug> [--decision "…"] [--issue "…"]` |
| finish | `$G accept <slug> --run`, `$G accept <slug> --check N --verdict pass --evidence "…"` (N is 0-based, as `accept --run` prints), `$G complete <slug>` |
| later | `$G status <slug>`, `$G rerun <slug> [--sources …] [--limit N]`, `$G snapshot <slug> <node>` |

Exit codes: `gate` 0 pass / 1 fail / 2 unresolved; `complete` 0 only when COMPLETE; `validate` 1 on errors.
