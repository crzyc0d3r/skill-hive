# Plan conformance audit

This audit walks the effectiveness requirements in `PLAN.md` and scores each one against the code a user actually runs. A row is **real** only when a product path executes it: the HTTP adapters, the test engine, or a screen that reads that engine's report. A localhost server that speaks the Anthropic Messages or OpenAI chat-completions wire protocol counts as a real adapter test. A row is **stand-in** when the only proof is `ScriptedProvider`, a frozen fixture, demo mode, or a fake binary on `PATH`. A row is **missing** when that behavior is not on a product path. Stand-in rows are not done.

The measured product path is `run_effectiveness` in `crates/hive-core/src/testing/effectiveness.rs`, invoked by the Tauri command `run_effectiveness_suite`. It creates a skill, runs `with_skill` and `without_skill` through `AnthropicProvider` against the caller-supplied base URL, and writes `grading.json`, `benchmark.json`, `history.json`, trigger results, a CSV, and JUnit XML. The screen test in `src/features/effectiveness/view.test.tsx` renders that report shape. It does not launch the engine in the same process.

## Counts

| status | rows |
|---|---|
| real | 38 |
| stand-in | 12 |
| missing | 13 |

38 of 63 rows are on a real path. 25 are not done.

## What still needs a real key or a real installed agent

- A live call to `https://api.anthropic.com` needs an Anthropic API key. The adapter is tested against a local server that speaks the Messages SSE protocol.
- A live call to `https://api.openai.com`, Azure OpenAI, or another hosted OpenAI-compatible endpoint needs that vendor's key. The OpenAI adapter is tested against a local server that speaks `/chat/completions`.
- The reference trigger and expectation measurement (`claude -p --output-format stream-json`) needs the Claude Code CLI installed. CI and the effectiveness suite do not spawn it. `LocalCli` can spawn a program, and the tests point it at a fake binary.
- Codex `exec` needs the Codex CLI.
- Description optimization that asks a model for a new description, and the P12 blind comparator that asks a model to score A against B, are not sent to a provider by the measured run.
- Live skills.sh downloads and GitHub device-flow completion still need network access and a user. They are outside this engine, and they are unchanged.

## Part 2 §3 — skill-creator model

| id | requirement | status | evidence |
|---|---|---|---|
| 3.1-layout | Iteration workspace with `with_skill` and `without_skill`, `outputs/metrics.json`, `timing.json`, `grading.json`, and `transcript.jsonl` | real | `crates/hive-core/src/testing/engine.rs` writes the cell directory and `crates/hive-core/src/testing/aggregate.rs` writes `iteration-1/benchmark.json`. `effectiveness.rs` tests assert both configurations. |
| 3.1-names | Descriptive eval directory names, `eval_metadata.json`, `skill-snapshot/`, and viewer `feedback.json` | missing | The measured cell directory is `iteration-1/eval-1/`. No product path writes `eval_metadata.json`, `skill-snapshot/`, or `feedback.json`. |
| 3.2 | `evals/evals.json` in the skill-creator schema, plus `trigger.json` | real | `crates/hive-core/src/testing/suite.rs` `save_suite`. `write_skill` in `effectiveness.rs` stores one eval and two trigger rows. |
| 3.3 | `grading.json` with `text`, `passed`, and `evidence`, and app-filled metrics rather than model-invented numbers | real | `engine.rs` grades the cell and writes `grading.json` beside `outputs/metrics.json`. The effectiveness test requires `"passed": true` for `with_skill` and `"passed": false` for `without_skill`. |
| 3.4-benchmark | `benchmark.json` comparing `with_skill` and `without_skill` | real | `aggregate_benchmark` runs at the end of `run_plan`. The effectiveness test reads `iteration-1/benchmark.json` and requires both configuration names. |
| 3.4-old | `old_skill` baseline from a skill snapshot when improving an existing skill | stand-in | `engine.rs` copies `skill-snapshot` when that directory already exists (`sandbox_copies_case_files_old_skill_and_deletes_the_directory` in `testing/tests.rs`). The suite command never snapshots an existing skill; it always measures a skill it just wrote against `without_skill`. |
| 3.5-api | Trigger evaluation on the API runner: the first tool call decides whether the description fires | real | `judge_triggers` in `effectiveness.rs` posts two live chat requests and writes `trigger-results.json`. The positive query must call `Skill` with the skill name. This is one shot per query, not three runs at a 0.5 threshold, and the prompt is not the P9 distractor prompt. |
| 3.5-cli | `claude -p` trigger runner matching `run_eval.py`, including the temporary command file and deletion on every exit | missing | `crates/hive-core/src/testing/trigger.rs` `run_trigger_set` judges pre-recorded stream text. `LocalCli` can spawn a program, but the measured suite does not call it. |
| 3.6-blind | Blind A/B: the two sides stay unlabeled until a person records a verdict | real | The report stores grading text as `outputA` and `outputB` and keeps the configuration name in `sealed`. `src/features/compare/CompareView.tsx` renders "Output A" and "Output B" and shows `sealed` only after Record verdict. |
| 3.6-model | P12 comparator call and post-hoc `analysis.json` from a model | stand-in | `crates/hive-core/src/analytics/compare.rs` builds a comparator request from in-memory sides and `start_analysis` returns a string. `analytics/workflow.rs` states that every model reply is a frozen fixture. The measured run does not send P12. |
| 3.7-history | `history.json` with `grading_result` in `baseline`, `won`, `lost`, `tie`, a parent chain, and exactly one `is_current_best` | real | `history_entries` and `write_history` in `effectiveness.rs` and `crates/hive-core/src/analytics/history.rs`. A lower `with_skill` rate is `lost` and leaves `without_skill` as the current best. |
| 3.7-loop | Improve loop: rewrite the skill, re-run, and optimize the description on a train/test split, applied only on click | missing | `apply_optimized_description` in `src-tauri/src/commands.rs` writes the typed string through `analytics/optimize.rs`. It does not ask a model, does not split train and test, and does not re-run the suite. |
| 3.8 | `quick_validate.py` rules, reported together, on save and before packaging | real | `crates/hive-core/src/authoring/validator.rs` `validate_skill` is the save path. The Python script itself is not bundled; the Rust validator is the product check. |
| 3.9 | Claude Code, sequential, and static-viewer environment modes | missing | There is no mode switch. The measured run does not emit a static HTML viewer. |

## Part 3 — authoring, generation, testing, workflows

| id | requirement | status | evidence |
|---|---|---|---|
| A.1-editor | Skill editor with frontmatter, body, problem list, preview, versions, and the 500-line gauge | real | `src/features/skills/SkillEditor.tsx` and `crates/hive-core/src/authoring/validator.rs`. |
| A.1-tree | File tree with new, rename, delete, duplicate, executable bit, and the `allowed-tools` inserter | missing | The editor is one Markdown buffer. Those tree actions are not on the screen. |
| A.2 | Agent editor with camelCase fields, problems, preview, export targets, and versions | real | `src/features/agents/AgentEditor.tsx`. Codex and Cursor targets are not offered as unverified greyed rows; that specific empty state is missing from this screen and is not a separate done row. |
| A.3 | Draft versions with snapshot and restore | real | `snapshot_skill_draft` and `restore_skill_draft` in `src-tauri/src/commands.rs`, wired from the editor store. |
| B.1-ask | Generate wizard interview that calls a provider and stores the questions | real | The Ask control in `src/features/generate/GenerateWizard.tsx` calls `interview_generation`, which calls `GenerationSession::interview` through `AnthropicProvider` (`effectiveness.rs` `interview_goal`). The test `interview_stores_the_provider_questions` asserts the question returned by the mock server. |
| B.1-rest | Draft, review, revise, and save steps that call the provider for P2 through P6 | stand-in | Continue does not call a provider. Accept all defaults calls `save_generated_draft`, which runs `run_review` on `skill_envelope` locally. `generation/session.rs` still says replies come from recorded fixtures. The database row for that command uses endpoint id `replay`. |
| B.2 | Prompt templates P1 through P13 rendered and sent | stand-in | `crates/hive-core/src/generation/templates.rs` contains the template text. Only the interview path sends one of them to `AnthropicProvider`. The other templates are not the measured run. |
| C.1-files | Suite files `evals.json`, `trigger.json`, and `checks.json` | real | `save_suite` writes all three. The measured suite's check list is empty, so `checks.json` is an empty list rather than a user-authored check. |
| C.1-ui | Cases, trigger set, and checks editors on the suite screen | missing | `src/features/suites/SuiteView.tsx` shows the with/without labels, the endpoint, the goal, and Measure. It does not edit cases, queries, or checks. |
| C.2-trigger | Trigger tests as specified, including rate, threshold, and the P9 prompt | missing | See 3.5. The product judge is a single HTTP call without the threshold or the distractor catalog. |
| C.2-expect | Expectation tests in a sandbox with the skill injected only for `with_skill` | real | `engine.rs` `run_plan` injects the skill body for `with_skill` only. The effectiveness fixture answers `PINECONE` only when the request contains `HIVE_SKILL_MARKER_9f3a`. |
| C.2-checks | Deterministic checks from `checks.json`, immune to the grader | missing | The engine writes an empty `checks.json` beside the grade. Nothing runs a check command or prevents the grader from overturning it. |
| C.2-ab | Version-versus-version blind comparator over outputs | stand-in | The screen blinds the two grading files from one measured run. It does not run P12, and it does not compare two versions or two endpoints. |
| C.3-matrix | Run matrix: configurations, endpoints, models, repetitions, estimate, presets, progress events | missing | `Plan` in `engine.rs` is a single endpoint, model, and repetition. The suite screen has no matrix chips, estimate line, or `test-run-progress` events. |
| C.3-budget | Budget pause with Continue, Stop and grade, or Cancel | stand-in | `engine.rs` can pause on `budget_usd`. The results screen's Budget dialog only sets a local paused flag (`ResultsView.tsx`). It does not call the engine. |
| C.4-summary | Results summary with mean, standard deviation, and a delta row | real | The report's `summary` array is the two pass rates and the delta. Standard deviation is 0 because the plan runs one repetition. `ResultsView.tsx` prints the two rates. |
| C.4-heatmap | Heatmap of cells, timeline from the transcript, and jump to evidence | real | Cells come from `get_effectiveness_report`. Timeline steps are parsed from `with_skill/transcript.jsonl` (`timeline_steps`). The evidence field resolves a step number or an `outputs/` path. |
| C.4-rest | Analyst notes, trigger panel, cell detail sheet, and per-expectation grading cards | missing | The results screen does not render notes, the trigger table, or a cell sheet. |
| C.5-history | History table fed by the measured `history.json` | real | `HistoryView.tsx` reads `history.iterations` from the effectiveness report when demo mode is off. |
| C.5-rules | Regression rules R1 through R6 against a pinned baseline | real | `record_regression` calls `analytics/regression.rs` `evaluate`. The first run has no baseline, so the report offers set-baseline and writes `regression/baseline.json`. Alerts appear on a later run against that file. The history screen lists them. |
| C.6-export | Export CSV and JUnit from the graded cells, with no endpoint secret | real | `write_run_export` writes `export/cells.csv` and `export/junit.xml` from `grading.json`. The results Export button shows that CSV. |
| C.6-rest | `benchmark.md`, skill-creator workspace zip, static HTML, and copy-summary | stand-in | `analytics/export.rs` can write a workspace and HTML for a hand-built `ExportSlice`. The results screen does not call it. `aggregate.rs` does write `benchmark.md` next to `benchmark.json` during `run_plan`. |
| D.1 | Create a skill from scratch and prove it by running the suite | real | `run_effectiveness` writes `SKILL.md`, runs the suite, and stores the report. The wizard's save step is still the local envelope, so the full D.1 narrative (interview, draft, revise, editor, then suite) is only the suite half. |
| D.2 | Import an existing skill and benchmark it | stand-in | `analytics/workflow.rs` `demonstrate_d2` replays fixtures. Import on the Install screen does not start a measured run. |
| D.3 | Generate an agent and test it on two endpoints | missing | The measured run is one Anthropic base URL and a skill, not an agent and not two endpoints. `demonstrate_d3` is a fixture recording. |

## Part 1 §4 — provider layer

| id | requirement | status | evidence |
|---|---|---|---|
| 4.1 | Provider-agnostic chat interface used by the engine | real | `ChatProvider` in `crates/hive-core/src/providers/types.rs`. `run_plan` takes `&dyn ChatProvider`. |
| 4.2 | Anthropic Messages adapter, `POST {base}/v1/messages`, `x-api-key`, `anthropic-version`, SSE | real | `AnthropicProvider` in `crates/hive-core/src/providers/http.rs`. `providers/tests.rs` `anthropic_openai_and_ollama_answer_on_localhost` and the effectiveness mock both speak that protocol. The base URL is an argument. |
| 4.3 | OpenAI-compatible adapter, Bearer or Azure `api-key`, `/chat/completions` | real | `OpenAiProvider` in `http.rs`, covered by the same localhost test. The new suite command constructs `AnthropicProvider` only. |
| 4.4 | Local agent CLI executor (`claude -p` stream-json, Codex probe) | stand-in | `crates/hive-core/src/providers/cli_agent.rs` spawns the configured program and parses stream-json. Tests and probes use a stand-in binary, not a vendor CLI. The suite command does not select this executor. |
| 4.5-secrets | Secrets redacted from transcripts and exports | real | `Transcript::push` scrubs the secret. CSV export does not write the API key. |
| 4.5-retry | Retries, rate limits, and cost on the provider middleware | real | `crates/hive-core/src/providers/middleware.rs`, with tests for retry-after, rate-limit ledger rows, and cost. The effectiveness command does not wrap the provider in that middleware, so a measured run does not retry. |
| 4.6 | Test connection flow with probe stages | stand-in | `providers/probe.rs` probes a provider and is tested on localhost. `src/features/endpoints/EndpointsView.tsx` renders the stage names and stores a secret in the page store. The Probe control does not call `probe_provider`. |

## Part 1 §5 — test execution engine

| id | requirement | status | evidence |
|---|---|---|---|
| 5.1 | Run lifecycle from a plan to persisted rows | real | `run_plan` in `engine.rs` writes `runs`, `run_cases`, and `usage_ledger`. The effectiveness test counts those tables in `hive.db`. |
| 5.2 | Isolated working directories and deletion after the cell | real | The sandbox directory is per cell. `testing/tests.rs` covers deletion. |
| 5.3 | Executors: injected skill, tool loop (`write_file`, `read_file`, `bash`, `list_dir`) jailed to the cell | real | `engine.rs` tool loop. The mock server's `write_file` lands in `outputs/answer.txt`. |
| 5.4 | Transcript capture and tool-call metrics | real | `transcript.jsonl` records `tool_use` and `tool_result`. `outputs/metrics.json` stores tool counts. `timeline_steps` reads those timestamps for the results screen. |
| 5.5-grade | Grading, aggregation, cancellation token, and SQLite persistence | real | `grade_cell`, `aggregate_benchmark`, and `CancellationToken` in the provider call. Cancel-after is a plan field the suite command leaves unset. |
| 5.6 | Seeded reproducibility and record/replay | real | The suite uses seed 7, so the run id is stable. `middleware.rs` record-then-replay is tested and is not what `run_effectiveness` uses; that command is live HTTP. |

## Part 4 §5.3 — new screens

| id | requirement | status | evidence |
|---|---|---|---|
| 5.3-agents | Agents library with search, tags, import, and the description budget | stand-in | `src/features/agents/AgentsView.tsx` lists agents. The budget meter, import sheet, and tag filter from the spec are not on that screen. Demo mode seeds cards. |
| 5.3-skill-editor | Skill editor screen as specified | real | Route `/skills/:id/edit` renders `SkillEditor`. See A.1 for the missing tree. |
| 5.3-agent-editor | Agent editor screen as specified | real | Route `/agents/:id/edit` renders `AgentEditor`. |
| 5.3-generate | Five-step generate sheet that calls a provider | real | `GenerateWizard.tsx` has the five steps. Only Ask calls a provider. Accept all defaults stays on the local draft command so the existing empty-interview test still holds. |
| 5.3-suite | Suite screen that starts a measured run against an endpoint | real | Measure posts `run_effectiveness_suite` with the typed base URL and goal. The Run suite link to `/runs/run-fixture` remains for the older chip and is not a measured run. |
| 5.3-matrix | Run matrix and live progress grid | missing | No matrix panel and no progress events. |
| 5.3-results | Results screen reading the measured report, including export | real | `ResultsView.tsx` prefers `get_effectiveness_report`, then the log excerpt. Export shows the CSV from the report. |
| 5.3-cell | Cell detail sheet with transcript, timeline, outputs, grading, checks, and feedback | missing | Timeline and evidence are inline. There is no sheet and no feedback file. |
| 5.3-compare | Compare screen, blind until the verdict | real | `CompareView.tsx`, covered by `src/features/effectiveness/view.test.tsx`. |
| 5.3-history | History screen with the measured chain and regression alerts | real | `HistoryView.tsx`. The trend chart, version diff, and re-pin control from the spec are not on the screen. |
| 5.3-endpoints | Endpoint catalog with keychain secrets, prices, and a live probe | stand-in | `EndpointsView.tsx` and `providers/probe.rs`. The screen does not probe, and secrets stay in the page store rather than the OS keychain. |

## How the two tests meet

`crates/hive-core/src/testing/effectiveness.rs` `measured_run_writes_artifacts_and_database_rows` creates the skill, runs the suite against a local Anthropic SSE server, and asserts `grading.json`, `benchmark.json`, `history.json`, `trigger-results.json`, `export/cells.csv`, `export/junit.xml`, the timeline timestamps, and rows in `runs`, `run_cases`, and `usage_ledger`.

`src/features/effectiveness/view.test.tsx` drives Measure, the results summary, Export, the blinded compare, the history row, and Ask. The webview test stubs IPC with the report shape. It does not open `hive.db`.
