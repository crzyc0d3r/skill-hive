#!/usr/bin/env python3
"""graph_context - turn a graphify graph.json into inputs for splitting a task into nodes.

The split rule is "split by what each node must read, not by folder". graphify's
communities are exactly that: sets of symbols and documents that are read together.
This script prints, deterministically, what the model needs to draw node boundaries:

  summary  --graph P              communities, their files, god nodes, cross-community edges
  focus    --graph P --query "…"  the communities, files and hubs a task touches, with a draft split
  files    --graph P --community N  every file in one community

Output is Markdown for the model to read (add --json for machines). No LLM is used.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

STOP = set("""a an the and or of to for in on at by with from into over under about as is are was were be been
being this that these those it its their there here which who whom what when where why how all any each every
some such no not only own same so than too very can will just should would could may might must shall do does
did done have has had having make makes made use uses used using get gets got run runs write writes read reads
new one two three first last then also per via""".split())


def load(graph: Path) -> tuple[dict, dict[str, dict], dict[str, set[str]], list[dict]]:
    data = json.loads(graph.read_text(encoding="utf-8"))
    nodes = {n["id"]: n for n in data.get("nodes", [])}
    links = data.get("links") or data.get("edges") or []
    adj: dict[str, set[str]] = defaultdict(set)
    for l in links:
        s, t = l.get("source"), l.get("target")
        if s in nodes and t in nodes and s != t:
            adj[s].add(t)
            adj[t].add(s)
    return data, nodes, adj, links


def is_internal(n: dict) -> bool:
    return not n.get("external") and bool(n.get("source_file"))


def cname(cid, labels: dict | None, nodes_in: list[dict]) -> str:
    for n in nodes_in:
        if n.get("community_name"):
            return n["community_name"]
    if labels and str(cid) in labels:
        return labels[str(cid)]
    return f"Community {cid}" if cid is not None else "unassigned"


def load_labels(graph: Path) -> dict | None:
    p = graph.parent / ".graphify_labels.json"
    if p.is_file():
        try:
            return json.loads(p.read_text(encoding="utf-8"))
        except Exception:
            return None
    return None


def communities(nodes: dict[str, dict], adj: dict[str, set[str]], labels: dict | None) -> list[dict]:
    groups: dict = defaultdict(list)
    for n in nodes.values():
        if is_internal(n):
            groups[n.get("community")].append(n)
    out = []
    for cid, members in groups.items():
        files = Counter(n["source_file"] for n in members)
        top = sorted(members, key=lambda n: -len(adj.get(n["id"], ())))[:6]
        out.append({
            "id": cid, "name": cname(cid, labels, members), "nodes": len(members),
            "files": [f for f, _ in files.most_common()],
            "top": [f"{n.get('label')} ({n['source_file']}:{n.get('source_location','')})" for n in top],
        })
    out.sort(key=lambda c: -c["nodes"])
    return out


def cross_edges(nodes: dict[str, dict], links: list[dict]) -> list[tuple[tuple, int, str]]:
    pairs: Counter = Counter()
    example: dict = {}
    for l in links:
        s, t = nodes.get(l.get("source")), nodes.get(l.get("target"))
        if not s or not t or not is_internal(s) or not is_internal(t):
            continue
        a, b = s.get("community"), t.get("community")
        if a == b or a is None or b is None:
            continue
        key = (min(a, b), max(a, b))
        pairs[key] += 1
        example.setdefault(key, f"{s.get('label')} --{l.get('relation','')}--> {t.get('label')}")
    return [(k, c, example[k]) for k, c in pairs.most_common(12)]


def god_nodes(nodes: dict[str, dict], adj: dict[str, set[str]], top: int) -> list[dict]:
    ranked = sorted((n for n in nodes.values() if is_internal(n)), key=lambda n: -len(adj.get(n["id"], ())))[:top]
    return [{"label": n.get("label"), "file": n["source_file"], "loc": n.get("source_location", ""),
             "community": n.get("community"), "degree": len(adj.get(n["id"], ()))} for n in ranked]


def tokens(text: str) -> list[str]:
    out = []
    for word in re.findall(r"[^\W\d_]+", text or "", re.UNICODE):
        parts = re.findall(r"[A-Z]+(?=[A-Z][a-z])|[A-Z]?[a-z]+|[A-Z]+", word) or [word]
        for p in parts:
            t = p.lower()
            if len(t) >= 3 and t not in STOP:
                out.append(t)
    return sorted(set(out))


def md_summary(graph: Path, top: int, max_files: int) -> str:
    _, nodes, adj, links = load(graph)
    labels = load_labels(graph)
    comms = communities(nodes, adj, labels)
    internal = sum(1 for n in nodes.values() if is_internal(n))
    lines = [f"# graphify map: {graph}", "",
             f"{internal} internal nodes · {len(nodes) - internal} external · {len(links)} edges · {len(comms)} communities", "",
             "## Communities (candidate node boundaries: what gets read together)", ""]
    for c in comms:
        files = c["files"][:max_files]
        more = f" … +{len(c['files']) - max_files} more" if len(c["files"]) > max_files else ""
        lines += [f"### {c['name']} (id {c['id']}) — {c['nodes']} nodes, {len(c['files'])} files",
                  "- files: " + ", ".join(f"`{f}`" for f in files) + more,
                  "- hubs: " + "; ".join(c["top"]), ""]
    lines += ["## God nodes (hubs many nodes must read; give them to one node or keep them read-only)", ""]
    lines += [f"- {g['label']} — `{g['file']}:{g['loc']}` (community {g['community']}, degree {g['degree']})" for g in god_nodes(nodes, adj, top)]
    lines += ["", "## Cross-community edges (likely data dependencies between task nodes)", ""]
    ce = cross_edges(nodes, links)
    lines += [f"- {a} ↔ {b}: {n} edges, e.g. {ex}" for (a, b), n, ex in ce] or ["- none: communities are independent"]
    return "\n".join(lines) + "\n"


def focus(graph: Path, query: str, depth: int, max_files: int) -> dict:
    _, nodes, adj, links = load(graph)
    labels = load_labels(graph)
    toks = tokens(query)
    scored = []
    for n in nodes.values():
        if not is_internal(n):
            continue
        label = (n.get("norm_label") or n.get("label") or "").lower()
        path = n["source_file"].lower()
        score = sum(2 for t in toks if t in label) + sum(1 for t in toks if t in path)
        if score:
            scored.append((score, len(adj.get(n["id"], ())), n["id"]))
    scored.sort(reverse=True)
    seeds = [nid for _, _, nid in scored[:8]]
    touched = set(seeds)
    frontier = set(seeds)
    for _ in range(depth):
        nxt = set()
        for nid in frontier:
            for m in adj.get(nid, ()):
                if m not in touched and is_internal(nodes[m]):
                    nxt.add(m)
        touched |= nxt
        frontier = nxt
    by_comm: dict = defaultdict(lambda: {"files": Counter(), "nodes": 0, "seeds": 0, "members": []})
    for nid in touched:
        n = nodes[nid]
        c = by_comm[n.get("community")]
        c["files"][n["source_file"]] += 1
        c["nodes"] += 1
        c["seeds"] += nid in seeds
        c["members"].append(n)
    # A file is "shared" when nodes IN it belong to more than one touched community
    # (membership), or when it is connected to more than one touched community (use).
    touched_comms = set(by_comm)
    file_member: dict = defaultdict(set)
    file_connected: dict = defaultdict(set)
    for nid in touched:
        n = nodes[nid]
        file_member[n["source_file"]].add(n.get("community"))
        for m in adj.get(nid, ()):
            if m in touched and nodes[m].get("community") != n.get("community"):
                file_connected[n["source_file"]].add(nodes[m].get("community"))
    shared = []
    for f in sorted(set(file_member) | set(file_connected)):
        members = sorted(x for x in file_member.get(f, ()) if x is not None)
        connected = sorted(x for x in file_connected.get(f, ()) if x is not None and x in touched_comms)
        if len(members) > 1 or len(connected) > 1:
            shared.append({"file": f, "member_of": members, "used_by": connected})
    shared.sort(key=lambda x: -(len(x["member_of"]) + len(x["used_by"])))
    comms = []
    for cid, c in sorted(by_comm.items(), key=lambda kv: (-kv[1]["seeds"], -kv[1]["nodes"])):
        comms.append({"id": cid, "name": cname(cid, labels, c["members"]), "nodes": c["nodes"], "seeds": c["seeds"],
                      "files": [f for f, _ in c["files"].most_common(max_files)]})
    # Draft split: greedily merge candidates whose file sets overlap (Jaccard >= 0.5) or nest,
    # because "prefer fewer nodes when results are comparable" and a tiny corpus fragments
    # into one community per file.
    groups: list[dict] = []
    for c in comms:
        fs = set(c["files"])
        for g in groups:
            gs = g["files"]
            inter = len(fs & gs)
            if fs and (inter / len(fs | gs) >= 0.5 or fs <= gs or gs <= fs):
                g["files"] |= fs
                g["communities"].append(c["id"])
                g["seeds"] += c["seeds"]
                break
        else:
            groups.append({"files": set(fs), "communities": [c["id"]], "seeds": c["seeds"]})
    draft = [{"candidate": i + 1, "communities": g["communities"], "reads": sorted(g["files"]), "seeds": g["seeds"]} for i, g in enumerate(groups)]
    return {"query": query, "tokens": toks, "seeds": [f"{nodes[s].get('label')} ({nodes[s]['source_file']})" for s in seeds],
            "touched_nodes": len(touched), "communities": comms, "shared_files": shared, "draft_split": draft}


def md_focus(res: dict) -> str:
    lines = [f"# focus: {res['query']}", "", f"tokens matched against graph vocabulary: {', '.join(res['tokens']) or '(none)'}", ""]
    if not res["seeds"]:
        lines += ["No node label or file path matches these words. graphify matches identifiers, not task words:",
                  "take symbol names from `summary` (god nodes, hubs) and use `graphify explain \"<symbol>\"` or",
                  "`graphify affected \"<symbol>\"` (both take --graph), or split from the per-community file lists in `summary`.", ""]
        return "\n".join(lines)
    lines += ["seed nodes: " + "; ".join(res["seeds"]), f"touched: {res['touched_nodes']} nodes in {len(res['communities'])} communities", "",
              "## Communities the task touches (ranked by seeds)", ""]
    for c in res["communities"]:
        lines += [f"- {c['name']} (community {c['id']}): {c['nodes']} touched nodes, {c['seeds']} seeds; files: " + ", ".join(f"`{f}`" for f in c["files"])]
    lines += ["", "## Draft split (communities merged when their files overlap; one candidate node each)", ""]
    for d in res["draft_split"]:
        lines += [f"### candidate node {d['candidate']} (communities {d['communities']}, {d['seeds']} seeds)",
                  "- reads: " + ", ".join(f"`{f}`" for f in d["reads"]), ""]
    lines += ["## Shared files (belong to, or are used by, more than one touched community: let ONE node own each; others read only)", ""]
    lines += [f"- `{h['file']}`: nodes in communities {h['member_of']}; used by communities {h['used_by'] or '[]'}" for h in res["shared_files"]] or ["- none"]
    lines += ["", "Rules: an edge only where a node consumes another node's output; merge two candidates when their reads overlap heavily;",
              "a deterministic step (inventory, merge, count, compare) is a code node, whatever community it touches.", ""]
    return "\n".join(lines)


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("summary"); s.add_argument("--graph", default="graphify-out/graph.json"); s.add_argument("--top", type=int, default=12)
    s.add_argument("--max-files", type=int, default=25); s.add_argument("--json", action="store_true")
    f = sub.add_parser("focus"); f.add_argument("--graph", default="graphify-out/graph.json"); f.add_argument("--query", required=True)
    f.add_argument("--depth", type=int, default=2); f.add_argument("--max-files", type=int, default=25); f.add_argument("--json", action="store_true")
    c = sub.add_parser("files"); c.add_argument("--graph", default="graphify-out/graph.json"); c.add_argument("--community", required=True)
    a = p.parse_args()
    graph = Path(a.graph)
    if not graph.is_file():
        print(f"error: {graph} not found. Build it first: graphify extract <sources> --code-only  (or /graphify <sources> for docs)", file=sys.stderr)
        sys.exit(1)
    if a.cmd == "summary":
        if a.json:
            _, nodes, adj, links = load(graph)
            print(json.dumps({"communities": communities(nodes, adj, load_labels(graph)), "god_nodes": god_nodes(nodes, adj, a.top),
                              "cross_edges": [{"pair": list(k), "edges": n, "example": ex} for k, n, ex in cross_edges(nodes, links)]}, indent=2))
        else:
            print(md_summary(graph, a.top, a.max_files))
    elif a.cmd == "focus":
        res = focus(graph, a.query, a.depth, a.max_files)
        print(json.dumps(res, indent=2) if a.json else md_focus(res))
    else:
        _, nodes, adj, _ = load(graph)
        want = None if a.community == "unassigned" else int(a.community)
        files = sorted({n["source_file"] for n in nodes.values() if is_internal(n) and n.get("community") == want})
        print("\n".join(files) or f"no files in community {a.community}")


if __name__ == "__main__":
    main()
