#!/usr/bin/env python3
"""graphctl - deterministic engine for graph-automation bundles.

Every step that has exactly one correct answer (readiness, gating, merging
evidence, completion, state) lives here so that no model call ever decides it.

Run it from the PROJECT ROOT. Every path inside graph.md is relative to that
root. A bundle lives at <root>/<slug>/ (default root: automations/).

  python3 ~/.claude/skills/graph-automation/scripts/graphctl.py <command> <slug> [options]

Commands
  init       create a bundle: graph.md (empty spec), state.json, progress.md, checks.md
  spec       print the JSON spec, or replace it from a file (--from spec.json)
  validate   check the spec: ids, DAG, one output per node, disjoint writes, edges declared
  render     refresh the derived tables inside graph.md from the spec
  approve    validate + mark the spec approved (records a hash of the nodes)
  prompt     write the governing prompt with {{placeholders}} filled to runs/<run>/prompt.md
  ready      list nodes whose upstream gates passed and that may run now
  dispatch   start an attempt for a model node and print its brief
  brief      print a node's brief without starting an attempt (--verify for the verifier brief)
  run-code   run a code node's command, then gate it
  report     record a node's (or verifier's) returned verdict and evidence
  gate       run the node's check against the saved artifact and record the verdict
  baseline   record the acceptance checks' state before any node runs (measure before changing)
  accept     record or run the acceptance checks for the deliverable
  complete   run the completion checklist; exit 0 only when everything passed
  progress   regenerate progress.md (auto sections) and append manual notes
  decide     record a decision that changed the graph (graph.md + progress.md)
  learn      record a rule a passed node proved (graph.md "Learned rules")
  status     one-screen status (--json for machines)
  snapshot   copy a node's current output into runs/<run>/snapshots before a risky change
  rerun      archive the current run and start a fresh one on the same template
"""
from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

SKILL_DIR = Path(__file__).resolve().parent.parent
PROMPT_PATH = SKILL_DIR / "references" / "prompt.md"
MAX_ATTEMPTS = 3
DEFAULT_ROOT = "automations"
DEFAULT_TIMEOUT = 600
FENCE_RE = re.compile(r"```json[^\n]*\n(.*?)\n```", re.S)
ID_RE = re.compile(r"^[a-z0-9][a-z0-9_-]*$")
VERDICTS = ("pass", "fail", "unresolved")


# ----------------------------------------------------------------------------
# helpers
# ----------------------------------------------------------------------------

def now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def die(msg: str, code: int = 1) -> None:
    print(f"graphctl: {msg}", file=sys.stderr)
    sys.exit(code)


def sha256_file(p: Path) -> str | None:
    if not p.is_file():
        return None
    h = hashlib.sha256()
    with p.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def nodes_hash(nodes: list[dict]) -> str:
    canon = json.dumps(sorted(nodes, key=lambda n: n.get("id", "")), sort_keys=True)
    return hashlib.sha256(canon.encode("utf-8")).hexdigest()[:16]


def one_line(text: str, limit: int = 300) -> str:
    """Markdown-table-safe single line (pipes escaped, newlines as <br>)."""
    text = (text or "").strip().replace("|", "\\|").replace("\r", "")
    text = "<br>".join(line.strip() for line in text.splitlines() if line.strip())
    return text if len(text) <= limit else text[: limit - 1] + "…"


def console(text: str, limit: int = 200) -> str:
    """Console-safe single line (no markdown escaping)."""
    text = " / ".join(line.strip() for line in (text or "").replace("\r", "").splitlines() if line.strip())
    return text if len(text) <= limit else text[: limit - 1] + "…"


def in_scope(path: str, scope: list[str]) -> bool:
    if not scope:
        return True
    norm = path.replace("\\", "/")
    for g in scope:
        g = g.replace("\\", "/")
        if g.endswith("/") and norm.startswith(g):
            return True
        if fnmatch.fnmatch(norm, g) or norm == g.rstrip("/"):
            return True
        if g.endswith("/**") and norm.startswith(g[:-3] + "/"):
            return True
    return False


def run_shell(cmd: str, cwd: Path, env: dict, timeout: int) -> tuple[int, str]:
    try:
        proc = subprocess.run(
            cmd, shell=True, cwd=str(cwd), env=env, capture_output=True, text=True, timeout=timeout
        )
    except subprocess.TimeoutExpired:
        return 124, f"timed out after {timeout}s"
    out = (proc.stdout or "") + (("\n[stderr]\n" + proc.stderr) if proc.stderr else "")
    return proc.returncode, out.strip()


def replace_between(text: str, name: str, body: str) -> str:
    begin = f"<!-- graphctl:auto:begin {name} -->"
    end = f"<!-- graphctl:auto:end {name} -->"
    block = f"{begin}\n{body.rstrip()}\n{end}"
    if begin in text and end in text:
        pre = text[: text.index(begin)]
        post = text[text.index(end) + len(end):]
        return pre + block + post
    return text.rstrip() + "\n\n" + block + "\n"


def append_under(text: str, heading: str, line: str) -> str:
    """Append a bullet at the end of a '## heading' section (create it if missing)."""
    pattern = re.compile(rf"^## {re.escape(heading)}\s*$", re.M)
    m = pattern.search(text)
    if not m:
        return text.rstrip() + f"\n\n## {heading}\n{line}\n"
    nxt = re.compile(r"^## ", re.M).search(text, m.end())
    cut = nxt.start() if nxt else len(text)
    section = text[m.end():cut].rstrip("\n")
    return text[: m.end()] + section + "\n" + line + "\n\n" + text[cut:].lstrip("\n")


# ----------------------------------------------------------------------------
# bundle
# ----------------------------------------------------------------------------

class Bundle:
    def __init__(self, slug: str, root: str):
        if not ID_RE.match(slug or ""):
            die(f"slug {slug!r} must match {ID_RE.pattern}")
        self.slug = slug
        self.project = Path.cwd()
        self.dir = Path(root) / slug
        self.graph_md = self.dir / "graph.md"
        self.state_path = self.dir / "state.json"
        self.checks_md = self.dir / "checks.md"
        self.progress_md = self.dir / "progress.md"

    # --- spec -----------------------------------------------------------
    def require(self) -> None:
        if not self.graph_md.is_file():
            die(f"no bundle at {self.dir}/ (run: graphctl init {self.slug} ...)")

    def load_spec(self) -> dict:
        self.require()
        text = self.graph_md.read_text(encoding="utf-8")
        m = FENCE_RE.search(text)
        if not m:
            die(f"{self.graph_md}: no ```json spec block found")
        try:
            return json.loads(m.group(1))
        except json.JSONDecodeError as e:
            die(f"{self.graph_md}: spec block is not valid JSON ({e})")

    def save_spec(self, spec: dict) -> None:
        text = self.graph_md.read_text(encoding="utf-8")
        m = FENCE_RE.search(text)
        if not m:
            die(f"{self.graph_md}: no ```json spec block found")
        body = json.dumps(spec, indent=2, ensure_ascii=False)
        text = text[: m.start(1)] + body + text[m.end(1):]
        self.graph_md.write_text(text, encoding="utf-8")

    # --- state ----------------------------------------------------------
    def load_state(self) -> dict:
        if not self.state_path.is_file():
            die(f"{self.state_path} missing; run graphctl init")
        return json.loads(self.state_path.read_text(encoding="utf-8"))

    def save_state(self, state: dict) -> None:
        self.state_path.write_text(json.dumps(state, indent=2, ensure_ascii=False), encoding="utf-8")

    def runs_dir(self, state: dict) -> Path:
        d = self.dir / "runs" / state["run_id"]
        d.mkdir(parents=True, exist_ok=True)
        return d

    def node_state(self, state: dict, nid: str) -> dict:
        return state["nodes"].setdefault(
            nid, {"status": "pending", "attempts": 0, "last_verdict": None, "output_hash": None, "gate": None, "tokens": 0}
        )

    def event(self, state: dict, kind: str, msg: str) -> None:
        state.setdefault("events", []).append({"at": now(), "type": kind, "msg": msg})


# ----------------------------------------------------------------------------
# spec validation
# ----------------------------------------------------------------------------

def node_map(spec: dict) -> dict[str, dict]:
    return {n.get("id"): n for n in spec.get("nodes", [])}


def topo_levels(nodes: list[dict]) -> list[list[str]]:
    ids = [n["id"] for n in nodes]
    deps = {n["id"]: list(n.get("deps") or []) for n in nodes}
    remaining = set(ids)
    done: set[str] = set()
    levels: list[list[str]] = []
    while remaining:
        level = sorted(i for i in remaining if all(d in done for d in deps[i]))
        if not level:
            raise ValueError("cycle among: " + ", ".join(sorted(remaining)))
        levels.append(level)
        done.update(level)
        remaining.difference_update(level)
    return levels


def validate_spec(spec: dict) -> tuple[list[str], list[str], list[list[str]]]:
    errors: list[str] = []
    warnings: list[str] = []
    inputs = spec.get("inputs") or {}
    for key in ("task", "output"):
        if not str(inputs.get(key) or "").strip():
            errors.append(f"inputs.{key} is empty")
    limit = inputs.get("limit")
    if not isinstance(limit, int) or limit <= 0:
        errors.append("inputs.limit must be a positive integer (max model-node attempts per run)")
    scope = list(inputs.get("scope") or [])
    nodes = spec.get("nodes") or []
    if not nodes:
        errors.append("spec has no nodes")
    ids: list[str] = []
    writes_seen: dict[str, str] = {}
    for n in nodes:
        nid = n.get("id")
        if not isinstance(nid, str) or not ID_RE.match(nid):
            errors.append(f"node id {nid!r} must match {ID_RE.pattern}")
            continue
        if nid in ids:
            errors.append(f"duplicate node id {nid}")
        ids.append(nid)
        kind = n.get("kind")
        if kind not in ("model", "code"):
            errors.append(f"{nid}: kind must be 'model' or 'code'")
        if not str(n.get("scope") or "").strip():
            errors.append(f"{nid}: scope is empty")
        writes = n.get("writes")
        if not isinstance(writes, str) or not writes.strip():
            errors.append(f"{nid}: writes must be exactly one output path")
        else:
            if writes in writes_seen:
                errors.append(f"{nid} and {writes_seen[writes]} both write {writes} (two nodes may never edit the same file)")
            writes_seen[writes] = nid
            if not in_scope(writes, scope):
                errors.append(f"{nid}: writes {writes} is outside inputs.scope {scope}")
        if kind == "code" and not str(n.get("run") or "").strip():
            errors.append(f"{nid}: code nodes need a run command")
        if kind == "model" and n.get("run"):
            warnings.append(f"{nid}: model node has a run command; it is ignored (deterministic work belongs in a code node)")
        check = n.get("check")
        if check is None:
            warnings.append(f"{nid}: no check given; the gate will only require a non-empty output file")
        elif not isinstance(check, dict) or not (check.get("cmd") or check.get("verify")):
            errors.append(f"{nid}: check must be an object with cmd and/or verify")
        for d in n.get("deps") or []:
            if d == nid:
                errors.append(f"{nid}: depends on itself")
        if not isinstance(n.get("reads", []), list):
            errors.append(f"{nid}: reads must be a list")
    idset = set(ids)
    for n in nodes:
        nid = n.get("id")
        if nid not in idset:
            continue
        for d in n.get("deps") or []:
            if d not in idset:
                errors.append(f"{nid}: unknown dep {d}")
    # edges must exist where outputs are consumed
    by_writes = {n["writes"]: n["id"] for n in nodes if n.get("id") in idset and isinstance(n.get("writes"), str)}
    for n in nodes:
        nid = n.get("id")
        if nid not in idset:
            continue
        deps = set(n.get("deps") or [])
        consumed = {by_writes[r] for r in (n.get("reads") or []) if r in by_writes and by_writes[r] != nid}
        for up in sorted(consumed - deps):
            errors.append(f"{nid} reads the output of {up} but has no edge to it (add {up} to deps)")
        for d in sorted(deps - consumed):
            warnings.append(f"{nid} -> {d}: edge without consumed output (fine for ordering a risky merge; otherwise drop it)")
    levels: list[list[str]] = []
    if not errors:
        try:
            levels = topo_levels(nodes)
        except ValueError as e:
            errors.append(str(e))
    out = inputs.get("output")
    if out and out not in writes_seen:
        warnings.append(f"no node writes the deliverable {out}; make sure a node (usually a code merge node) produces it")
    return errors, warnings, levels


# ----------------------------------------------------------------------------
# rendering: graph.md tables, briefs, checks.md, progress.md
# ----------------------------------------------------------------------------

GRAPH_MD_TEMPLATE = """# graph: {slug}

Governing prompt: `{prompt_path}` (a filled copy is written to `runs/<run>/prompt.md` by `graphctl prompt`).
Engine: `{engine}` — run every command from the project root `{project}`.

## How to read this file

- The JSON block under **Spec** is the single source of truth. `deps` are the edges; every edge carries a gate.
- A node is one bounded job: one `scope`, one output (`writes`), one `check`. Nodes with no path between them run in parallel.
- `kind: code` nodes are deterministic commands; `kind: model` nodes are subagents given only their brief.
- The tables below the spec are generated by `graphctl render`; edit the JSON, not the tables.

## Spec

```json
{spec}
```

<!-- graphctl:auto:begin tables -->
(run `graphctl render {slug}` after editing the spec)
<!-- graphctl:auto:end tables -->

## Decisions that changed the graph

## Learned rules
"""


def render_tables(spec: dict, levels: list[list[str]]) -> str:
    nodes = spec.get("nodes") or []
    lines = ["## Nodes", "", "| id | kind | scope | reads | writes | check | deps |", "|---|---|---|---|---|---|---|"]
    for n in nodes:
        check = n.get("check") or {}
        chk = []
        if check.get("cmd"):
            chk.append("cmd: `" + one_line(check["cmd"], 80) + "`")
        if check.get("verify"):
            chk.append("verify: " + one_line(check["verify"], 80))
        if not chk:
            chk.append("default: output exists and is non-empty")
        lines.append(
            f"| {n.get('id')} | {n.get('kind')} | {one_line(str(n.get('scope','')), 120)} | "
            f"{one_line(', '.join(n.get('reads') or []), 120) or '—'} | `{n.get('writes')}` | {'; '.join(chk)} | "
            f"{', '.join(n.get('deps') or []) or '—'} |"
        )
    lines += ["", "## Edges (each carries a gate: upstream check must pass before downstream runs)", ""]
    edges = [(d, n["id"]) for n in nodes for d in (n.get("deps") or [])]
    lines += [f"- {a} → {b}" for a, b in edges] or ["- (none: every node is independent)"]
    lines += ["", "## Parallel levels (nodes in one level share no edge and may run together)", ""]
    lines += [f"- L{i}: {', '.join(level)}" for i, level in enumerate(levels)] or ["- (spec not valid yet)"]
    lines += ["", "## Code nodes (deterministic; never routed through a subagent)", ""]
    code = [n for n in nodes if n.get("kind") == "code"]
    lines += [f"- {n['id']}: `{n.get('run')}`" for n in code] or ["- (none)"]
    return "\n".join(lines)


def render_graph_md(b: Bundle, spec: dict) -> None:
    errors, warnings, levels = validate_spec(spec)
    text = b.graph_md.read_text(encoding="utf-8")
    body = render_tables(spec, levels)
    if errors:
        body += "\n\nValidation errors (fix the spec):\n" + "\n".join(f"- {e}" for e in errors)
    text = replace_between(text, "tables", body)
    b.graph_md.write_text(text, encoding="utf-8")


def upstream_outputs(spec: dict) -> dict[str, str]:
    return {n["writes"]: n["id"] for n in spec.get("nodes") or [] if isinstance(n.get("writes"), str)}


def graphify_cmd(cmd: str, graph_json: str | None) -> str:
    """Append --graph <path> to bare graphify commands so subagents hit the right graph."""
    if graph_json and cmd.strip().startswith("graphify ") and "--graph" not in cmd:
        return f"{cmd.strip()} --graph {shlex.quote(graph_json)}"
    return cmd


def render_brief(b: Bundle, spec: dict, state: dict, node: dict, attempt: int, reason: str | None) -> str:
    inputs = spec.get("inputs") or {}
    producers = upstream_outputs(spec)
    graph_json = (spec.get("map") or {}).get("graph_json")
    check = node.get("check") or {}
    cmd = check.get("cmd") or f"test -s {shlex.quote(node['writes'])}"
    reads = []
    for r in node.get("reads") or []:
        p = b.project / r
        tag = f" (output of node {producers[r]})" if r in producers else ""
        exists = "exists" if p.exists() else "MISSING"
        reads.append(f"- `{r}`{tag} — {exists}")
    ctx = [f"- `{graphify_cmd(c, graph_json)}`" for c in node.get("context") or []]
    lessons = (spec.get("map") or {}).get("lessons")
    if lessons:
        lp = b.project / lessons
        try:
            ltxt = lp.read_text(encoding="utf-8") if lp.is_file() else ""
        except OSError:
            ltxt = ""
        if not ltxt or "No marked outcomes" in ltxt or "from 0 session" in ltxt:
            lessons = None  # nothing learned yet; do not send the subagent to read an empty file
    lines = [
        f"# Node brief: {node['id']} (attempt {attempt} of {MAX_ATTEMPTS}) — automation `{b.slug}`",
        "",
        "You are running ONE node of a graph of bounded nodes. Build this node's artifact and nothing else.",
        f"Task context (do not try to complete all of it here): {inputs.get('task','')}",
        f"Project root (all paths below are relative to it): `{b.project}`",
        "",
        "## Scope (your whole job)",
        str(node.get("scope", "")).strip(),
        "",
        "## Inputs — read these; do not go looking for more unless the scope requires it",
    ]
    lines += reads or ["- (no file inputs)"]
    if ctx:
        lines += ["", "## Bounded context from the knowledge graph — run these first and work from their output instead of reading the whole corpus",
                  "(if a command returns nothing useful, say so in `notes` and continue from the inputs)"]
        lines += ctx
    if lessons:
        lines += ["", f"Lessons from earlier runs (preferred sources, dead ends): `{lessons}` — skim it before you start."]
    lines += [
        "",
        "## Output — exactly one file",
        f"`{node['writes']}` — write the complete artifact here (create parent directories; overwriting is fine).",
        "",
        "## Check — the gate will run this against the saved file; exit code 0 means pass",
        "```bash",
        cmd,
        "```",
    ]
    if check.get("verify"):
        lines += ["", f"A separate verifier will also judge the artifact against: {check['verify']}"]
    if attempt > 1:
        lines += [
            "",
            "## Correction — the previous attempt failed its gate",
            "Reason and evidence:",
            "```",
            (reason or "(no evidence recorded)").strip(),
            "```",
            "Fix this node only. Do not widen the scope, do not touch other outputs, do not change the check.",
        ]
    scope = inputs.get("scope") or []
    lines += [
        "",
        "## Rules",
        f"- Allowed changes: {', '.join('`'+s+'`' for s in scope) if scope else 'only your output file'}. Nothing else; another node may own any other file.",
        "- This is a judgment node: write the artifact directly. Only if a sub-step inside it has exactly one correct answer and would be error-prone by hand"
        f" (counting, sorting or merging many entries) write a small helper under `{b.dir}/code/`; never generate prose from a script.",
        "- Do not publish, send, spend, or ask the user anything; if something blocks you, return verdict fail with the blocker as evidence.",
        "- Run the check yourself before returning. If it fails, fix and re-run; return fail honestly if it still fails.",
        "",
        "## Return format — your final message must be exactly this JSON and nothing else",
        "```json",
        json.dumps({"node": node["id"], "output_path": node["writes"], "verdict": "pass|fail", "evidence": "<the check's actual output, or why it failed>", "notes": "<optional, one line>"}, indent=2),
        "```",
    ]
    return "\n".join(lines) + "\n"


def render_verify_brief(b: Bundle, spec: dict, node: dict, attempt: int) -> str:
    check = node.get("check") or {}
    return "\n".join([
        f"# Verifier brief: {node['id']} (attempt {attempt}) — automation `{b.slug}`",
        "",
        "You are the independent gate for one node's artifact. You did not write it; judge it.",
        f"Project root: `{b.project}`",
        "",
        "## Artifact", f"`{node['writes']}`",
        "",
        "## Requirement to judge", str(check.get("verify", "")).strip(),
        "",
        "## Scope the node was given", str(node.get("scope", "")).strip(),
        "",
        "## Rules",
        "- Judge from the artifact and the inputs it cites, nothing else. Do not rebuild or re-derive the artifact from scratch.",
        "- Quote concrete evidence (paths, lines, numbers). When the artifact has many entries, check a stated sample plus every entry that looks doubtful.",
        "- Default to fail when you cannot find evidence; a gate opens on evidence, not on plausibility.",
        "- Do not fix anything. Do not edit any file. Keep this short: one pass, one verdict.",
        "",
        "## Return format — your final message must be exactly this JSON and nothing else",
        "```json",
        json.dumps({"node": node["id"], "role": "verify", "verdict": "pass|fail", "evidence": "<what you checked and what you found>"}, indent=2),
        "```",
        "",
    ])


def write_checks_md(b: Bundle, spec: dict, state: dict) -> None:
    inputs = spec.get("inputs") or {}
    lines = [f"# checks: {b.slug}", "", f"Run `{state['run_id']}` · updated {now()} · verdicts: pass, fail, unresolved", "",
             "## Node gates", "", "| node | requirement | verdict | evidence |", "|---|---|---|---|"]
    rows = 0
    for n in spec.get("nodes") or []:
        ns = state["nodes"].get(n["id"])
        if not ns:
            lines.append(f"| {n['id']} | (not run yet) | unresolved | — |")
            continue
        gates = ns.get("gates") or []
        if not gates:
            lines.append(f"| {n['id']} | gate | unresolved | no gate has run (status {ns.get('status')}) |")
            continue
        for g in gates:
            latest = g is gates[-1]
            for req, verdict, evidence in g["rows"]:
                mark = "" if latest else " (superseded)"
                lines.append(f"| {n['id']} | a{g['attempt']} {req}{mark} | {verdict} | {one_line(evidence)} · log `{g['log']}` |")
                rows += 1
    lines += ["", "## Acceptance checks (the deliverable)", "", "| check | verdict | evidence |", "|---|---|---|"]
    acc = {a["index"]: a for a in state.get("acceptance") or []}
    for i, c in enumerate(inputs.get("checks") or []):
        text = c.get("text") if isinstance(c, dict) else str(c)
        a = acc.get(i)
        lines.append(f"| {i}. {one_line(text, 160)} | {a['verdict'] if a else 'unresolved'} | {one_line(a['evidence']) if a else 'no evidence recorded'} |")
    base = state.get("baseline")
    if base:
        lines += ["", f"## Baseline before the run ({base['at']})", "", "| item | verdict | evidence |", "|---|---|---|"]
        lines += [f"| {one_line(i, 120)} | {v} | {one_line(e)} |" for i, v, e in base["rows"] if i != "note"]
        notes = [e for i, _, e in base["rows"] if i == "note"]
        if notes:
            lines += ["", "Observed before the run:", *[f"- {one_line(n)}" for n in notes]]
    comp = state.get("completion")
    if comp:
        lines += ["", f"## Completion ({comp['at']}) — {comp['status']}", "", "| item | verdict | evidence |", "|---|---|---|"]
        lines += [f"| {one_line(i, 120)} | {v} | {one_line(e)} |" for i, v, e in comp["rows"]]
    b.checks_md.write_text("\n".join(lines) + "\n", encoding="utf-8")


PROGRESS_TEMPLATE = """# progress: {slug}

<!-- graphctl:auto:begin status -->
<!-- graphctl:auto:end status -->

## Decisions

## Open issues

<!-- graphctl:auto:begin issues -->
<!-- graphctl:auto:end issues -->

## Next action

<!-- graphctl:auto:begin next -->
<!-- graphctl:auto:end next -->
"""


def compute_ready(spec: dict, state: dict) -> tuple[list[dict], list[str]]:
    """Nodes whose deps passed and that may still run. Returns (ready, notes)."""
    notes: list[str] = []
    nodes = spec.get("nodes") or []
    limit = int((spec.get("inputs") or {}).get("limit") or 0)
    ready: list[dict] = []
    for n in nodes:
        ns = state["nodes"].get(n["id"], {"status": "pending", "attempts": 0})
        if ns.get("status") in ("passed", "running", "blocked"):
            continue
        if ns.get("status") in ("reported", "unresolved"):
            notes.append(f"{n['id']} is {ns['status']}: it needs `graphctl gate` (and any missing report), not a new dispatch")
            continue
        if ns.get("attempts", 0) >= MAX_ATTEMPTS:
            continue
        deps_ok = all(state["nodes"].get(d, {}).get("status") == "passed" for d in n.get("deps") or [])
        if not deps_ok:
            continue
        if n.get("kind") == "model" and state.get("dispatches", 0) >= limit:
            notes.append(f"run limit reached ({state.get('dispatches')}/{limit} model dispatches); {n['id']} is ready but held")
            continue
        ready.append(n)
    return ready, notes


def write_progress_md(b: Bundle, spec: dict, state: dict) -> None:
    if not b.progress_md.is_file():
        b.progress_md.write_text(PROGRESS_TEMPLATE.format(slug=b.slug), encoding="utf-8")
    text = b.progress_md.read_text(encoding="utf-8")
    inputs = spec.get("inputs") or {}
    nodes = spec.get("nodes") or []
    buckets: dict[str, list[str]] = {"done": [], "running": [], "blocked": [], "failed": [], "unresolved": [], "pending": []}
    rows = ["| node | kind | status | attempts | last verdict |", "|---|---|---|---|---|"]
    for n in nodes:
        ns = state["nodes"].get(n["id"], {"status": "pending", "attempts": 0, "last_verdict": None})
        st = ns.get("status", "pending")
        key = {"passed": "done", "reported": "running"}.get(st, st)
        buckets.setdefault(key, []).append(n["id"])
        rows.append(f"| {n['id']} | {n.get('kind')} | {st} | {ns.get('attempts',0)}/{MAX_ATTEMPTS} | {ns.get('last_verdict') or '—'} |")
    outputs = []
    for n in nodes:
        p = b.project / n["writes"]
        ns = state["nodes"].get(n["id"], {})
        if p.is_file():
            h = sha256_file(p) or ""
            gate = ns.get("gate") or {}
            stale = " · CHANGED SINCE GATE" if gate.get("output_hash") and gate.get("output_hash") != h else ""
            outputs.append(f"- {n['id']} → `{n['writes']}` ({p.stat().st_size} bytes, sha {h[:8]}{stale})")
        else:
            outputs.append(f"- {n['id']} → `{n['writes']}` (not written yet)")
    limit = inputs.get("limit")
    status = "\n".join([
        f"Updated {now()} · run `{state['run_id']}` · model dispatches {state.get('dispatches',0)}/{limit}",
        "",
        "### Graph",
        "",
        *rows,
        "",
        f"Done: {', '.join(buckets['done']) or '—'} · Running: {', '.join(buckets['running']) or '—'} · "
        f"Blocked: {', '.join(buckets['blocked']) or '—'} · Failed: {', '.join(buckets['failed']) or '—'} · "
        f"Unresolved: {', '.join(buckets['unresolved']) or '—'} · Pending: {', '.join(buckets['pending']) or '—'}",
        "",
        "### Outputs",
        "",
        *outputs,
        f"- deliverable → `{inputs.get('output')}` ({'exists' if (b.project / str(inputs.get('output'))).exists() else 'not written yet'})",
    ])
    text = replace_between(text, "status", status)
    issues = []
    for n in nodes:
        ns = state["nodes"].get(n["id"], {})
        if ns.get("status") in ("failed", "blocked", "unresolved"):
            gate = ns.get("gate") or {}
            ev = "; ".join(f"{r}: {v}" for r, v, _ in gate.get("rows", []))
            issues.append(f"- {n['id']} is {ns['status']} after {ns.get('attempts',0)} attempt(s): {ev or 'no gate yet'} (log `{gate.get('log','—')}`)")
    for note in compute_ready(spec, state)[1]:
        issues.append(f"- {note}")
    text = replace_between(text, "issues", "\n".join(issues) or "- (no failed, blocked or unresolved nodes; manual notes, if any, follow)")
    ready, _ = compute_ready(spec, state)
    comp = state.get("completion")
    if comp and comp["status"] == "COMPLETE":
        nxt = "- complete: run `graphctl complete` again only after outputs change"
    elif ready:
        nxt = "- ready now: " + ", ".join(f"{n['id']} ({n['kind']})" for n in ready)
    elif buckets["running"]:
        nxt = "- waiting on running nodes: " + ", ".join(buckets["running"]) + " (record with `graphctl report`, then `graphctl gate`)"
    elif buckets["blocked"] or buckets["failed"] or buckets["unresolved"]:
        nxt = "- nothing ready; resolve the open issues above (correction edge is exhausted for blocked nodes)"
    else:
        nxt = "- nothing ready; run `graphctl complete`"
    text = replace_between(text, "next", nxt)
    b.progress_md.write_text(text, encoding="utf-8")


# ----------------------------------------------------------------------------
# commands
# ----------------------------------------------------------------------------

def cmd_init(a) -> None:
    b = Bundle(a.slug, a.root)
    if b.graph_md.exists() and not a.force:
        die(f"{b.graph_md} exists; use --force to overwrite the spec (state is kept unless --force)")
    for sub in ("out", "code", "runs"):
        (b.dir / sub).mkdir(parents=True, exist_ok=True)
    checks = []
    for c in a.checks or []:
        if "::" in c:
            text, cmd = c.split("::", 1)
            checks.append({"text": text.strip(), "cmd": cmd.strip()})
        else:
            checks.append(c.strip())
    spec = {
        "slug": a.slug,
        "version": 1,
        "approved": False,
        "approved_hash": None,
        "inputs": {
            "task": a.task,
            "output": a.output,
            "checks": checks,
            "sources": a.sources or [],
            "scope": a.scope or [a.output, f"{b.dir.as_posix()}/**"],
            "limit": a.limit,
        },
        "map": {"graph_json": a.graph_json, "lessons": a.lessons},
        "nodes": [],
    }
    b.graph_md.write_text(
        GRAPH_MD_TEMPLATE.format(
            slug=a.slug, prompt_path=PROMPT_PATH, engine=Path(__file__).resolve(), project=b.project,
            spec=json.dumps(spec, indent=2, ensure_ascii=False),
        ),
        encoding="utf-8",
    )
    state = {
        "run_id": "r1", "started": now(), "finished": None, "project_root": str(b.project),
        "dispatches": 0, "nodes": {}, "attempts": [], "acceptance": [], "events": [], "completion": None,
    }
    if b.state_path.exists() and not a.force:
        state = b.load_state()
    b.save_state(state)
    b.progress_md.write_text(PROGRESS_TEMPLATE.format(slug=a.slug), encoding="utf-8")
    write_checks_md(b, spec, state)
    write_progress_md(b, spec, state)
    print(f"bundle created at {b.dir}/")
    print(f"next: write the nodes into the spec ({b.graph_md}), or `graphctl spec {a.slug} --from spec.json`, then `graphctl validate {a.slug}`")


def cmd_spec(a) -> None:
    b = Bundle(a.slug, a.root)
    spec = b.load_spec()
    if a.from_file:
        new = json.loads(Path(a.from_file).read_text(encoding="utf-8"))
        for key, val in new.items():
            if key in ("inputs", "map") and isinstance(val, dict) and isinstance(spec.get(key), dict):
                spec[key].update(val)          # partial inputs/map are merged field by field
            elif key in ("approved", "approved_hash", "approved_at"):
                continue                       # approval is only granted by graphctl approve
            else:
                spec[key] = val                # nodes and anything else are replaced
        spec["slug"] = b.slug
        if spec.get("approved") and spec.get("approved_hash") != nodes_hash(spec.get("nodes") or []):
            spec["approved"] = False
            print("note: nodes changed since approval; approval cleared (re-run graphctl approve after review)")
        b.save_spec(spec)
        render_graph_md(b, spec)
        print(f"spec updated in {b.graph_md}")
        return
    print(json.dumps(spec, indent=2, ensure_ascii=False))


def cmd_validate(a) -> None:
    b = Bundle(a.slug, a.root)
    spec = b.load_spec()
    errors, warnings, levels = validate_spec(spec)
    for w in warnings:
        print(f"warning: {w}")
    for e in errors:
        print(f"error: {e}")
    if errors:
        print(f"INVALID: {len(errors)} error(s)")
        sys.exit(1)
    print(f"VALID: {len(spec['nodes'])} nodes, {sum(len(n.get('deps') or []) for n in spec['nodes'])} edges, {len(levels)} parallel levels")
    for i, level in enumerate(levels):
        print(f"  L{i}: {', '.join(level)}")


def cmd_render(a) -> None:
    b = Bundle(a.slug, a.root)
    spec = b.load_spec()
    render_graph_md(b, spec)
    state = b.load_state()
    write_checks_md(b, spec, state)
    write_progress_md(b, spec, state)
    print(f"rendered {b.graph_md}, {b.checks_md}, {b.progress_md}")


def cmd_approve(a) -> None:
    b = Bundle(a.slug, a.root)
    spec = b.load_spec()
    errors, _, _ = validate_spec(spec)
    if errors:
        die("cannot approve an invalid spec:\n  " + "\n  ".join(errors))
    spec["approved"] = True
    spec["approved_hash"] = nodes_hash(spec["nodes"])
    spec["approved_at"] = now()
    b.save_spec(spec)
    render_graph_md(b, spec)
    state = b.load_state()
    b.event(state, "approve", f"spec approved (nodes hash {spec['approved_hash']})")
    b.save_state(state)
    print(f"approved: {len(spec['nodes'])} nodes, hash {spec['approved_hash']}")


def cmd_prompt(a) -> None:
    b = Bundle(a.slug, a.root)
    spec = b.load_spec()
    state = b.load_state()
    if not PROMPT_PATH.is_file():
        die(f"governing prompt missing at {PROMPT_PATH}")
    inputs = spec.get("inputs") or {}
    checks = inputs.get("checks") or []
    check_lines = "\n".join(f"- {c['text']} (check: `{c['cmd']}`)" if isinstance(c, dict) else f"- {c}" for c in checks) or "(none given)"
    filled = PROMPT_PATH.read_text(encoding="utf-8")
    for key, val in {
        "TASK": inputs.get("task", ""),
        "OUTPUT": inputs.get("output", ""),
        "CHECKS": check_lines,
        "SOURCES": ", ".join(inputs.get("sources") or []) or "(none given)",
        "SCOPE": ", ".join(inputs.get("scope") or []) or "(unrestricted)",
        "LIMIT": str(inputs.get("limit", "")),
    }.items():
        filled = filled.replace("{{" + key + "}}", str(val))
    out = b.runs_dir(state) / "prompt.md"
    out.write_text(filled, encoding="utf-8")
    print(f"filled governing prompt written to {out}")
    if a.show:
        print()
        print(filled)
    else:
        print("(read it now if you have not read references/prompt.md in this session; --show prints it here)")


def cmd_ready(a) -> None:
    b = Bundle(a.slug, a.root)
    spec = b.load_spec()
    state = b.load_state()
    errors, _, _ = validate_spec(spec)
    if errors:
        die("spec invalid; run graphctl validate")
    if not spec.get("approved") and not a.unapproved:
        die("spec is not approved; run graphctl approve (after review) or pass --unapproved to peek")
    ready, notes = compute_ready(spec, state)
    if a.json:
        print(json.dumps({"ready": ready, "notes": notes, "dispatches": state.get("dispatches", 0), "limit": spec["inputs"].get("limit")}, indent=2))
        return
    for note in notes:
        print(f"note: {note}")
    if not ready:
        print("ready: none")
        return
    print("ready:")
    for n in ready:
        att = state["nodes"].get(n["id"], {}).get("attempts", 0)
        print(f"  {n['id']} ({n['kind']}, attempt {att + 1} of {MAX_ATTEMPTS}){' — RETRY via correction edge' if att else ''}")


def _start_attempt(b: Bundle, spec: dict, state: dict, node: dict, role: str) -> tuple[int, str | None]:
    ns = b.node_state(state, node["id"])
    if ns["status"] == "passed":
        die(f"{node['id']} already passed; use rerun to start a new run")
    if ns["status"] == "running":
        die(f"{node['id']} is already running (record its result with graphctl report)")
    if ns["attempts"] >= MAX_ATTEMPTS:
        ns["status"] = "blocked"
        b.save_state(state)
        die(f"{node['id']} exhausted {MAX_ATTEMPTS} attempts; it is blocked — report it, do not weaken the gate")
    for d in node.get("deps") or []:
        if state["nodes"].get(d, {}).get("status") != "passed":
            die(f"{node['id']} is not ready: upstream {d} has not passed its gate")
    reason = None
    if ns["attempts"] and ns.get("gate"):
        reason = "\n".join(f"{r}: {v} — {e}" for r, v, e in ns["gate"]["rows"] if v != "pass")
    ns["attempts"] += 1
    ns["status"] = "running"
    if node.get("kind") == "model":
        state["dispatches"] = state.get("dispatches", 0) + 1
    state["attempts"].append({"node": node["id"], "attempt": ns["attempts"], "role": role, "started": now(), "finished": None,
                              "verdict": None, "output": node["writes"], "evidence": None, "tokens": None, "reason": reason})
    if node.get("risky") or (node.get("kind") == "code" and (b.project / node["writes"]).exists()):
        snap = snapshot_output(b, state, node)
        snapshot = snap or f"skipped: {node['writes']} did not exist yet"
        b.event(state, "snapshot", f"{node['id']}: {('saved ' + node['writes'] + ' to ' + snap) if snap else snapshot}")
    else:
        snapshot = "n/a (node is not risky)"
    state["attempts"][-1]["snapshot"] = snapshot
    b.save_state(state)
    return ns["attempts"], reason


def snapshot_output(b: Bundle, state: dict, node: dict) -> str | None:
    src = b.project / node["writes"]
    if not src.is_file():
        return None
    ns = b.node_state(state, node["id"])
    dst = b.runs_dir(state) / "snapshots" / node["id"] / f"a{ns['attempts']}" / Path(node["writes"]).name
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)
    return dst.relative_to(b.project).as_posix() if dst.is_relative_to(b.project) else str(dst)


def cmd_snapshot(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    node = node_map(spec).get(a.node) or die(f"unknown node {a.node}")
    snap = snapshot_output(b, state, node)
    b.save_state(state)
    print(snap or f"nothing to snapshot: {node['writes']} does not exist yet")


def cmd_dispatch(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    if not spec.get("approved"):
        die("spec is not approved; run graphctl approve after review")
    node = node_map(spec).get(a.node) or die(f"unknown node {a.node}")
    if node.get("kind") != "model":
        die(f"{a.node} is a code node; use graphctl run-code")
    limit = int(spec["inputs"].get("limit") or 0)
    if state.get("dispatches", 0) >= limit:
        die(f"run limit reached ({state['dispatches']}/{limit} model dispatches); stop and report partial status")
    attempt, reason = _start_attempt(b, spec, state, node, "node")
    brief = render_brief(b, spec, state, node, attempt, reason)
    path = b.runs_dir(state) / "briefs" / f"{node['id']}.a{attempt}.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(brief, encoding="utf-8")
    b.event(state, "dispatch", f"{node['id']} attempt {attempt} (brief {path.relative_to(b.project) if path.is_relative_to(b.project) else path})")
    b.save_state(state)
    write_progress_md(b, spec, state)
    print(f"brief saved to {path} (stdout below is the brief, verbatim)", file=sys.stderr)
    print(brief)


def cmd_brief(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    node = node_map(spec).get(a.node) or die(f"unknown node {a.node}")
    ns = state["nodes"].get(a.node, {"attempts": 0})
    attempt = max(1, ns.get("attempts", 0))
    if a.verify:
        if not (node.get("check") or {}).get("verify"):
            die(f"{a.node} has no check.verify requirement")
        print(render_verify_brief(b, spec, node, attempt))
        return
    reason = None
    if ns.get("gate"):
        reason = "\n".join(f"{r}: {v} — {e}" for r, v, e in ns["gate"]["rows"] if v != "pass")
    shown = attempt if ns.get("status") == "running" else ns.get("attempts", 0) + 1
    print(render_brief(b, spec, state, node, shown, reason))


def _record(state: dict, node_id: str, role: str, verdict: str, evidence: str, output: str | None, tokens: int | None) -> dict:
    rec = None
    if role == "node":
        for r in reversed(state["attempts"]):
            # an attempt is open until it has a verdict: a gate that ran before the report
            # closes it as unresolved, and the documented flow is "record the report, gate again"
            if r["node"] == node_id and r["role"] in ("node", "code") and (r["finished"] is None or r.get("verdict") is None):
                rec = r
                break
        if rec is None:
            die(f"no open attempt for {node_id}; dispatch it first")
        rec.update({"finished": now(), "verdict": verdict, "evidence": evidence, "tokens": tokens})
        if output:
            rec["output"] = output
    else:
        ns = state["nodes"].get(node_id) or die(f"{node_id} has no attempts")
        rec = {"node": node_id, "attempt": ns["attempts"], "role": role, "started": now(), "finished": now(),
               "verdict": verdict, "evidence": evidence, "tokens": tokens, "output": output, "reason": None}
        state["attempts"].append(rec)
    return rec


def cmd_report(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    node = node_map(spec).get(a.node) or die(f"unknown node {a.node}")
    verdict, evidence, output, tokens, role = a.verdict, a.evidence, a.output, a.tokens, a.role
    if a.json_file:
        a.json_payload = Path(a.json_file).read_text(encoding="utf-8")
    if a.json_payload:
        raw = a.json_payload.strip()
        raw = re.sub(r"^```(?:json)?\s*|\s*```$", "", raw, flags=re.S)
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError:
            m = re.search(r"\{.*\}", raw, re.S)
            payload = json.loads(m.group(0)) if m else die("--json payload is not JSON")
        verdict = payload.get("verdict", verdict)
        evidence = payload.get("evidence", evidence) or ""
        if payload.get("notes"):
            evidence = f"{evidence} | notes: {payload['notes']}"
        output = payload.get("output_path", output)
        role = payload.get("role", role)
        tokens = payload.get("tokens", tokens)
    if verdict not in ("pass", "fail"):
        die("verdict must be pass or fail")
    if role == "verify" and not (node.get("check") or {}).get("verify"):
        die(f"{a.node} has no check.verify; a verifier report is not expected")
    if output and output != node["writes"]:
        print(f"warning: reported output {output} differs from spec writes {node['writes']}; the gate checks {node['writes']}")
    ns = b.node_state(state, a.node)
    _record(state, a.node, role, verdict, evidence or "", output, tokens)
    if tokens:
        ns["tokens"] = (ns.get("tokens") or 0) + int(tokens)
    if role == "node":
        ns["status"] = "reported"
        ns["last_verdict"] = f"self:{verdict}"
    b.event(state, "report", f"{a.node} {role} verdict={verdict}")
    b.save_state(state)
    print(f"recorded {role} report for {a.node}: {verdict}. next: graphctl gate {b.slug} {a.node}")


def cmd_run_code(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    if not spec.get("approved"):
        die("spec is not approved; run graphctl approve after review")
    node = node_map(spec).get(a.node) or die(f"unknown node {a.node}")
    if node.get("kind") != "code":
        die(f"{a.node} is a model node; use graphctl dispatch")
    attempt, _ = _start_attempt(b, spec, state, node, "code")
    env = dict(os.environ, NODE_ID=node["id"], NODE_OUTPUT=str(b.project / node["writes"]), BUNDLE_DIR=str(b.dir),
               RUN_ID=state["run_id"], PROJECT_ROOT=str(b.project))
    (b.project / node["writes"]).parent.mkdir(parents=True, exist_ok=True)
    rc, out = run_shell(node["run"], b.project, env, int(node.get("timeout") or DEFAULT_TIMEOUT))
    log = b.runs_dir(state) / "gates" / f"{node['id']}.a{attempt}.run.txt"
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(f"$ {node['run']}\n(exit {rc})\n{out}\n", encoding="utf-8")
    verdict = "pass" if rc == 0 else "fail"
    _record(state, node["id"], "node", verdict, f"exit {rc}: {out[:2000]}", node["writes"], None)
    b.node_state(state, node["id"])["last_verdict"] = f"run:{verdict}"
    b.save_state(state)
    print(f"{node['id']}: `{node['run']}` exit {rc}" + (f"\n{out[:1500]}" if out else ""))
    a.node = node["id"]
    cmd_gate(a)


def cmd_gate(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    node = node_map(spec).get(a.node) or die(f"unknown node {a.node}")
    ns = b.node_state(state, a.node)
    if ns["attempts"] == 0:
        die(f"{a.node} has not been dispatched")
    attempt = ns["attempts"]
    check = node.get("check") or {}
    rows: list[tuple[str, str, str]] = []
    env = dict(os.environ, NODE_ID=node["id"], NODE_OUTPUT=str(b.project / node["writes"]), BUNDLE_DIR=str(b.dir),
               RUN_ID=state["run_id"], PROJECT_ROOT=str(b.project))
    out_path = b.project / node["writes"]
    # 1. deterministic check first
    cmd = check.get("cmd") or f"test -s {shlex.quote(node['writes'])}"
    if not out_path.is_file():
        rows.append(("check.cmd", "fail", f"output {node['writes']} does not exist"))
        rc, out = 1, "output missing"
    else:
        rc, out = run_shell(cmd, b.project, env, int(check.get("timeout") or DEFAULT_TIMEOUT))
        rows.append(("check.cmd", "pass" if rc == 0 else "fail", f"`{cmd}` → exit {rc}: {out or '(no output)'}"))
    # 2. the node's own report
    reports = [r for r in state["attempts"] if r["node"] == node["id"] and r["attempt"] == attempt and r["role"] in ("node", "code")]
    if not reports or reports[-1]["finished"] is None:
        rows.append(("node.report", "unresolved", "no report recorded for this attempt (graphctl report)"))
    else:
        # a closed attempt with no verdict (gated before its report) is unresolved, never a pass
        rows.append(("node.report", reports[-1].get("verdict") or "unresolved", reports[-1].get("evidence") or "no report recorded for this attempt (graphctl report)"))
    # 3. the verifier's judgment, last
    if check.get("verify"):
        vs = [r for r in state["attempts"] if r["node"] == node["id"] and r["attempt"] == attempt and r["role"] == "verify"]
        if not vs:
            rows.append(("check.verify", "unresolved", f"verifier has not reported: {check['verify']}"))
        else:
            rows.append(("check.verify", vs[-1]["verdict"], vs[-1]["evidence"] or ""))
    verdicts = [v for _, v, _ in rows]
    verdict = "fail" if "fail" in verdicts else ("unresolved" if "unresolved" in verdicts else "pass")
    log = b.runs_dir(state) / "gates" / f"{node['id']}.a{attempt}.md"
    log.parent.mkdir(parents=True, exist_ok=True)
    snapshot = next((r.get("snapshot") for r in reversed(state["attempts"]) if r["node"] == node["id"] and r["attempt"] == attempt and r.get("snapshot")), "n/a")
    log.write_text("\n".join([f"# gate {node['id']} attempt {attempt}: {verdict} ({now()})", "", f"snapshot before this attempt: {snapshot}", ""] +
                             [f"## {r}: {v}\n\n```\n{e}\n```\n" for r, v, e in rows]), encoding="utf-8")
    rel_log = log.relative_to(b.project).as_posix() if log.is_relative_to(b.project) else str(log)
    gate = {"attempt": attempt, "verdict": verdict, "rows": rows, "at": now(), "output_hash": sha256_file(out_path), "log": rel_log}
    ns["gate"] = gate
    ns.setdefault("gates", []).append(gate)
    ns["last_verdict"] = f"gate:{verdict}"
    if verdict == "pass":
        ns["status"] = "passed"
        ns["output_hash"] = gate["output_hash"]
    elif verdict == "fail":
        ns["status"] = "blocked" if attempt >= MAX_ATTEMPTS else "failed"
    else:
        ns["status"] = "unresolved"
    for r in state["attempts"]:
        if r["node"] == node["id"] and r["attempt"] == attempt and r["role"] in ("node", "code") and r["finished"] is None:
            r["finished"] = now()
    b.event(state, "gate", f"{node['id']} attempt {attempt}: {verdict}")
    b.save_state(state)
    write_checks_md(b, spec, state)
    write_progress_md(b, spec, state)
    print(f"GATE {node['id']} attempt {attempt}: {verdict.upper()}")
    for r, v, e in rows:
        print(f"  {r}: {v} — {console(e)}")
    if ns["status"] == "failed":
        print(f"  → correction edge: dispatch {node['id']} again with the evidence above (attempt {attempt + 1} of {MAX_ATTEMPTS})")
    elif ns["status"] == "blocked":
        print(f"  → blocked: {MAX_ATTEMPTS} attempts used; report it in progress.md, do not weaken the gate")
    elif ns["status"] == "unresolved":
        print("  → unresolved: record the missing report, then gate again")
    sys.exit({"pass": 0, "fail": 1, "unresolved": 2}[verdict])


def cmd_accept(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    checks = (spec.get("inputs") or {}).get("checks") or []
    acc = {x["index"]: x for x in state.get("acceptance") or []}
    env = dict(os.environ, BUNDLE_DIR=str(b.dir), RUN_ID=state["run_id"], PROJECT_ROOT=str(b.project),
               OUTPUT=str(b.project / str(spec["inputs"].get("output"))))
    if a.run:
        for i, c in enumerate(checks):
            if isinstance(c, dict) and c.get("cmd"):
                rc, out = run_shell(c["cmd"], b.project, env, DEFAULT_TIMEOUT)
                acc[i] = {"index": i, "verdict": "pass" if rc == 0 else "fail", "evidence": f"`{c['cmd']}` → exit {rc}: {out or '(no output)'}", "at": now()}
                print(f"{i}. {c['text']}: {acc[i]['verdict']}")
            else:
                print(f"{i}. {c if isinstance(c, str) else c.get('text')}: no cmd — record manually with --check {i} --verdict ... --evidence ...")
    else:
        if a.check is None or a.verdict not in VERDICTS or not a.evidence:
            die("usage: accept <slug> --run | --check N --verdict pass|fail|unresolved --evidence '...'")
        if a.check >= len(checks):
            die(f"check index {a.check} out of range ({len(checks)} checks)")
        acc[a.check] = {"index": a.check, "verdict": a.verdict, "evidence": a.evidence, "at": now()}
    state["acceptance"] = [acc[k] for k in sorted(acc)]
    b.save_state(state)
    write_checks_md(b, spec, state)
    print(f"acceptance recorded in {b.checks_md}")


def cmd_baseline(a) -> None:
    """Run the deterministic acceptance checks BEFORE any node runs, so the run
    measures a change against a recorded starting point."""
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    checks = (spec.get("inputs") or {}).get("checks") or []
    env = dict(os.environ, BUNDLE_DIR=str(b.dir), RUN_ID=state["run_id"], PROJECT_ROOT=str(b.project),
               OUTPUT=str(b.project / str(spec["inputs"].get("output"))))
    rows = []
    out = b.project / str(spec["inputs"].get("output"))
    rows.append(("deliverable exists before the run", "pass" if out.is_file() and out.stat().st_size > 0 else "fail",
                 f"{spec['inputs'].get('output')} ({out.stat().st_size} bytes)" if out.is_file() else f"{spec['inputs'].get('output')} absent"))
    for i, c in enumerate(checks):
        text = c.get("text") if isinstance(c, dict) else str(c)
        if isinstance(c, dict) and c.get("cmd"):
            rc, outp = run_shell(c["cmd"], b.project, env, DEFAULT_TIMEOUT)
            rows.append((f"acceptance {i}: {text}", "pass" if rc == 0 else "fail", f"`{c['cmd']}` → exit {rc}: {outp or '(no output)'}"))
        else:
            rows.append((f"acceptance {i}: {text}", "unresolved", "no cmd; judged at completion"))
    for note in a.note or []:
        rows.append(("note", "noted", note))
    state["baseline"] = {"at": now(), "rows": rows}
    b.event(state, "baseline", f"{sum(1 for r in rows if r[1] == 'pass')}/{len(rows)} pass before the run")
    b.save_state(state)
    write_checks_md(b, spec, state)
    print(f"baseline recorded ({state['baseline']['at']}):")
    for item, v, e in rows:
        print(f"  {v}: {item} — {console(e, 160)}")
    print(f"recorded in {b.checks_md}")


def cmd_complete(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    rows: list[tuple[str, str, str]] = []
    errors, _, _ = validate_spec(spec)
    rows.append(("graph.md is a valid spec", "pass" if not errors else "fail", "; ".join(errors) or "validate ok"))
    rows.append(("spec approved before execution", "pass" if spec.get("approved") else "fail",
                 f"hash {spec.get('approved_hash')}" if spec.get("approved") else "not approved"))
    left: list[str] = []
    for n in spec.get("nodes") or []:
        ns = state["nodes"].get(n["id"])
        p = b.project / n["writes"]
        exists = p.is_file() and p.stat().st_size > 0
        rows.append((f"{n['id']}: output exists and can be opened", "pass" if exists else "fail", f"{n['writes']} ({p.stat().st_size} bytes)" if p.is_file() else f"{n['writes']} missing"))
        if not ns or not ns.get("gate"):
            rows.append((f"{n['id']}: gate ran against the saved version", "fail", "no gate recorded"))
            left.append(n["id"])
            continue
        g = ns["gate"]
        fresh = g.get("output_hash") == sha256_file(p)
        rows.append((f"{n['id']}: gate ran against the saved version", "pass" if (g["verdict"] == "pass" and fresh) else "fail",
                     f"gate {g['verdict']} at {g['at']}" + ("" if fresh else " but the output changed afterwards — gate again")))
        if not (g["verdict"] == "pass" and fresh):
            left.append(n["id"])
    ran = set(state["nodes"])
    declared = {n["id"] for n in spec.get("nodes") or []}
    outside = sorted(ran - declared)
    not_run = sorted(declared - ran)
    rows.append(("graph.md matches the nodes that actually ran", "pass" if not outside else "fail",
                 f"ran {len(ran & declared)} of {len(declared)} declared nodes" + (f"; not run yet: {not_run}" if not_run else "") + (f"; RAN OUTSIDE THE GRAPH: {outside}" if outside else "")))
    left.extend(n for n in not_run if n not in left)
    # acceptance checks
    checks = (spec.get("inputs") or {}).get("checks") or []
    acc = {x["index"]: x for x in state.get("acceptance") or []}
    env = dict(os.environ, BUNDLE_DIR=str(b.dir), RUN_ID=state["run_id"], PROJECT_ROOT=str(b.project),
               OUTPUT=str(b.project / str(spec["inputs"].get("output"))))
    for i, c in enumerate(checks):
        text = c.get("text") if isinstance(c, dict) else str(c)
        if isinstance(c, dict) and c.get("cmd"):
            rc, out = run_shell(c["cmd"], b.project, env, DEFAULT_TIMEOUT)
            acc[i] = {"index": i, "verdict": "pass" if rc == 0 else "fail", "evidence": f"`{c['cmd']}` → exit {rc}: {out or '(no output)'}", "at": now()}
        rec = acc.get(i)
        rows.append((f"acceptance {i}: {text}", rec["verdict"] if rec else "unresolved", rec["evidence"] if rec else "no evidence recorded (graphctl accept)"))
    state["acceptance"] = [acc[k] for k in sorted(acc)]
    out = b.project / str(spec["inputs"].get("output"))
    rows.append(("deliverable exists", "pass" if out.is_file() and out.stat().st_size > 0 else "fail", str(spec["inputs"].get("output"))))
    limit = int(spec["inputs"].get("limit") or 0)
    rows.append(("run limit respected", "pass" if state.get("dispatches", 0) <= limit else "fail", f"{state.get('dispatches',0)}/{limit} model dispatches"))
    failing = [r for r in rows if r[1] != "pass"]
    status = "COMPLETE" if not failing else "PARTIAL"
    state["completion"] = {"at": now(), "status": status, "rows": rows, "nodes_left": left}
    state["finished"] = now() if status == "COMPLETE" else None
    b.event(state, "complete", status)
    b.save_state(state)
    write_checks_md(b, spec, state)
    write_progress_md(b, spec, state)
    tokens = sum(int(r.get("tokens") or 0) for r in state["attempts"])
    print(f"{status}: {len(rows) - len(failing)}/{len(rows)} checklist items pass")
    for item, v, e in failing:
        print(f"  {v}: {item} — {console(e, 160)}")
    if left:
        print(f"nodes left: {', '.join(left)}")
    print(f"measured usage: {tokens:,} tokens recorded across attempts" if tokens else "measured usage: none recorded (pass --tokens to graphctl report when the Agent result shows usage)")
    sys.exit(0 if status == "COMPLETE" else 1)


def cmd_progress(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    write_progress_md(b, spec, state)
    text = b.progress_md.read_text(encoding="utf-8")
    stamp = now()
    for note in a.decision or []:
        text = append_under(text, "Decisions", f"- {stamp}: {note}")
    for note in a.issue or []:
        text = append_under(text, "Open issues", f"- {stamp}: {note}")
    b.progress_md.write_text(text, encoding="utf-8")
    print(f"updated {b.progress_md}")


def cmd_decide(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    stamp = now()
    g = b.graph_md.read_text(encoding="utf-8")
    b.graph_md.write_text(append_under(g, "Decisions that changed the graph", f"- {stamp}: {a.text}"), encoding="utf-8")
    write_progress_md(b, spec, state)
    p = b.progress_md.read_text(encoding="utf-8")
    b.progress_md.write_text(append_under(p, "Decisions", f"- {stamp}: {a.text}"), encoding="utf-8")
    b.event(state, "decision", a.text)
    b.save_state(state)
    print("decision recorded in graph.md and progress.md")


def cmd_learn(a) -> None:
    b = Bundle(a.slug, a.root)
    state = b.load_state()
    if a.node and state["nodes"].get(a.node, {}).get("status") != "passed":
        die(f"learning edge only fires after an accepted result; {a.node} has not passed its gate")
    g = b.graph_md.read_text(encoding="utf-8")
    tag = f" (proved by {a.node})" if a.node else ""
    b.graph_md.write_text(append_under(g, "Learned rules", f"- {now()}{tag}: {a.text}"), encoding="utf-8")
    b.event(state, "learn", a.text)
    b.save_state(state)
    print("rule recorded under 'Learned rules' in graph.md")


def cmd_status(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    ready, notes = compute_ready(spec, state)
    summary = {
        "slug": b.slug, "run_id": state["run_id"], "approved": bool(spec.get("approved")),
        "dispatches": state.get("dispatches", 0), "limit": spec["inputs"].get("limit"),
        "nodes": {n["id"]: {"kind": n["kind"], "status": state["nodes"].get(n["id"], {}).get("status", "pending"),
                            "attempts": state["nodes"].get(n["id"], {}).get("attempts", 0)} for n in spec.get("nodes") or []},
        "ready": [n["id"] for n in ready], "notes": notes,
        "completion": (state.get("completion") or {}).get("status"),
    }
    if a.json:
        print(json.dumps(summary, indent=2))
        return
    print(f"{b.slug} · run {summary['run_id']} · approved={summary['approved']} · dispatches {summary['dispatches']}/{summary['limit']} · completion={summary['completion']}")
    for nid, s in summary["nodes"].items():
        print(f"  {nid:<24} {s['kind']:<5} {s['status']:<10} attempts {s['attempts']}/{MAX_ATTEMPTS}")
    print("ready: " + (", ".join(summary["ready"]) or "none"))
    for note in notes:
        print(f"note: {note}")


def cmd_rerun(a) -> None:
    b = Bundle(a.slug, a.root)
    spec, state = b.load_spec(), b.load_state()
    old = b.runs_dir(state)
    (old / "state.final.json").write_text(json.dumps(state, indent=2), encoding="utf-8")
    for n in spec.get("nodes") or []:
        src = b.project / n["writes"]
        if src.is_file():
            dst = old / "outputs" / n["writes"]
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
    num = int(re.sub(r"\D", "", state["run_id"]) or 0) + 1
    inputs = spec["inputs"]
    if a.sources:
        inputs["sources"] = a.sources
    if a.output:
        inputs["output"] = a.output
    if a.limit:
        inputs["limit"] = a.limit
    if a.graph_json:
        spec.setdefault("map", {})["graph_json"] = a.graph_json
    if spec.get("approved") and spec.get("approved_hash") != nodes_hash(spec.get("nodes") or []):
        spec["approved"] = False
    b.save_spec(spec)
    state = {"run_id": f"r{num}", "started": now(), "finished": None, "project_root": str(b.project), "dispatches": 0,
             "nodes": {}, "attempts": [], "acceptance": [], "events": [{"at": now(), "type": "rerun", "msg": f"from {old.name}"}],
             "completion": None, "baseline": None}
    b.save_state(state)
    render_graph_md(b, spec)
    write_checks_md(b, spec, state)
    write_progress_md(b, spec, state)
    print(f"new run {state['run_id']} started (previous run archived under {old}/). approved={spec.get('approved')}")
    if not spec.get("approved"):
        print("the node graph changed since approval: review and `graphctl approve` before dispatching")


# ----------------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------------

def main(argv: list[str] | None = None) -> None:
    p = argparse.ArgumentParser(prog="graphctl", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)

    def add(name, fn, **kw):
        sp = sub.add_parser(name, **kw)
        sp.add_argument("slug")
        sp.add_argument("--root", default=DEFAULT_ROOT, help=f"bundles directory (default {DEFAULT_ROOT}/)")
        sp.set_defaults(fn=fn)
        return sp

    sp = add("init", cmd_init, help="create a bundle")
    sp.add_argument("--task", required=True)
    sp.add_argument("--output", required=True, help="deliverable path (relative to project root)")
    sp.add_argument("--checks", nargs="*", help="acceptance checks; 'text::shell command' makes one deterministic")
    sp.add_argument("--sources", nargs="*")
    sp.add_argument("--scope", nargs="*", help="allowed-change globs; default: the output and the bundle")
    sp.add_argument("--limit", type=int, default=12, help="max model-node attempts per run")
    sp.add_argument("--graph-json", help="graphify graph.json for this corpus")
    sp.add_argument("--lessons", help="graphify LESSONS.md path, if any")
    sp.add_argument("--force", action="store_true")
    sp = add("spec", cmd_spec, help="print or replace the JSON spec")
    sp.add_argument("--from", dest="from_file", help="JSON file: a full spec, or {\"nodes\": [...]} to replace nodes only")
    add("validate", cmd_validate, help="validate the spec")
    add("render", cmd_render, help="refresh derived tables and reports")
    add("approve", cmd_approve, help="mark the spec approved")
    sp = add("prompt", cmd_prompt, help="write the filled governing prompt")
    sp.add_argument("--show", action="store_true", help="also print the filled prompt")
    sp = add("ready", cmd_ready, help="list ready nodes")
    sp.add_argument("--json", action="store_true")
    sp.add_argument("--unapproved", action="store_true")
    sp = add("dispatch", cmd_dispatch, help="start a model-node attempt; prints the brief")
    sp.add_argument("node")
    sp = add("brief", cmd_brief, help="print a brief without dispatching")
    sp.add_argument("node")
    sp.add_argument("--verify", action="store_true", help="print the verifier brief instead")
    sp = add("run-code", cmd_run_code, help="run a code node and gate it")
    sp.add_argument("node")
    sp = add("report", cmd_report, help="record a node or verifier result")
    sp.add_argument("node")
    sp.add_argument("--verdict", choices=("pass", "fail"))
    sp.add_argument("--evidence", default="")
    sp.add_argument("--output")
    sp.add_argument("--tokens", type=int)
    sp.add_argument("--role", choices=("node", "verify"), default="node")
    sp.add_argument("--json", dest="json_payload", help="the subagent's returned JSON, verbatim")
    sp.add_argument("--json-file", help="file holding the subagent's returned JSON (safer than quoting it)")
    sp = add("gate", cmd_gate, help="run the gate for a node")
    sp.add_argument("node")
    sp = add("accept", cmd_accept, help="record or run acceptance checks")
    sp.add_argument("--run", action="store_true", help="run every acceptance check that has a cmd")
    sp.add_argument("--check", type=int)
    sp.add_argument("--verdict", choices=VERDICTS)
    sp.add_argument("--evidence")
    sp = add("baseline", cmd_baseline, help="record the acceptance checks' state before any node runs")
    sp.add_argument("--note", action="append", help="free-text baseline observation (e.g. a measured number)")
    add("complete", cmd_complete, help="run the completion checklist")
    sp = add("progress", cmd_progress, help="regenerate progress.md; add notes")
    sp.add_argument("--decision", action="append")
    sp.add_argument("--issue", action="append")
    sp = add("decide", cmd_decide, help="record a decision that changed the graph")
    sp.add_argument("text")
    sp = add("learn", cmd_learn, help="record a learned rule")
    sp.add_argument("text")
    sp.add_argument("--node")
    sp = add("status", cmd_status, help="show status")
    sp.add_argument("--json", action="store_true")
    sp = add("snapshot", cmd_snapshot, help="snapshot a node's output")
    sp.add_argument("node")
    sp = add("rerun", cmd_rerun, help="archive the run and start a new one")
    sp.add_argument("--sources", nargs="*")
    sp.add_argument("--output")
    sp.add_argument("--limit", type=int)
    sp.add_argument("--graph-json")

    a = p.parse_args(argv)
    a.fn(a)


if __name__ == "__main__":
    main()
