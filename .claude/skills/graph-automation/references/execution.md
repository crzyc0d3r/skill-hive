# Running nodes

The main session is the lead: it decides nothing a script can decide, and it never builds a node's artifact itself when a subagent can. One "wave" is everything `ready` lists. Every `graphctl` below is `python3 ~/.claude/skills/graph-automation/scripts/graphctl.py`, run from the project root.

## Code nodes

```bash
graphctl run-code <slug> <node>
```

graphctl runs the command, records its exit code as the node's report, snapshots a risky target (or logs why not), and gates it. Nothing passes through a model. If the command fails, fix the script (that is a decision: `graphctl decide`) and run it again; the attempt counter still caps at 3.

## Model nodes: one subagent per node, all in one message

For every ready model node:

```bash
graphctl dispatch <slug> <node>      # starts attempt N; stdout is the brief verbatim (also saved to runs/<run>/briefs/<node>.a<N>.md)
```

Then call the Agent tool once per node, in the same message, so they run in parallel:

- `subagent_type`: `general-purpose` (it must write files; Explore cannot)
- `description`: `node <id>`
- `prompt`: the brief, verbatim. Do not add the task's full context; the brief is the bounded context by design.

The brief tells the subagent to end with one JSON object. When it returns:

1. Save the returned text to `automations/<slug>/runs/<run>/returns/<node>.a<N>.json` (a quoted heredoc is fine). A ```json fence around the object is tolerated by `--json-file`; so is JSON embedded in a short sentence.
2. `graphctl report <slug> <node> --json-file <that file> --tokens <the token count the Agent result shows, e.g. subagent_tokens>`
3. If the node has `check.verify`: `graphctl brief <slug> <node> --verify`, send it to a fresh general-purpose agent (description `verify <id>`), save its JSON to `runs/<run>/returns/<node>.verify.a<N>.json`, then `graphctl report <slug> <node> --role verify --json-file … --tokens …`. Tokens from both reports are summed. Verifiers for different nodes run in parallel with each other and with the next wave.
4. `graphctl gate <slug> <node>`

A subagent that returns prose with no JSON object gets one SendMessage asking for the JSON only; if that fails, `graphctl report <slug> <node> --verdict fail --evidence "no structured report"` so the gate records it honestly.

## Correction edge

`gate` exits 1 and the node is `failed`: the unit goes back to the node that made it, not to the batch.

```bash
graphctl dispatch <slug> <node>      # attempt N+1; the brief now carries a "Correction" section with the evidence
```

Prefer SendMessage to the same agent with the new brief (its context is intact and it knows what it wrote); start a fresh Agent if that agent is gone. Three failed attempts make the node `blocked`; graphctl refuses a fourth. Report blocked nodes in progress.md and keep running every node that does not depend on them.

Never change `check.cmd` or `check.verify` to get a pass. If a check is genuinely wrong (it tests the wrong file, has a typo), that is a graph change: `graphctl decide`, edit the spec, `graphctl validate`, and re-approve (interactively: ask; under `--auto`: approve and say so in the decision). Tightening a check is always allowed.

## Learning edge

After a node passes, if the attempt taught a rule the next run should start from (a format the check needs, a source that was a dead end, a file two nodes both wanted), record it:

```bash
graphctl learn <slug> "Summaries must cite file:line from the inventory, not from memory" --node n2_summaries
graphify save-result --question "<what the node needed>" --answer "<what worked>" --type query --nodes <graph node labels> --outcome useful --memory-dir MAP/memory
```

`MAP` is the directory holding `graph.json` (Step 1 of the runbook). `graphify reflect` reads `MAP/memory` only when told to with `--memory-dir`; a memory saved anywhere else is never reflected into LESSONS.md. `graphctl learn` refuses to record a rule for a node that has not passed: rules come from accepted results only.

## Limits and blockers

- `inputs.limit` counts model dispatches. When `ready` holds nodes for the limit, finish the code nodes, run `complete`, and report PARTIAL with the exact nodes left.
- A tool a node needs is unavailable: record it, `graphctl report <slug> <node> --verdict fail --evidence "blocker: …"`, gate, note it with `progress --issue`, and continue with the nodes that can still complete.
- The Agent tool is not in your tool list (some hosts give subagents none; claude.ai has none): run the brief yourself, one node at a time, in a fresh mental frame; still `report` and `gate` every node. Say in the final message that nodes ran inline.

## Optional: Workflow runner

Only when the user has opted into multi-agent orchestration (they said "use a workflow", or ultracode is on). `assets/run.js` fans out one wave:

```
Workflow({ scriptPath: "/Users/<you>/.claude/skills/graph-automation/assets/run.js",
           args: { nodes: [ { id: "<node>", brief: "<brief text>", output: "<writes>" }, … ] } })
```

It returns `{reports: [{node, output_path, verdict, evidence, notes}]}` validated against the schema the briefs ask for; save and record each with `report --json-file` and gate as usual. Readiness and gating stay in graphctl because scripts have no filesystem.
