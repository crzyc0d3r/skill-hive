# Skill Hive

One library for every agent skill, subagent and slash command you use, with a way to find the
right ones for a job, deploy them to each agent tool without conflicts, and measure whether a
skill's instructions actually help.

Skill Hive is a cross-platform desktop app (Tauri 2 + React) and a CLI (`hive`) that share one
Rust core. It works with the [Agent Skills](https://agentskills.io) format used by Claude Code,
Codex, Cursor, Gemini CLI, Copilot and others.

## What it does

| Goal | How |
| --- | --- |
| **Collect** skills and agents from this machine or online | Scans every known agent directory (`~/.claude/skills`, `~/.codex/skills`, …) and registered projects; imports from `owner/repo`, GitHub tree/blob URLs, any git URL, archive URLs or local folders; ships a curated catalog of well-known collections and searches GitHub. |
| **Library** to choose from | Items are copied into `~/.skill-hive/library` with provenance sidecars and indexed in SQLite with full-text search, tags, validation findings, deployments and measured effectiveness. Update checks compare recorded revisions with the source. |
| **Cross-platform desktop app** | Tauri 2 shell, React 19 UI, dark/light themes, command palette, keyboard shortcuts; installers for macOS, Windows and Linux from the release workflow. |
| **Right skills for the job, without conflicts** | *Find* ranks the library for a task (BM25 + lexical features + measured effectiveness, optional model re-ranking) and composes a suggested set that has no high-severity conflicts and fits the agent's description budget. *Agents* shows what each tool has installed, its context cost and conflicts (name collisions, duplicate content, overlapping descriptions and triggers, contradictory directives, budget overruns). *Loadouts* apply curated sets exclusively or additively, with a dry-run plan. |
| **Measure skill effectiveness** | Evaluation suites (skill-creator compatible `evals/evals.json` + checks + trigger cases) run each prompt in arms — with the skill, without it, or against a variant — with repeats, deterministic checks and an LLM grader that must cite evidence. The benchmark reports pass rate, tokens, duration and tool calls per arm with mean ± sd, the delta with a bootstrap 95 % CI and p-value, a verdict, a per-expectation matrix (always passes / only with skill / flaky …), trigger precision/recall and analyst notes. History tracks each version of the skill. |

## Quick start

Prerequisites: Rust (stable), Node 22, and on Linux the Tauri build dependencies
(`libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev patchelf libxdo-dev libssl-dev`).

```bash
npm install
npm run app:dev          # desktop app with hot reload
cargo run -p hive-cli -- --help
```

The UI also runs in a plain browser against an in-memory mock backend (`npm run dev`), which is
what the unit tests and screenshots use.

### CLI in two minutes

```bash
hive scan                                  # what is already installed on this machine
hive scan --import                         # bring it all into the library
hive add anthropics/skills --tag official  # import a collection from GitHub
hive add obra/superpowers
hive find pull the tables out of a pdf invoice and build a spreadsheet --target claude-code
hive deploy pdf xlsx --to claude-code      # symlinks into ~/.claude/skills
hive targets claude-code                   # installed set, context cost, conflicts
hive loadout create "Docs" --item pdf --item docx --item pptx
hive loadout plan docs --to claude-code --exclusive
hive provider detect                       # register claude / codex CLIs as providers
hive eval suite pdf --draft --write        # starter suite inside the skill
hive eval run pdf --repeats 2 --trigger    # with_skill vs without_skill
hive eval report <run-id>
```

Every command accepts `--json`.

## Layout

```
crates/hive-core/   library, discovery, sources, deployment, conflicts, recommender,
                    providers, evaluations; the Hive facade; ts-rs exported model
crates/hive-cli/    the `hive` binary (clap) with end-to-end tests
src-tauri/          Tauri 2 shell: async commands over the facade, event bridge
src/                React 19 + TypeScript UI (Vite, Tailwind 4, Radix, TanStack Query)
  lib/api/          typed client, generated bindings, mock backend, hooks
  features/         home, library, discover, find, agents, loadouts, evals, settings
docs/               architecture, evaluations
```

Data lives under `~/.skill-hive` (override with `HIVE_HOME`): `hive.db`, `library/`,
`cache/`, `workspaces/` (evaluation sandboxes and reports), `logs/`. Secrets go to the OS
keychain, or a 0600 file when no keychain is available.

## Supported agent targets

Claude Code, Codex CLI, Cursor, Gemini CLI, GitHub Copilot, Windsurf, OpenCode, Amp, Goose,
Kiro, Cline, Roo Code, Antigravity, the shared `.agents/skills` convention, OpenClaw and Hermes.
Directory conventions are defaults: any target's skills folder can be overridden and custom
targets can be added in Settings.

## Development

```bash
cargo test --workspace                     # Rust tests (core + CLI end-to-end)
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck && npm run lint && npm test && npm run build
npm run bindings && node scripts/gen-bindings-index.mjs   # regenerate TypeScript types from Rust
npm run build && node scripts/screenshots.mjs             # screenshots of every screen (mock data)
```

CI runs the Rust suite on Linux, macOS and Windows, the web checks, and a Linux desktop
bundle. Pushing a `v*` tag builds installers for every platform and the CLI binaries into a
draft release.

## Status

- Core, CLI, sources, deployment, conflict analysis, recommendations, loadouts and the
  evaluation engine are covered by automated tests (offline, using a scripted provider).
- The Claude Code, Codex, Anthropic API and OpenAI-compatible providers parse real payload
  shapes and are unit-tested against them, but have not been exercised against live services
  from the environment this was built in. Treat the first live run as a smoke test.
- Agent directory conventions for the less common tools are best-effort defaults.

## License

MIT. See `LICENSE`.
