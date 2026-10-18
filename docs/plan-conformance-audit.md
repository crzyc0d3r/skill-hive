# Plan conformance audit

Scored against `main` at `695bb46` (the parity port). This branch does not add an effectiveness engine. An earlier commit on the branch did, and it has been removed so the test design can be reviewed before anything is built.

A row is **real** only when the code path that satisfies it is the one a person can run, and, where a model is involved, that path uses the HTTP adapter's wire protocol. A library test that talks to a local server speaking the Anthropic Messages or OpenAI chat-completions protocol counts as real for that adapter. It does not make the screen or the CLI command real.

A row is **stand-in** when the path that actually runs is `ScriptedProvider`, a frozen fixture, demo-mode sample rows, a fake `claude` binary, or a localhost POST whose body is not the vendor protocol. A row is **missing** when the behavior is not implemented. Stand-in rows are not done.

## Counts

| status | rows |
|---|---|
| real | 18 |
| stand-in | 29 |
| missing | 16 |

18 of 63 rows are on a real path. 45 are not done.

## What a real measurement still needs

These are not implemented as product runs. They are the list to design before building.

- A suite command that calls `AnthropicProvider` or `OpenAiProvider`, not `ScriptedProvider` and not a custom JSON POST.
- `with_skill` and `without_skill` in that same run, with `grading.json` and `benchmark.json` produced from the model's transcript.
- Trigger evaluation that asks a model (the API runner or `claude -p`), not a pre-recorded stream.
- Blind comparison that sends P12 to a provider and hides the side names until a person records a verdict.
- An improve loop that re-runs after a rewrite and writes `history.json` from those rates, plus regression rules R1–R6 against a pinned baseline.
- The generate wizard calling a provider for the interview and the draft. `save_generated_draft` reviews a local envelope.
- Results, compare, history, and export screens reading that run. Today the screens read demo rows or an empty state.
- The library-interference additions below: trigger runs in a library context, a conflict matrix, a machine-to-machine interference report, per-model pass bars, and an optional gold set for the grader. None of that is in PLAN.md, and none of it is on `main`.

A live call to `https://api.anthropic.com` or `https://api.openai.com` still needs that vendor's key. The reference `claude -p` measurement needs the Claude Code CLI. Live skills.sh downloads and GitHub device-flow completion need network access and a user. None of those are reasons to treat the stand-ins below as done.

## Building blocks already on main

These exist and are not the product measurement. A later build should call them rather than rewrite them.

| piece | where | what it actually does |
|---|---|---|
| Anthropic and OpenAI adapters | `crates/hive-core/src/providers/http.rs` | `POST {base}/v1/messages` with `x-api-key` and `anthropic-version`, and `POST {base}/chat/completions`. `providers/tests.rs` answers both on a local server that speaks the wire protocol. |
| One live engine cell | `crates/hive-core/src/testing/tests.rs` `local_http_endpoint_can_drive_one_live_cell` | `run_plan` plus `AnthropicProvider` against that kind of local server, for a single cell. |
| Engine | `crates/hive-core/src/testing/engine.rs` | Sandboxes, tool loop, `transcript.jsonl`, `grading.json`, `benchmark.json` via `aggregate.rs`, SQLite rows. The CLI calls it with `ScriptedProvider`. |
| Suite files | `crates/hive-core/src/testing/suite.rs` `save_suite` | Writes `evals/evals.json`, `trigger.json`, and `checks.json`. |
| Checks | `crates/hive-core/src/testing/checks.rs` `run_check` | Local file and transcript predicates. A bound expectation is not sent to the grader. |
| Trigger judge | `crates/hive-core/src/testing/trigger.rs` `run_trigger_set` | Scores streams that the caller already recorded. |
| History and regression | `crates/hive-core/src/analytics/history.rs`, `regression.rs` | `write_history` and rules R1–R6. The app's Baseline button does not call `evaluate`. |
| Validator | `crates/hive-core/src/authoring/validator.rs` | The save-time skill check. |

## Part 2 §3 — skill-creator model

| id | requirement | status | evidence |
|---|---|---|---|
| 3.1-layout | Iteration workspace with `with_skill` and `without_skill`, outputs, timing, grading, and a transcript | stand-in | `engine.rs` `run_plan` can write that layout. The CLI command that runs a suite is `test_run_scripted` in `crates/hive-cli/src/eval.rs`, which passes `ScriptedProvider::new("offline")`. |
| 3.1-names | Descriptive eval directory names, `eval_metadata.json`, `skill-snapshot/`, and viewer `feedback.json` | missing | No product path writes those names or files. |
| 3.2 | `evals/evals.json` and `trigger.json` in the skill-creator schema | real | `save_suite` in `suite.rs`. `hive-cli` `test_save_suite` calls it. |
| 3.3 | `grading.json` whose pass/fail comes from a model looking at the transcript | stand-in | The product run grades through `ScriptedProvider`. The one HTTP cell is a library test, not the CLI or the app. |
| 3.4-benchmark | `benchmark.json` for `with_skill` versus `without_skill` from a real pair of runs | stand-in | `aggregate_benchmark` runs at the end of `run_plan`. The CLI fills it with the scripted provider. |
| 3.4-old | `old_skill` from a snapshot taken when improving an existing skill | stand-in | `engine.rs` copies `skill-snapshot` when that directory is already there. Nothing in the app takes that snapshot before a run. |
| 3.5-streams | Trigger judge: first tool call, rate, and threshold | stand-in | `run_trigger_set` scores `TriggerQuery.streams` supplied by the caller. It does not ask a model. |
| 3.5-cli | `claude -p` trigger runner with a temporary command file deleted on every exit | missing | `LocalCli` in `providers/cli_agent.rs` can spawn a program. The suite does not call it, and the tests point it at a stand-in binary. |
| 3.6-blind | Blind A/B held unlabeled until a person records a verdict | stand-in | `CompareView.tsx` shows demo sentences ("Findings list the seeded vulnerability"). `store_verdict` in `analytics/compare.rs` compares the fixed strings `left` and `right`. |
| 3.6-model | P12 comparator and post-hoc analysis from a model | missing | `generation/templates.rs` holds the P12 text. No command sends it to `AnthropicProvider` or `OpenAiProvider`. |
| 3.7-history | `history.json` with `won`, `lost`, or `tie` from a measured with-versus-without rate | stand-in | `write_history_baseline` in `src-tauri/src/commands.rs` writes the fixed chain (0.92, 0.97, 0.97) when `test-runs/last.json` has zero cells. |
| 3.7-loop | Improve loop: rewrite, re-run, train/test description search, apply only on click | missing | `apply_optimized_description` writes the typed string through `analytics/optimize.rs`. It does not call a model or re-run a suite. |
| 3.8 | `quick_validate.py` rules, reported together, on save | real | `authoring/validator.rs` `validate_skill` is the save path. The Python script is not bundled. |
| 3.9 | Claude Code, sequential, and static-viewer modes | missing | No mode switch and no static viewer generated from a run. |

## Part 3 — authoring, generation, testing, workflows

| id | requirement | status | evidence |
|---|---|---|---|
| A.1-editor | Skill editor with frontmatter, body, problems, preview, versions, and the 500-line gauge | real | `src/features/skills/SkillEditor.tsx` and `validator.rs`. |
| A.1-tree | File tree, executable bit, and the `allowed-tools` inserter | missing | The editor is one Markdown buffer. |
| A.2 | Agent editor with fields, problems, preview, export, and versions | real | `src/features/agents/AgentEditor.tsx`. |
| A.3 | Draft snapshot and restore | real | `snapshot_skill_draft` and `restore_skill_draft` in `src-tauri/src/commands.rs`. |
| B.1 | Generate wizard interview and draft that call a provider | stand-in | `save_generated_draft` calls `GenerationSession::run_review` on `skill_envelope`. `generation/session.rs` says replies come from recorded fixtures. The CLI live path in `hive-cli/src/generate.rs` POSTs `{"task","name","kind"}` to a localhost URL and refuses `api.anthropic.com`. |
| B.2 | Templates P1–P13 sent to the selected endpoint | stand-in | The strings live in `generation/templates.rs`. The app path does not send them. |
| C.1-files | `evals.json`, `trigger.json`, and `checks.json` | real | `save_suite`. |
| C.1-ui | Cases, trigger set, and checks editors on the suite screen | missing | `SuiteView.tsx` shows two sentences, Optimize, Apply, and a link to `/runs/run-fixture`. |
| C.2-trigger | Trigger tests with the P9 prompt, three runs, and a 0.5 threshold against a model | stand-in | Same recorded-stream judge as 3.5. The product suite has no trigger run. |
| C.2-expect | Expectation test in a sandbox with the skill injected only for `with_skill` | stand-in | `engine.rs` injects the skill for `with_skill` only. The product caller is `ScriptedProvider`. |
| C.2-checks | Deterministic checks that the grader cannot overturn | real | `checks.rs` `run_check`. `engine.rs` runs them and records `checks.json`. The CLI's saved suite includes a `file_exists` check. |
| C.2-ab | Blind version-versus-version comparison through P12 | missing | The compare screen does not send outputs to a model. |
| C.3-matrix | Configurations, endpoints, models, repetitions, estimate, presets, and progress events | missing | `Plan` is one model and one repetition. The suite screen has no matrix. |
| C.3-budget | Budget pause with Continue, Stop and grade, or Cancel | stand-in | `Plan.budget_usd` can stop `run_plan`. The Budget button in `ResultsView.tsx` only sets a local paused flag. |
| C.4-results | Results summary, heatmap, and timeline from a measured run | stand-in | Demo mode renders `FIXTURE_CELLS` in `results/model.ts`. Otherwise the screen parses `get_recent_log_excerpt` or stays empty. |
| C.4-rest | Analyst notes, trigger panel, and the cell detail sheet | missing | Not on `ResultsView.tsx`. |
| C.5-history | History screen fed by a measured `history.json` | stand-in | Demo mode renders `HISTORY_ROWS`. Live mode stays empty until Baseline, which writes the canned chain described above. |
| C.5-rules | R1–R6 evaluated when a product run finishes | stand-in | `regression.rs` `evaluate` is unit-tested. `write_history_baseline` does not call it. `analytics/workflow.rs` runs it on frozen fixtures. |
| C.6 | Export of benchmark, CSV, JUnit, and HTML from the graded cells | stand-in | `hive-cli` `test_export` builds one `ExportCell` from the `cells` and `passed` counts in `last.json`. The results screen has no export of a run. |
| D.1 | Create a skill and prove it by running the suite | stand-in | `analytics/workflow.rs` `demonstrate_d1` says every model reply is a frozen fixture. |
| D.2 | Import a skill and benchmark it | stand-in | `demonstrate_d2` is the same fixture recording. Import does not start `run_plan`. |
| D.3 | Generate an agent and test it on two endpoints | missing | No product path runs one subject against two endpoints. `demonstrate_d3` is a fixture. |

## Part 1 §4 — provider layer

| id | requirement | status | evidence |
|---|---|---|---|
| 4.1 | Provider-agnostic chat interface used by the engine | real | `ChatProvider` in `providers/types.rs`. `run_plan` takes `&dyn ChatProvider`. |
| 4.2-adapter | Anthropic Messages adapter, injectable base URL, SSE | real | `AnthropicProvider` in `providers/http.rs`. `anthropic_openai_and_ollama_answer_on_localhost` in `providers/tests.rs`. |
| 4.2-product | The app or the CLI suite uses that adapter | missing | `test_run` uses `ScriptedProvider` until a localhost base URL is set, then `post_local` sends `{"skill","mode"}`. `is_local_http` refuses `api.anthropic.com`. |
| 4.3-adapter | OpenAI-compatible adapter, Bearer or Azure `api-key` | real | `OpenAiProvider` in `http.rs`, same localhost test. |
| 4.3-product | A suite or wizard call uses that adapter | missing | No app command constructs `OpenAiProvider`. |
| 4.4 | Local agent CLI executor as the measurement | stand-in | `cli_agent.rs` spawns the configured program and parses stream-json. Tests and `probe.rs` use a stand-in binary. The suite command does not select it. |
| 4.5-secrets | Secrets scrubbed from transcripts | real | `Transcript::push` in `engine.rs` scrubs the secret. |
| 4.5-retry | Retries, rate limits, and cost on the calls a suite makes | stand-in | `providers/middleware.rs` implements them and has tests. `test_run_scripted` does not wrap the provider in that middleware. |
| 4.6-probe | Probe a provider and report the stages | real | `probe_provider` in `providers/probe.rs`, tested on localhost. |
| 4.6-screen | The endpoints screen runs that probe and stores the key in the OS keychain | stand-in | `EndpointsView.tsx` lists the stage names and keeps the secret in the page store. `providers_probe_live` POSTs `{"id","model"}` and refuses vendor hosts. |

## Part 1 §5 — test execution engine

| id | requirement | status | evidence |
|---|---|---|---|
| 5.1 | A product run expands a plan and persists `runs`, `run_cases`, and `usage_ledger` | stand-in | `run_plan` writes those tables. The CLI calls it with `ScriptedProvider` and `kind: "scripted"`. |
| 5.2 | Isolated working directories, removed after the cell | real | The sandbox is per cell. `testing/tests.rs` covers deletion. |
| 5.3 | Tool loop jailed to the cell (`write_file`, `read_file`, `bash`, `list_dir`) | real | `engine.rs` tool loop. On the product path the model deciding the tool calls is scripted. |
| 5.4 | Transcript and tool-call metrics | real | `transcript.jsonl` and `outputs/metrics.json` are written by `run_plan`. |
| 5.5 | Grading, aggregation, cancellation, and persistence of a live run | stand-in | Aggregation and the cancellation token are in `engine.rs`. The product grade comes from `ScriptedProvider`. Cancel is covered by `ScriptedProvider::new("cancel")`. |
| 5.6 | Seeded runs and record/replay | real | `Plan.seed` is part of `run_plan`. `middleware.rs` record-then-replay is tested. The scripted CLI run uses seed 4 and does not go through that middleware. |

## Part 4 §5.3 — new screens

| id | requirement | status | evidence |
|---|---|---|---|
| 5.3-agents | Agents library with search, tags, import, and the description budget | stand-in | `AgentsView.tsx` lists agents. The budget meter and import sheet from the spec are not on the screen. Demo mode seeds cards. |
| 5.3-skill-editor | Skill editor | real | `/skills/:id/edit` renders `SkillEditor`. The file tree from the spec is still missing (A.1-tree). |
| 5.3-agent-editor | Agent editor | real | `/agents/:id/edit` renders `AgentEditor`. |
| 5.3-generate | Five-step wizard that calls a provider | stand-in | `GenerateWizard.tsx` has the five steps. Accept all defaults calls `save_generated_draft`, which does not call a provider. The interview step stays empty until that click. |
| 5.3-suite | Suite screen that starts a measured run | missing | Run suite is a link to `/runs/run-fixture`. |
| 5.3-matrix | Run matrix and live progress grid | missing | No matrix panel and no `test-run-progress` events. |
| 5.3-results | Results screen of a measured run, including export | stand-in | `ResultsView.tsx` uses demo cells or a log excerpt. There is no export of graded cells. |
| 5.3-cell | Cell sheet with transcript, timeline, outputs, grading, checks, and feedback | missing | Not implemented. |
| 5.3-compare | Compare screen, blind until the verdict, then unblinded from the run | stand-in | Demo cards, then `record_compare_verdict`. Live mode says "No comparison yet." until that command returns. |
| 5.3-history | History with the measured chain, a trend, and regression alerts | stand-in | Demo rows or the canned baseline chain. No trend and no R1–R6 alerts. |
| 5.3-endpoints | Endpoint catalog with a live probe, prices, and a keychain secret | stand-in | `EndpointsView.tsx` plus the page store. See 4.6-screen. |

## Additions beyond PLAN.md

These five requirements are not in PLAN.md. They are not in the 63 rows above. On `main` every one of them is absent: `run_trigger_set` scores streams the caller already recorded, and it does not know which other skills were installed. Nothing exports a library snapshot, nothing bisects a library, and nothing stores a hand-labelled gold set. This section is the proposed design. It is not built.

| id | addition | on main |
|---|---|---|
| L1 | Trigger runs in a library context, with an ordered activation list per adapter and the four outcomes | not built |
| L2 | Conflict matrix of which skills steal each other's queries | not built |
| L3 | Library Interference Report: snapshot, overlap, re-run, bisect, rewrite re-test, install warning | not built |
| L4 | Per-model results with a configurable pass bar, flagging strong-model-only descriptions | not built |
| L5 | Optional hand-labelled gold set that calibrates the grader | not built |

### L1 — Library context

A trigger run names one candidate skill and a distractor set. The distractors are the user's enabled installed skills (`skills` rows: `name`, `description`, `content_hash`, `source_type`, `source_ref`, `source_revision`) plus generated lookalikes. A lookalike is a skill whose description shares the candidate's domain and whose trigger phrases are near misses. Lookalikes are written into the run directory, not into the user's library.

The run directory is a fresh project that contains only that context, so the user's real global skills do not leak in. The result for each query records the ordered list of skills that activated, the adapter that observed them, and one outcome:

| outcome | rule |
|---|---|
| fired alone | the candidate is the only skill in the activation list |
| fired with others | the candidate appears, and at least one other skill appears |
| hijacked | the candidate does not appear, and another skill does. The culprit is the first other skill in the list |
| none | the activation list is empty |

The same queries run on each selected platform adapter. The ordered list is parsed from that adapter, not asked of a second model.

- Claude Code: `claude -p --output-format stream-json`. Walk `tool_use` blocks in order. `Skill` counts when its input names a skill in the context. `Read` counts when the path lands inside that skill's directory. This is the same first-tool idea as `judge_stream` in `crates/hive-core/src/testing/trigger.rs`, extended from one boolean to the full ordered list.
- Codex and Cursor-style agents: there is no `Skill` tool. A skill counts when a file read resolves to its `SKILL.md` or to a file under its directory. The adapter name is stored on the row.
- Raw API models: the emulated runner. The system prompt lists the context in the `available_skills` shape from PLAN.md P9 and exposes a `Skill` tool plus stub tools that end the turn. Every row from this runner is labelled `emulated`. `AnthropicProvider` and `OpenAiProvider` in `providers/http.rs` are the transports. The label is required so an emulated pass is not reported as a Claude Code pass.

Repetitions stay configurable. The default stays the skill-creator default of three runs and a 0.5 threshold, applied per outcome: a positive query passes only when `fired alone` is at least the threshold. `fired with others` and `hijacked` are failures for a positive query even when the candidate sometimes appears. A negative query passes when the candidate stays absent.

### L2 — Conflict matrix

The matrix is computed from L1's activation lists. Rows and columns are the skills in the context. For queries that belong to skill A:

- `stole` counts queries where B is the culprit and A did not activate
- `co-fired` counts queries where A and B both activated
- `neither` counts queries where neither activated

No second model call. The cell stores the query ids, so the report can open the transcript. A skill with no saved queries does not get a row until it has some.

### L3 — Library Interference Report

One report compares two machine snapshots and the candidate.

**Snapshot.** Export and import a JSON document, one entry per enabled skill: `name`, `description`, `content_hash`, `source_type`, `source_ref`, `source_revision`. Those are the columns already on `SkillRecord` in `crates/hive-core/src/db/skill_store.rs`. The document does not include secrets, install paths that point outside the library, or file bodies. `content_hash` is the version identity, because the table has no separate version column. Import creates a named context the trigger runner can load. It does not install those skills onto the machine.

**Static overlap.** Before any model call, compare descriptions and the `query` strings in each skill's `evals/trigger.json`. Flag a pair when the descriptions share a long distinctive phrase, or when one skill's trigger query is contained in the other's description. This scan is a warning list, not a verdict.

**Re-run.** Each saved trigger query for the candidate is run in four contexts, on the adapters selected for the report:

1. candidate alone
2. candidate plus machine A's library
3. candidate plus machine B's library
4. candidate plus one suspect, repeated for each skill the overlap scan or a previous hijack named

The four outcomes from L1 are stored per context. A query that is `fired alone` when the candidate is alone and `hijacked` or `fired with others` inside a library is interference.

**Bisect.** When a library context breaks a query, the view searches for the smallest distractor set that still breaks it. Start from the failing set. Split it in half, re-run the failing queries on each half, and keep every half that still produces `hijacked` or `fired with others`. Stop when removing any one remaining skill restores `fired alone`. The screen shows that set and the queries it breaks. This is a search over L1 runs, not a new judge.

**Rewrite re-test.** A suggestion is a replacement description. It is produced from the failing queries and the culprit names, using the description-optimization step PLAN.md already specifies (P8), once that step actually calls a provider. The suggestion is scored again in the same contexts that failed. It is not written onto the skill until the user clicks Apply, and Apply stays disabled when the re-test did not restore `fired alone` on the queries that broke.

**Install warning.** After a new skill is installed, re-run the saved trigger queries of the skills already in the library, in the library context that now includes the new skill. Compare each query's outcome with the last saved outcome. Warn when a query moves from `fired alone` to `fired with others`, `hijacked`, or `none`. Name the new skill as the suspect. Do not block the install. The warning is the regression, and it links to the bisect view with the new skill pinned in the set.

### L4 — Per-model pass bar

L1 and the expectation run record a pass rate per model, not one rate for the whole matrix. The user sets a pass bar per suite. The default for triggers is the 0.5 threshold above. The default for expectations is the fraction of expectations that must pass.

A description is flagged `strong-model only` when it meets the bar on every model the user marked strong, and misses the bar on at least one model they did not mark strong. The flag is stored on the trigger result and shown next to the description. It is not a failure by itself. A suite can require the bar on every selected model, which turns the flag into a failure.

The model id and the adapter label (`claude-code`, `codex`, `cursor`, `emulated`) are both stored, so a strong-model flag is not confused with an emulated pass.

### L5 — Gold set for the grader

Optional file `evals/gold.json` next to `evals.json`. Each row is a hand label: a trigger query with the expected outcome (`fired alone`, `fired with others`, `hijacked`, `none`, and the culprit name when the label is `hijacked`), or an expectation id with the expected `passed` boolean. The person writes this file. The run does not generate it.

When the file is present, the grader's verdicts and L1's outcomes on those rows are compared with the labels. The report shows agreement and the disagreements, with the transcript link. Disagreement does not change the suite's pass rate. It tells the user the grader and the labels diverged, which is the calibration. When the file is absent, the run records that calibration was not requested and does not invent labels.

A gold row that names a culprit is wrong if the activation list's culprit differs, even when both say `hijacked`. That keeps the calibration on the same ordered list L1 stored.
