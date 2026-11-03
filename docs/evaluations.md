# Evaluations: does the skill help?

A skill is a set of instructions. The only honest way to know whether it helps is to run the
same tasks with and without it and compare. Skill Hive automates that loop, following the
model of Anthropic's `skill-creator`, and keeps the results per version of the skill.

## Suites

A suite lives inside the skill directory so it travels with it:

```
<skill>/evals/
  evals.json     skill-creator compatible: {"skill_name", "evals": [{id, prompt, expected_output, files, expectations}]}
  checks.json    Hive extension: deterministic checks per eval id
  trigger.json   Hive extension: [{"query", "should_trigger"}]
```

- **Expectations** are verifiable statements about the result ("A CSV with the invoice line
  items exists", "Totals match the PDF").
- **Checks** verify an expectation without a model: `file_exists`, `file_contains`,
  `file_matches` (regex), `output_contains`, `output_matches`, `json_pointer`, `command` (exit
  code inside the sandbox). A check that names an expectation decides it; the grader is not
  asked.
- **Trigger cases** ask: given this user query and the list of available skills, would the agent
  pick this skill? Positive and negative queries give precision and recall of the description.

`hive eval suite <skill> --draft` (or *Draft a suite* in the app) writes a starter from the
description and its trigger phrases.

## Runs

A run config has:

- **Arms**: `with_skill` (the library version, snapshotted), `without_skill` (baseline), and
  optional `variant` arms pointing at another skill directory (an older snapshot, a rewrite, a
  competitor). The baseline for deltas is the first non-`with_skill` arm unless chosen.
- **Repeats** and **concurrency** (bounded by the provider's own limit).
- **Executor** provider: runs the task in a fresh sandbox directory per case × arm × repeat, with
  the case's input files copied in and, for skill arms, the skill installed where that agent
  discovers project skills (`.claude/skills/<name>` for Claude Code, `.codex/skills/<name>` for
  Codex) plus a one-line notice in the prompt. API providers (Anthropic, OpenAI-compatible) have
  no tools; they receive the skill body as the system prompt, which measures instruction quality
  but not tool use.
- **Grader** provider (optional): an LLM judge that reads the prompt, the final output, a
  transcript excerpt and the files created, and must answer strict JSON with `passed` and cited
  `evidence` per expectation. The burden of proof is on the expectation; there is no partial
  credit. Without a grader only checks count and grader-only expectations are excluded from the
  pass rate (noted on the result).
- **Trigger evaluation** (optional) through any provider that can complete text.

Every case result stores status, per-expectation results (with their source: check or grader),
pass counts, tokens, duration, tool calls, cost, an output excerpt, the sandbox path and the
transcript path. Progress streams to the UI and CLI as events; runs can be cancelled; runs left
open by a crash are marked failed on the next start.

## Benchmark

When every case is graded the run is aggregated into a `Benchmark`:

- Per arm: runs, errors, and mean / sample standard deviation / min / max of pass rate, tokens,
  duration and tool calls, plus cost when known.
- Per arm vs baseline: pass-rate delta, a **95 % bootstrap confidence interval** (2 000
  resamples, deterministic seed), **Welch's t-test** p-value (two-sided; absent when an arm has
  fewer than two results), token and duration deltas.
- **Verdict**: *inconclusive* when any arm has fewer than two results; *helps a lot* when the
  delta ≥ 0.25 and the CI excludes zero; *helps* when delta ≥ 0.10 and the CI lower bound is
  above −0.05; *hurts a lot* when delta ≤ −0.25 and the CI is below zero; *hurts* when delta ≤
  −0.10; otherwise *neutral*.
- **Expectation matrix**: pass fraction per arm for every expectation, with a pattern:
  *always passes* (does not measure the skill), *always fails* (broken expectation or beyond the
  model), *only with skill* (clear value), *only baseline* (the skill may be hurting), *flaky*
  (inconsistent across repeats), *mixed*.
- **Trigger summary**: precision, recall, F1 and every query's outcome.
- **Notes**: what the aggregates hide (expectations that never differentiate, token ratios,
  duration outliers).

`benchmark.json` and `benchmark.md` are written into the run workspace; `hive eval report
<run> --markdown` prints the latter. The item's *effectiveness* badge (verdict and delta) comes
from its latest finished run, and *history* plots every run against the skill's content hash so
you can see whether a rewrite moved the needle.

## Providers

| Kind | Executes tasks with tools | Completes text | Notes |
| --- | --- | --- | --- |
| Claude Code CLI | yes (`claude -p --output-format stream-json`) | yes | Found on PATH by `hive provider detect`. |
| Codex CLI | yes (`codex exec --json`) | yes | |
| Anthropic API | system-prompt only | yes | Key in the OS keychain or an env var. |
| OpenAI-compatible | system-prompt only | yes | OpenAI, OpenRouter, Ollama, LM Studio, vLLM… |
| Scripted | simulated | simulated | JSON rules; what the test suite and offline demo use. |

Roles: *executor*, *grader*, *recommender* (used by *Find* when model re-ranking is on).
Defaults are chosen in Settings or by `hive config set executor <id>`.
