---
name: graph-automation
description: Run a task as a graph of bounded nodes and leave behind a rerunnable automation. graphify maps the source material, the task is split into nodes with one input, one output and one check each, deterministic steps run as code, model steps run as bounded subagents behind evidence gates, and graph.md, checks.md and progress.md make the run resumable, repeatable and schedulable. Use this whenever the user wants work done "as a graph of nodes", mentions bounded nodes, node graphs, gates, graph.md, checks.md, progress.md, an autoresearch-style run, a repeatable or rerunnable automation built from subagents, or a multi-step job with verification and a handoff another session can resume; also for /graph-automation resume, rerun or status. Not for a single quick edit or a one-shot question.
argument-hint: '"<task>" [--output P] [--sources P ...] [--checks "text::cmd" ...] [--scope G ...] [--limit N] [--slug s] [--auto] [--no-map] | resume <slug> | rerun <slug> [--sources P ...] | status <slug>'
---

# graph-automation

Invocation: `$ARGUMENTS`

You are the lead that designs, runs and maintains one graph of bounded nodes. The governing prompt is
`references/prompt.md`; read it first. This file is the runbook that executes it with two tools:

- **graphify** maps the source material into a knowledge graph (communities = what gets read together, god nodes = shared inputs, edges = dependencies). That is the information layer 1 needs: "split by what each node must read, not by folder".
- **graphctl** is the deterministic engine. Readiness, gates, attempt caps, evidence tables, completion and state all have one correct answer, so they are code, never a model call. It is one script; call it by its full path, from the project root, in every command:

```bash
python3 ~/.claude/skills/graph-automation/scripts/graphctl.py <command> <slug> …
```

Type that path literally. Do not stash it in a shell variable: zsh does not word-split `$G`, and shell state does not survive between tool calls anyway. Below, "graphctl" in prose means that command. If this skill loaded from a different directory (another agent's skills folder, or the Skills Manager library at `~/.skills-manager/skills/graph-automation`), use that directory in place of `~/.claude/skills/graph-automation` in every command; the scripts resolve their own location, so nothing else changes.

What a run leaves behind, in `automations/<slug>/`: `graph.md` (the reusable template: spec, edges, code nodes, decisions, learned rules), `checks.md` (node | requirement | verdict | evidence), `progress.md` (handoff), `out/`, `code/`, `runs/<run>/` (filled prompt, briefs, returns, gate logs, snapshots), plus the deliverable at its own path. That bundle is the automation: any session can `resume` or `rerun` it.

| prompt layer | what you do | with |
|---|---|---|
| inputs | fill task, output, checks, sources, scope, limit | Step 0 |
| 1 split | map with graphify, write the spec, pause for approval | Steps 1–2, `graph_context.py`, graphctl `validate` / `approve` |
| 2 nodes | one subagent per model node, given only its brief | Step 3, graphctl `dispatch`, Agent tool |
| 3 code nodes | deterministic steps as `kind: code` | graphctl `run-code` |
| 4 permissions | scope globs enforced on every `writes`; snapshots before risky merges | graphctl `validate`, `risky: true` |
| 5 gates | cmd first, node report second, verifier last; rows in checks.md | graphctl `gate` |
| 6 return paths | correction edge (3 attempts), learning edge | graphctl `dispatch` again, graphctl `learn` |
| 7 completion | checklist with observable evidence | graphctl `baseline` / `accept` / `complete` |
| handoff | progress.md after every wave | graphctl `progress` |

## Modes

- **New task** (default): Steps 0 to 4.
- **`resume <slug>`**: read `graph.md` and `progress.md` first, then graphctl `status`, and continue at Step 3 from the recorded next action. Passed nodes are never re-run.
- **`rerun <slug> [--sources …] [--limit N]`**: graphctl `rerun` archives the finished run and resets the nodes on the same template. Refresh the map (Step 1) and go to Step 3. An approved template whose nodes did not change needs no new approval.
- **`status <slug>`**: graphctl `status` plus the Open issues and Next action from progress.md. Stop there.
- **`--auto`**: no pauses and no approval table. Approve a new graph yourself, record `auto-approved by --auto` with graphctl `decide`, and say so in the final message. Reruns of an approved template never pause anyway. Use this for scheduled or headless runs. Without `--auto`, the approval pause in Step 2 is mandatory.

## Step 0 — Fill the inputs

Every field of the governing prompt's `<inputs>` must be filled before anything runs.

| field | source | default when the user gave nothing |
|---|---|---|
| task | the invocation text | ask |
| output | `--output` | `automations/<slug>/out/result.md`, and say so |
| checks | `--checks`; `text::cmd` makes one deterministic | "deliverable exists and is non-empty" plus one check per requirement the task states |
| sources | `--sources` | the project root |
| scope | `--scope` | the output path plus `automations/<slug>/**` |
| limit | `--limit` | 3 × the number of model nodes, once the split exists |
| slug | `--slug` | kebab-case from the task, short |

The limit caps model-node dispatches for the whole run (code nodes are free); the per-node cap of 3 attempts is fixed and separate. The default lets every model node use its full correction budget.

Ask one focused question only if a missing detail changes the outcome (which file is the deliverable, which of two codebases). Otherwise choose the default and record it with graphctl `decide` once the bundle exists. Under `--auto`, never ask.

Also read the project's own instructions (CLAUDE.md) and look at the current state of the deliverable path before editing anything: the lead inspects current work first.

## Step 1 — Map the sources with graphify

The map replaces reading the whole corpus. Communities become candidate nodes, god nodes become shared inputs that one node owns and others read, cross-community edges become candidate task edges, and `graphify explain` / `affected` give each subagent bounded context later. graphify writes its map to `<scan root>/graphify-out/`; call that directory `MAP` and pass it explicitly to every graphify memory command, because `reflect` and `save-result` otherwise use `./graphify-out/` relative to the current directory and the learning edge silently misses.

```bash
ls <sources>/graphify-out/graph.json graphify-out/graph.json 2>/dev/null || true     # reuse an existing map (prints whichever exists)
graphify extract <sources> --code-only                                         # code-only corpus: deterministic, no API key, seconds
# docs, papers or images in the corpus: invoke the graphify skill (/graphify <sources>); it extracts them with subagents and costs tokens, so say so.
# With GEMINI_API_KEY or GOOGLE_API_KEY set, `graphify extract <sources>` does it headlessly instead.
graphify reflect --if-stale --graph MAP/graph.json --memory-dir MAP/memory --out MAP/reflections/LESSONS.md && cat MAP/reflections/LESSONS.md
python3 ~/.claude/skills/graph-automation/scripts/graph_context.py summary --graph MAP/graph.json
python3 ~/.claude/skills/graph-automation/scripts/graph_context.py focus --graph MAP/graph.json --query "<the task>"
```

`summary` lists communities with their files, god nodes and cross-community edges. `focus` finds the communities the task touches and prints a draft split with candidates already merged where their files overlap, plus the shared files one node must own. graphify matches identifiers, not task words: when `focus` finds no vocabulary, take symbol names from `summary` and use `graphify explain "<symbol>" --graph MAP/graph.json` or `graphify affected "<symbol>" --graph MAP/graph.json`, or split from the per-community file lists. `GRAPH_REPORT.md` exists only after the full graphify skill or `graphify cluster-only <sources> --no-viz --no-label`; neither is required.

If `graphify extract` refreshes an existing map it refuses to shrink it; pass `--force` only after confirming files were really deleted. `--no-map` skips this step; then split from the file list and say in the final message that the split was not graph-informed. If graphify is missing and installing it is not possible, that is a named blocker, not a reason to stop: preserve the nodes that can still be completed.

## Step 2 — Split into graph.md and pause for approval

Write the nodes before any work starts. Read `references/graph-spec.md` for the exact fields. A good node:

- has one `scope` sentence a stranger could execute, one `writes` path, one `check`;
- reads what one community reads; two candidates whose reads overlap heavily become one node (fewer nodes when results are comparable);
- is `kind: code` when the step has exactly one correct answer (inventory, merge, sort, dedupe, count, compare, diff, format); a model call is for judgment only;
- gets an edge only when it consumes another node's output;
- gets `check.verify` only when correctness is a judgment that no command can test. A verifier is a second subagent and costs as much as the node did; for small nodes a `check.cmd` is enough. Likewise add `context` commands only when the node must understand code it does not receive as input;
- is `risky: true` when it writes a real project file (usually the final code merge node).

In this order:

```bash
python3 ~/.claude/skills/graph-automation/scripts/graphctl.py init <slug> --task '…' --output P --checks 'text::cmd' 'text' --sources … --scope … --limit N --graph-json MAP/graph.json --lessons MAP/reflections/LESSONS.md
# now the bundle exists: write the code-node scripts and check scripts under automations/<slug>/code/
python3 ~/.claude/skills/graph-automation/scripts/graphctl.py spec <slug> --from /path/to/spec.json       # {"nodes": [...]} is enough; "inputs" there is merged field by field
python3 ~/.claude/skills/graph-automation/scripts/graphctl.py validate <slug>                             # fix every error; its warnings are usually right too
python3 ~/.claude/skills/graph-automation/scripts/graphctl.py render <slug>
python3 ~/.claude/skills/graph-automation/scripts/graphctl.py prompt <slug>                               # writes runs/<run>/prompt.md (the governing prompt with the inputs filled in) and prints its path
```

Single-quote `--checks` and `--task` values so `$(…)` and `'^## '` are stored, not expanded; or put `inputs.checks` in the spec file instead. `init` is safe on an existing directory and refuses to overwrite an existing spec without `--force`.

Then **pause** (not under `--auto`): show the user the Nodes table and Edges from `graph.md` (not the raw JSON), name the code nodes and the checks, state the limit, and ask for approval in one message. Do not dispatch anything until they answer. Edits they ask for go into the spec, then `validate` again. When they approve, or under `--auto`:

```bash
python3 ~/.claude/skills/graph-automation/scripts/graphctl.py approve <slug>
python3 ~/.claude/skills/graph-automation/scripts/graphctl.py baseline <slug> --note "<anything measured before the run, e.g. current test count>"
```

The baseline records which acceptance checks already pass before any node runs, so the run measures a change rather than asserting one.

Check the permission mode before dispatching (`grep -h -A3 '"permissions"' .claude/settings.json ~/.claude/settings.json 2>/dev/null || true`, or the mode shown in your session context): subagents inherit it, and a mode that blocks writes blocks every node. Scope is enforced by `validate` (every `writes` must match `inputs.scope`) and by each brief; if the user wants the sandbox to enforce it too, that is a settings change for the `update-config` skill, not something to do silently.

## Step 3 — Execute the loop

Read `references/execution.md` once for the mechanics (Agent tool shape, return files, verifier agents, SendMessage for corrections, the optional Workflow runner, inline fallback). The loop, with `graphctl` standing for the full command:

```bash
graphctl ready <slug>                                      # 1. next ready nodes (deps passed, attempts < 3, limit not reached)
graphctl run-code <slug> <code-node>                       # 2a. code nodes: runs, reports, gates; no subagent
graphctl dispatch <slug> <model-node>                      # 2b. model nodes: prints the brief; one Agent call per node, all in ONE message
graphctl report <slug> <node> --json-file automations/<slug>/runs/<run>/returns/<node>.a<N>.json --tokens <subagent_tokens from the Agent result>
graphctl gate <slug> <node>                                # 4. deterministic check → node report → verifier; exit 0 pass / 1 fail / 2 unresolved
graphctl progress <slug>                                   # 5. after every wave
```

- Run everything `ready` lists together; independent nodes are parallel by construction. `dispatch` counts against the limit, so call it only when you are about to call the Agent.
- A failed gate returns that unit only: `dispatch` again prints a brief with the Correction section (reason, evidence, "fix this node only"). Three failures block the node; keep running everything that does not depend on it.
- Merge accepted outputs in code (a `kind: code` node), never by hand.
- Record decisions that change the graph with graphctl `decide`; after a passed node proves a rule worth starting from next time, graphctl `learn … --node <id>`.
- Never weaken a gate to obtain a pass. Fixing a wrong check is a decision plus a spec change plus re-approval.
- Continue until `ready` is empty and nothing is running, a blocker stops the graph, or the limit holds the remaining model nodes.

## Step 4 — Complete and deliver

```bash
graphctl accept <slug> --run                                            # deterministic acceptance checks
graphctl accept <slug> --check N --verdict pass|fail --evidence "…"     # judged ones (N is 0-based, as `accept --run` prints them): point at the evidence you actually looked at
graphctl complete <slug>                                                # outputs open, gates ran against the saved versions, nothing ran outside the graph, limit respected
graphctl progress <slug> [--issue "…"]                                  # final handoff; --issue for anything unresolved, in words
```

`complete` exits 0 only for COMPLETE; PARTIAL prints the exact nodes left. Do not edit checks.md or state.json by hand to change a verdict.

Close the learning edge with graphify when the map helped or misled: `graphify save-result --question "…" --answer "…" --type query --nodes <graph node labels> --outcome useful|dead_end --memory-dir MAP/memory` (the same MAP as Step 1, so the next `reflect` sees it), and refresh the map after the run changed code under the sources (`graphify update <sources>`).

The final message to the user: the deliverable path; the bundle path; what was checked and passed; what remains unresolved or blocked, by node; whether the graph was auto-approved; measured usage only if `complete` printed recorded tokens (never inferred savings); and the one-line rerun command:

```bash
/graph-automation rerun <slug> --auto
```

Offer, in one line, to put that command on a schedule with the `schedule` skill when the task is recurring.

## Boundaries

- Ask before publishing, sending or spending; nodes never do those, and every brief tells them so.
- Two nodes never edit the same file (`validate` refuses it). Nodes write only inside `inputs.scope`.
- Risky merges are snapshotted to `runs/<run>/snapshots/` before each attempt; the gate log records the snapshot or why it was skipped.
- Evidence stays visible: missing reports show as unresolved in checks.md, failed attempts stay in the table marked superseded.
- Do not delete `automations/<slug>/` or a `graphify-out/` directory on the user's behalf.

## Reference files

- `references/prompt.md` — the governing prompt (verbatim; graphctl `prompt` fills its placeholders)
- `references/graph-spec.md` — spec fields, gate semantics, statuses, bundle layout, graphctl cheat sheet
- `references/execution.md` — running nodes: Agent calls, return files, verifiers, correction edge, limits, fallbacks, Workflow runner
- `scripts/graphctl.py` — the engine (`--help` lists every command)
- `scripts/graph_context.py` — graphify map → split inputs (`summary`, `focus`, `files`)
- `assets/run.js` — optional Workflow script that fans out one wave
