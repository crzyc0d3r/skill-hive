# Architecture

Skill Hive is one Rust library (`hive-core`) with two front ends: the `hive` CLI and a Tauri
desktop app whose React UI talks to the same facade through typed commands. Nothing in the UI
reaches the file system or the network directly; every operation is a core method.

```
┌──────────────┐   invoke / events   ┌──────────────┐
│ React UI     │ ◄─────────────────► │ Tauri shell  │
│ src/         │                     │ src-tauri/   │
└──────────────┘                     └──────┬───────┘
                                            │ Hive facade
┌──────────────┐                     ┌──────▼───────┐
│ hive CLI     │ ───────────────────►│ hive-core    │
│ crates/cli   │                     │ crates/core  │
└──────────────┘                     └──────┬───────┘
                                            │
            SQLite (hive.db) · ~/.skill-hive/library · git · agent dirs · providers
```

## Core modules

| Module | Responsibility |
| --- | --- |
| `model` | Every type that crosses the boundary (serde, `ts-rs` exported). `cargo test -p hive-core` regenerates `src/lib/api/bindings`. |
| `paths` | The Hive home layout (`HIVE_HOME` or `~/.skill-hive`). Agent directories resolve against the user's home, or `HIVE_TARGET_HOME` when set (tests, sandboxes). |
| `db` | SQLite with WAL and FTS5. Items, tags, deployments, projects, custom targets, loadouts, providers, runs, case results, settings, cache, audit. Rebuildable from disk. |
| `skill` | Parsing and validation of `SKILL.md`, subagent and command files (Agent Skills rules: name regex and length, description length, body, unknown keys, parent-path references). |
| `hash` | Content hashes over sorted relative paths + bytes, so copies compare equal anywhere. |
| `library` | Import (copy + sidecar + index), replace, refresh, create, remove, files, reindex, orphan detection. Names are unique per kind; slugs get suffixes. |
| `targets` | The table of agent tools and their user/project directories; overrides and custom targets. |
| `discovery` | Depth-limited scans of agent directories, projects and extra folders; candidates classified as new / in library / modified / managed / invalid. |
| `sources` | `parse_spec` for `owner/repo`, GitHub tree/blob URLs, git URLs, archive URLs and paths; git-CLI clone cache; GitHub API (search, tarball fallback); safe zip/tar extraction; curated catalog; previews, imports, update checks and updates. |
| `deploy` | Link (symlink/junction) or copy into a target directory; install state (linked, in sync, stale, modified, unmanaged, broken); undeploy; resync; what a directory contains. |
| `conflicts` | Analysis of a set that would share one agent: name collisions, duplicate content, description overlap (log-TF cosine, IDF from the whole library when it is large enough), trigger overlap, tool overlap, contradictory always/never directives, and the description budget. |
| `recommend` | Task → ranked items (FTS5 BM25 + name/tag/trigger/description features + measured effectiveness), pairwise conflicts, greedy conflict-free in-budget suggested set, optional model re-ranking. |
| `loadouts` | Named sets; plan (adds/removes/keeps/unmanaged, resulting conflicts and budget) and apply (exclusive or additive). |
| `providers` | `Provider` trait with `complete` and `execute`; Claude Code CLI, Codex CLI, Anthropic Messages API, OpenAI-compatible chat completions, and a scripted offline provider; registry, probes, secrets. `trace` builds an `ExecutionTrace` (tool calls by category, files, thinking, token split, cache, timeline) from a CLI agent's event stream. |
| `evals` | Suites (`evals/evals.json`, `checks.json`, `trigger.json`), deterministic checks, LLM grader, trigger evaluation, the run engine (sandboxes, arms, repeats, concurrency, cancellation, events) and analytics, including per-arm activity statistics. See `docs/evaluations.md`. |
| `hive` | The facade: owns the database, paths, secrets, target resolver and event bus. |

## Data on disk

```
~/.skill-hive/
  hive.db                SQLite index (PRAGMA user_version = 1)
  library/skills/<slug>/ one directory per skill (deployed by symlink or copy)
  library/agents/*.md    subagent definitions
  library/commands/*.md  slash commands
  library/meta/<id>.json provenance sidecars (id, slug, source, tags) → reindex
  cache/git/<key>/       shallow clones, refreshed in place
  cache/archives/<key>/  extracted downloads
  workspaces/<skill>/<run-id>/
    skill-snapshot/      the skill exactly as tested
    cases/<case>/<arm>/<n>/work/   sandbox: inputs, outputs, .hive/transcript.jsonl
    benchmark.json, benchmark.md
  secrets.json           only without an OS keychain (0600)
```

## Deployment semantics

- The library is the source of truth. A **link** deployment points the agent at the library
  directory, so library updates are visible immediately. A **copy** records the hash it was made
  from; later the state is *stale* (library moved on) or *modified* (the copy was edited) and
  can be re-synced.
- Install names use the item's `name` (`<name>/` for skills, `<name>.md` for agents and
  commands), which is why names are unique per kind in the library.
- Unmanaged assets (put there by the user or another tool) are never touched unless the user
  asks (`force` on deploy, explicit remove, or import to the library first).

## Conflicts and budgets

Agents choose a skill from the list of names and descriptions in their prompt. Two things break
that silently: near-duplicate descriptions (the agent picks at random) and lists longer than the
agent's budget (Claude Code truncates skill listings at about 15 000 characters). The analyser
runs on installed sets, loadout plans and recommendation results, and every target status shows
its context cost against the budget.

## IPC contract

Rust types are the contract. `ts-rs` writes one `.ts` file per type into
`src/lib/api/bindings/` (camelCase fields, enums as string unions, tagged unions for
`CheckKind`, `ArmKind` and `HiveEvent`). `src/lib/api/client.ts` lists every command; outside
Tauri the same calls go to `src/lib/api/mock.ts`. Core events (`HiveEvent`) are broadcast
through the shell as `hive://event` and drive React Query invalidation and live run progress.

## Testing

- `cargo test -p hive-core`: unit and integration tests for every module (temporary homes,
  local git repositories, scripted providers).
- `cargo test -p hive-cli`: the binary end to end, including a scripted evaluation run.
- `npm test`: component and hook tests against the mock backend.
- `scripts/screenshots.mjs`: Playwright captures of every screen in both themes.
