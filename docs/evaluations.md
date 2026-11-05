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

Progress streams to the UI and CLI as events; runs can be cancelled; runs left open by a crash
are marked failed on the next start.

### What a case result records

One result per case × arm × repeat. Besides the verdict it keeps:

- **Status** (queued, running, executed, graded, failed, timeout, cancelled) and the error text
  when there is one.
- **Expectations**: every expectation with passed / failed, the evidence, and its source
  (`check` or `grader`); the passed, failed and total counts; the pass rate; the grader's notes.
- **Tokens** (input and output, as the executor reports them), **duration**, **tool calls** and
  **cost** when the executor knows it.
- **Grader tokens**: input and output tokens the LLM grader spent on this case (0 without a
  grader). They are kept apart from the executor's tokens so a verbose grader does not inflate
  the arm comparison.
- An **output excerpt** (the first characters of the final answer), the **sandbox path** and the
  **transcript path** (`.hive/transcript.jsonl`, the raw event stream).
- The **activity trace**, described next. It is optional: runs recorded before it existed, and
  providers that cannot report it, leave it empty, and the UI, CLI and benchmark show nothing
  rather than zeros.

#### The activity trace

The trace is read off the executor's event stream while the case runs; nothing is asked of the
model afterwards. It records *how* the agent worked, which the pass rate alone does not show:

- **Turns**: model calls. For Claude Code one per distinct assistant message (falling back to
  the reported `num_turns`); for Codex one per item; one for an API provider.
- **Tool calls by category**: `reads`, `searches`, `writes`, `commands`, `web`, `subagents`,
  `otherTools`, and `toolCalls` for the total. The category comes from the tool name when it is
  unambiguous (`Read`, `Glob`, `Grep`, `Write`, `Edit`, `WebFetch`, `Task`, …). For shell tools
  (`Bash`, Codex command executions) it comes from the command line: `cat`, `head`, `tail`,
  `sed -n`, `jq`, `diff` and the like are reads; `ls`, `find`, `grep`, `rg`, `tree`, `wc`,
  `git status` / `git log` / `git diff` are searches; a `>` redirect (other than `2>&1`
  plumbing), `| tee`, `mkdir`, `cp`, `mv`, `rm`, `touch`, `sed -i` are writes; `curl` and
  `wget` are web; everything else is a command. A shell wrapper (`bash -lc '…'`, which is how
  Codex runs every command) is unwrapped first, a heredoc body is ignored, a `>` inside quotes
  is not a redirect, and a leading `cd … &&`, `export … &&`, `sudo`, `env` and variable
  assignments are skipped; only the first pipeline stage decides. MCP tools, todo lists and
  questions to the user are *other*. The raw count per tool name is kept too (`tools`), so a
  disputed classification can always be checked. Events of subagents (Claude Code forwards
  them with `parent_tool_use_id`) are counted apart in `subagentTurns` and
  `subagentToolCalls`, shown in the timeline with a `↳` prefix, and left out of the headline
  counters and the tool time, which the parent's own `Task` call already spans.
- **Tool errors**: results the agent's tool flagged as an error (non-zero exit codes for Codex
  commands, `is_error` for Claude Code).
- **Files read and written**: the distinct paths, sandbox-relative when inside the sandbox,
  capped at 60 per list; `filesReadCount` and `filesWrittenCount` are the true distinct counts.
  A read command's target (`cat path`, `head -n 5 path`) counts as a file read.
- **Skill reads**: reads of any file inside the installed skill directory, in plain words "did
  the agent open `SKILL.md`" (or a bundled script). Loading the skill through Claude Code's
  `Skill` tool counts as a read of `SKILL.md`; invoking some other skill does not. A skill arm
  with zero skill reads got no help from the skill, whatever the pass rate says. Always 0 in
  the `without_skill` arm. API providers never read files: they get `SKILL.md` as the system
  prompt, and the trace says so with `skillInContext` instead.
- **Thinking and text**: the number of thinking (chain-of-thought) blocks and their characters,
  and the number of visible text blocks and their characters. A redacted thinking block counts
  as a block of unknown length.
- **Tool input and result characters**: the serialised tool arguments (file contents written,
  commands, patterns) and the tool results returned to the model.
- **Output-token split** (`thinkingTokens`, `textTokens`, `toolInputTokens`): the output tokens
  the executor reported, divided among thinking, text and tool arguments in proportion to their
  characters. An estimate: it says where the output went, not what the tokenizer counted. When
  the executor reports no output tokens the three are estimated at four characters per token.
- **Tool-result tokens** (`toolResultTokens`): the tool-result characters at four characters per
  token. An estimate on the input side.
- **IO share** (`ioShare`, 0…1):
  `(toolInputTokens + toolResultTokens) / (toolInputTokens + toolResultTokens + thinkingTokens + textTokens)`.
  The fraction of the model's traffic that moved data through tools rather than reasoned or
  explained. A high share with a low pass rate usually means the agent was busy rather than
  effective.
- **Cache split** (`cacheReadTokens`, `cacheCreationTokens`, `uncachedInputTokens`): the input
  tokens served from the prompt cache, written to it, and neither, as the executor reports them.
  Claude Code gives all three (from the final `result` event, or summed over the assistant
  messages when it is missing); Codex reports cached input (a subset of its input tokens) and
  cache writes, from which the three follow.
- **Model time** (`modelTimeMs`): time spent waiting for the model. Claude Code's
  `duration_api_ms`; an API provider's request latency; absent for Codex, which does not report
  it. **Tool time** (`toolTimeMs`): the sum of the gaps between each tool call and its result,
  measured on Hive's clock as the stream lines arrive, so it includes the agent's own overhead
  around the tool; absent when the stream was not timed. `firstToolMs` is when the first tool
  call happened. Model time, tool time and the case duration are three different clocks, not
  parts of one total.
- **Timeline** (`steps`): the ordered steps, each with `atMs` (milliseconds since the execution
  started), `kind` (`thinking`, `text`, `tool_use`, `tool_result`), the tool name and category
  for tool steps, a summary of at most 160 characters (the path, the command, the pattern, or
  the first words of the text), the characters of the block and `isError`. At most 400 steps are
  kept; `stepsTruncated` counts the rest. The full stream is still in the transcript.

What each executor contributes:

| Executor | Trace content |
| --- | --- |
| Claude Code CLI | Everything above: tool calls with categories and paths, thinking blocks when the model emits them, the cache split, `duration_api_ms` as model time, timed tool gaps. |
| Codex CLI | Commands (classified from the script inside the `bash -lc` wrapper), file changes and patches as writes, reasoning items as thinking, web searches, MCP and custom tool calls as *other*; cached, uncached and cache-write input tokens; no model time. Both the current `item.*` and the legacy `msg` schemas are read. |
| Anthropic API, OpenAI-compatible | One turn, one text block, no tools; latency as model time; `skillInContext` when the skill was the system prompt. The trace says how long the answer was and nothing else. |
| Scripted | Simulated: every file written is a write; a rule's `reads`, `searches` and `commands` add that many calls of each kind (the first read opens `SKILL.md` when a skill is installed); `thinking` is one thinking block. |

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
- **Activity** per arm, over the scored results that carry a trace (`traced` says how many):
  mean / sample standard deviation / min / max of turns, tool calls, reads, searches, writes,
  commands, web calls, tool errors, distinct files read and written, thinking / text /
  tool-argument / tool-result tokens, IO share, cache-read tokens, fresh input tokens (cache
  creation plus uncached), model time and tool time (`n` is 0 for a clock no trace reported),
  and the **skill-opened share**: the fraction of traced runs that read a file of the skill,
  invoked it, or had it in context. The share only means something for arms that have a
  skill; `without_skill` is always 0. Arms without a single trace have no activity at all, so
  a run recorded before this version shows none.
- **Notes**: what the aggregates hide (expectations that never differentiate, token ratios,
  duration outliers), and from the activity: whether a skill arm opened the skill in every run
  or only in some ("with_skill opened the skill in only 1 of 2 run(s): the other run(s) never
  read it, so its instructions could not help there"), arms that average one tool error per run
  or more, and arms that read 1.5× as many files, think 1.5× as much, or spend 15 points more or
  less of their traffic on tool IO than the baseline ("with_skill spends 55% of its traffic on
  tool IO vs 25% for without_skill: more of its tokens move data through tools, fewer reason or
  explain").

`benchmark.json` and `benchmark.md` are written into the run workspace; `hive eval report
<run> --markdown` prints the latter. When any arm has activity the markdown carries an
`## Activity` table, one metric per row and one column per arm (traced runs, model turns, file
reads, searches, writes, commands, web, tool errors, distinct files read and written, opened the
skill, the four estimated token figures, IO share, cache-read and fresh input tokens, model time
and tool time; `—` where an arm has no figure), and `hive eval report` prints a shorter version
of the same table after the per-arm summary. The item's *effectiveness* badge (verdict and
delta) comes from its latest finished run, and *history* plots every run against the skill's
content hash so you can see whether a rewrite moved the needle.

## Providers

| Kind | Executes tasks with tools | Completes text | Activity trace | Notes |
| --- | --- | --- | --- | --- |
| Claude Code CLI | yes (`claude -p --output-format stream-json`) | yes | full | Found on PATH by `hive provider detect`. |
| Codex CLI | yes (`codex exec --json`) | yes | tools, reasoning, cache; no model time | |
| Anthropic API | system-prompt only | yes | one text turn | Key in the OS keychain or an env var. |
| OpenAI-compatible | system-prompt only | yes | one text turn | OpenAI, OpenRouter, Ollama, LM Studio, vLLM… |
| Scripted | simulated | simulated | simulated | JSON rules; what the test suite and offline demo use. |

Roles: *executor*, *grader*, *recommender* (used by *Find* when model re-ranking is on).
Defaults are chosen in Settings or by `hive config set executor <id>`.
