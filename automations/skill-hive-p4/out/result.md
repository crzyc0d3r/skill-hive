# Phase 4 result

Product: Skill Hive.

The split is graph-informed. graphify extract --code-only wrote graphify-out/graph.json (graphifyy 0.9.90). Communities 73, 21, 62, 42, and 7 compile into the hive-core test binary and were merged into n1_core, which also owns community 29 (hive-cli). Communities 23 and 54 share App.tsx and were merged with communities 35 and 43 into n2_screens. Shadcn communities were not given nodes.

Criterion 1: pass. Anthropic, OpenAI-compatible (Ollama flavor), and Claude Code CLI endpoints configure, probe through seven stages, and run. The scrub test scans return payloads, logs, transcripts, and exports.
Criterion 2: pass. The fixture skill run with with_skill, without_skill, three repetitions, trigger tests, and one deterministic check finishes in live mode against a local endpoint and in replay mode. Limit: no vendor API key is configured, so live mode is the local HTTP endpoint, not api.anthropic.com or api.openai.com.
Criterion 3: pass. Trigger verdicts agree with fixtures/testing/run_eval.py on at least 18 of 20 queries across three runs.
Criterion 4: pass. A run pauses at the budget, cancellation kills the process tree (job object on Windows, process group on Unix), and resume finishes the remaining cells.
Criterion 5: pass. C.7 criteria 1 to 9 pass. The replay regression suite is a hive-core test, so the Ubuntu, macOS, and Windows cargo test jobs run it with zero LLM cost.

Gate summary:
core exit 0
frontend npm test 0

auto-approved by --auto
graphify was installed and the split followed graphify-out/graph.json.
