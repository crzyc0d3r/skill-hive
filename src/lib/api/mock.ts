// In-memory backend used outside Tauri: browser development, unit tests and screenshots.
// It mirrors the Rust command surface closely enough to exercise every screen.

import type * as T from "./bindings";
import type { Backend } from "./client";
import { ApiError } from "./client";

const now = Date.now();
const day = 86_400_000;
let seq = 1000;
const nextId = (prefix: string) => `${prefix}-${(seq++).toString(36)}`;

const HOME = "/Users/you";
const LIB = `${HOME}/.skill-hive/library`;

function src(kind: T.SourceKind, locator: string, subpath?: string): T.SourceRef {
  return {
    kind,
    locator,
    subpath: subpath ?? null,
    reference: kind === "github" ? "main" : null,
    revision: kind === "github" ? "b3349684c1d2" : null,
    targetId: null,
    fetchedAt: now - 3 * day,
  };
}

function item(
  partial: Partial<T.LibraryItem> & Pick<T.LibraryItem, "name" | "description">,
  body = "",
): T.LibraryItem & { body: string } {
  const kind = partial.kind ?? "skill";
  const slug = partial.slug ?? partial.name;
  const path = kind === "skill" ? `${LIB}/skills/${slug}` : `${LIB}/${kind}s/${slug}.md`;
  return {
    id: partial.id ?? nextId("item"),
    kind,
    name: partial.name,
    description: partial.description,
    slug,
    path,
    contentHash: partial.contentHash ?? Math.random().toString(16).slice(2).padEnd(40, "0"),
    sizeBytes: partial.sizeBytes ?? 2400 + Math.floor(Math.random() * 40_000),
    tokenEstimate: partial.tokenEstimate ?? Math.ceil(body.length / 4) + 300,
    descriptionTokens: Math.ceil(partial.description.length / 4),
    tags: partial.tags ?? [],
    source: partial.source ?? src("github", "anthropics/skills", `skills/${slug}`),
    frontmatter: { name: partial.name, description: partial.description, ...(partial.frontmatter ?? {}) },
    validation: partial.validation ?? "ok",
    effectiveness: partial.effectiveness ?? null,
    favorite: partial.favorite ?? false,
    notes: partial.notes ?? null,
    deploymentCount: partial.deploymentCount ?? 0,
    createdAt: partial.createdAt ?? now - 7 * day,
    updatedAt: partial.updatedAt ?? now - 2 * day,
    body,
  };
}

const PDF_BODY = `# PDF Processing Guide

## Overview

This guide covers essential PDF processing operations using Python libraries and command-line tools.

## Quick start

\`\`\`python
import pdfplumber
with pdfplumber.open("file.pdf") as pdf:
    for page in pdf.pages:
        print(page.extract_text())
\`\`\`

## Tables

Use \`page.extract_tables()\` and write the result with pandas. Always check the first row for headers.

## Forms

If the user needs to fill a form, read \`FORMS.md\` first.
`;

const items = [
  item(
    {
      name: "pdf",
      description:
        "Use this skill whenever the user wants to do anything with PDF files: reading or extracting text and tables, merging, splitting, rotating, watermarking, filling forms, OCR on scanned PDFs.",
      tags: ["documents", "official"],
      effectiveness: { runId: "run-1", verdict: "helps_a_lot", withSkillPassRate: 0.92, baselinePassRate: 0.5, delta: 0.42, finishedAt: now - day },
      deploymentCount: 2,
      favorite: true,
      frontmatter: { license: "Proprietary. LICENSE.txt has complete terms" },
    },
    PDF_BODY,
  ),
  item(
    {
      name: "xlsx",
      description:
        "Comprehensive spreadsheet creation, editing and analysis with formulas, formatting and charts. Use when the user asks for an Excel file, a .xlsx, or data analysis in a spreadsheet.",
      tags: ["documents", "official"],
      deploymentCount: 1,
      effectiveness: { runId: "run-2", verdict: "helps", withSkillPassRate: 0.8, baselinePassRate: 0.62, delta: 0.18, finishedAt: now - 3 * day },
    },
    "# Spreadsheets\n\nUse openpyxl for .xlsx. Recalculate formulas with LibreOffice when values matter.\n",
  ),
  item(
    {
      name: "docx",
      description:
        "Create, edit and analyse Word documents with tracked changes, comments and formatting. Use when the user mentions a .docx or asks for a report document.",
      tags: ["documents", "official"],
    },
    "# Word documents\n\nUse docx-js for new files and python-docx for edits.\n",
  ),
  item(
    {
      name: "pptx",
      description:
        "Build and edit PowerPoint presentations: layouts, speaker notes, images and charts. Use when the user asks for slides or a deck.",
      tags: ["documents", "official"],
    },
    "# Presentations\n\nStart from html2pptx when designing from scratch.\n",
  ),
  item(
    {
      name: "skill-creator",
      description:
        "Guide for creating effective skills: drafting SKILL.md, writing evals, running with-skill vs without-skill comparisons and iterating. Use when the user wants to write or improve a skill.",
      tags: ["authoring", "official"],
      deploymentCount: 1,
    },
    "# Skill creator\n\nDecide what the skill should do, draft it, write 2–3 realistic prompts, run with and without the skill, compare, rewrite.\n",
  ),
  item(
    {
      name: "systematic-debugging",
      description:
        "Four-phase debugging discipline: reproduce, isolate, diagnose, fix. Use when a bug resists the first obvious fix or when a test fails for an unclear reason.",
      tags: ["workflow", "debugging"],
      source: src("github", "obra/superpowers", "skills/systematic-debugging"),
      deploymentCount: 1,
    },
    "# Systematic debugging\n\n1. Reproduce. 2. Isolate. 3. Diagnose. 4. Fix and add a regression test.\n",
  ),
  item(
    {
      name: "test-driven-development",
      description:
        "Write the failing test first, make it pass with the smallest change, then refactor. Use when adding behaviour to code that has or should have tests.",
      tags: ["workflow", "testing"],
      source: src("github", "obra/superpowers", "skills/test-driven-development"),
      effectiveness: { runId: "run-3", verdict: "neutral", withSkillPassRate: 0.7, baselinePassRate: 0.68, delta: 0.02, finishedAt: now - 5 * day },
    },
    "# TDD\n\nRed, green, refactor. Never write production code without a failing test.\n",
  ),
  item(
    {
      name: "conventional-commits",
      description:
        "Write commit messages in the Conventional Commits format with a scoped type, imperative subject and a body that explains why. Use when committing changes.",
      tags: ["git"],
      source: src("local_path", `${HOME}/dev/team-skills/conventional-commits`),
    },
    "# Conventional commits\n\n`type(scope): subject`. Types: feat, fix, docs, refactor, test, chore.\n",
  ),
  item(
    {
      name: "git-commit-helper",
      description:
        "Helps write commit messages using the conventional commits convention with a type, scope and imperative subject line. Use when the user is committing.",
      tags: ["git"],
      source: src("agent_dir", `${HOME}/.claude/skills/git-commit-helper`),
      validation: "warnings",
    },
    "# Commit helper\n\nAlways use conventional commit prefixes.\n",
  ),
  item(
    {
      name: "sql-query-tuning",
      description:
        "Analyse slow PostgreSQL queries with EXPLAIN ANALYZE, propose indexes and rewrite joins. Use when a query is slow or a database endpoint times out.",
      tags: ["database", "performance"],
      source: src("github", "supabase/agent-skills", "skills/sql-query-tuning"),
    },
    "# Query tuning\n\nRun EXPLAIN (ANALYZE, BUFFERS). Look for Seq Scan on large tables.\n",
  ),
  item(
    {
      name: "webapp-testing",
      description:
        "Test local web applications with Playwright: launch, navigate, assert, capture screenshots and logs. Use when verifying a web UI works or debugging front-end behaviour.",
      tags: ["testing", "web", "official"],
      effectiveness: { runId: "run-4", verdict: "hurts", withSkillPassRate: 0.45, baselinePassRate: 0.6, delta: -0.15, finishedAt: now - 9 * day },
    },
    "# Web app testing\n\nUse scripts/with_server.py to start the app, then drive it with Playwright.\n",
  ),
  item(
    {
      name: "brand-guidelines",
      description:
        "Apply the company brand colours, typography and tone to generated material. Use when designing marketing or customer-facing documents.",
      tags: ["design", "official"],
      validation: "errors",
    },
    "",
  ),
  item(
    {
      kind: "agent",
      name: "code-reviewer",
      description: "Reviews pull requests for correctness, security and style. Delegate when the user asks for a review.",
      tags: ["review"],
      source: src("agent_dir", `${HOME}/.claude/agents/code-reviewer.md`),
      frontmatter: { tools: "Read, Grep, Glob", model: "sonnet" },
      deploymentCount: 1,
    },
    "You are a meticulous code reviewer. Prioritise correctness bugs, then security, then clarity.\n",
  ),
  item(
    {
      kind: "agent",
      name: "test-writer",
      description: "Writes focused unit tests for a module. Use when coverage is missing for new code.",
      tags: ["testing"],
      source: src("authored", ""),
      frontmatter: { tools: "Read, Write, Bash", model: "haiku" },
    },
    "Write the smallest test that would catch the bug described. Prefer table-driven tests.\n",
  ),
  item(
    {
      kind: "command",
      name: "standup",
      description: "Summarise yesterday's commits and open PRs into a standup update.",
      tags: ["workflow"],
      source: src("authored", ""),
    },
    "Summarise the git log since yesterday for $ARGUMENTS as Yesterday / Today / Blockers.\n",
  ),
];

const byId = new Map(items.map((i) => [i.id, i]));
const byName = (name: string) => items.find((i) => i.name === name)!;

const targetSpecs: T.TargetSpec[] = [
  { id: "claude-code", name: "Claude Code", category: "coding", userSkillsDir: ".claude/skills", projectSkillsDir: ".claude/skills", userAgentsDir: ".claude/agents", projectAgentsDir: ".claude/agents", userCommandsDir: ".claude/commands", projectCommandsDir: ".claude/commands", detectDir: ".claude", descriptionBudgetChars: 15000, docsUrl: "https://docs.claude.com/en/docs/claude-code/skills", custom: false },
  { id: "codex", name: "Codex CLI", category: "coding", userSkillsDir: ".codex/skills", projectSkillsDir: ".codex/skills", detectDir: ".codex", descriptionBudgetChars: 15000, docsUrl: "https://developers.openai.com/codex/skills", custom: false },
  { id: "cursor", name: "Cursor", category: "coding", userSkillsDir: ".cursor/skills", projectSkillsDir: ".cursor/skills", detectDir: ".cursor", descriptionBudgetChars: 15000, docsUrl: "https://cursor.com/docs/context/skills", custom: false },
  { id: "gemini-cli", name: "Gemini CLI", category: "coding", userSkillsDir: ".gemini/skills", projectSkillsDir: ".gemini/skills", detectDir: ".gemini", descriptionBudgetChars: 15000, custom: false },
  { id: "github-copilot", name: "GitHub Copilot", category: "coding", userSkillsDir: ".copilot/skills", projectSkillsDir: ".github/skills", detectDir: ".copilot", descriptionBudgetChars: 15000, custom: false },
  { id: "windsurf", name: "Windsurf", category: "coding", userSkillsDir: ".codeium/windsurf/skills", projectSkillsDir: ".windsurf/skills", detectDir: ".codeium/windsurf", descriptionBudgetChars: 15000, custom: false },
  { id: "opencode", name: "OpenCode", category: "coding", userSkillsDir: ".config/opencode/skills", projectSkillsDir: ".opencode/skills", detectDir: ".config/opencode", descriptionBudgetChars: 15000, custom: false },
  { id: "agents-shared", name: "Shared (.agents)", category: "coding", userSkillsDir: ".agents/skills", projectSkillsDir: ".agents/skills", detectDir: ".agents", descriptionBudgetChars: 15000, custom: false },
];

/** Original user skills dir per target, so `target_set_skills_dir(id, null)` can reset an override. */
const defaultSkillsDirs = new Map<string, string>();

interface MockState {
  deployments: T.Deployment[];
  loadouts: T.Loadout[];
  providers: T.ProviderConfig[];
  settings: T.Settings;
  projects: T.ProjectRef[];
  runs: T.EvalRun[];
  caseResults: Map<string, T.CaseResult[]>;
  suites: Map<string, T.EvalSuite>;
  unmanaged: Record<string, T.InstalledAsset[]>;
}

function deployment(itemName: string, targetId: string, mode: T.LinkMode = "link", state: T.InstallState = "linked"): T.Deployment {
  const it = byName(itemName);
  const spec = targetSpecs.find((t) => t.id === targetId)!;
  const dir = it.kind === "skill" ? spec.userSkillsDir : it.kind === "agent" ? spec.userAgentsDir : spec.userCommandsDir;
  return {
    id: nextId("dep"),
    itemId: it.id,
    itemName: it.name,
    targetId,
    scope: "user",
    projectPath: null,
    path: `${HOME}/${dir}/${it.name}${it.kind === "skill" ? "" : ".md"}`,
    mode,
    deployedHash: it.contentHash,
    state,
    createdAt: now - 4 * day,
    updatedAt: now - day,
  };
}

function makeBenchmark(skill: string, withRate: number, baseRate: number, verdict: T.Verdict): T.Benchmark {
  const stats = (mean: number, sd: number, n: number): T.Stats => ({ n, mean, stddev: sd, min: Math.max(0, mean - sd * 1.5), max: Math.min(1, mean + sd * 1.5) });
  return {
    skillName: skill,
    skillHash: "b3349684c1d2e4f5",
    executor: "Claude Code (claude-sonnet-4-5)",
    grader: "Anthropic API (claude-sonnet-4-5)",
    arms: [
      { label: "with_skill", runs: 6, errors: 0, pass_rate: stats(withRate, 0.08, 6), tokens: { n: 6, mean: 41200, stddev: 6100, min: 32000, max: 50100 }, duration_ms: { n: 6, mean: 98_000, stddev: 21_000, min: 70_000, max: 130_000 }, tool_calls: { n: 6, mean: 19, stddev: 4, min: 13, max: 25 }, cost_usd: 0.42 } as unknown as T.ArmSummary,
      { label: "without_skill", runs: 6, errors: 0, pass_rate: stats(baseRate, 0.14, 6), tokens: { n: 6, mean: 28400, stddev: 9000, min: 17000, max: 41000 }, duration_ms: { n: 6, mean: 76_000, stddev: 30_000, min: 40_000, max: 120_000 }, tool_calls: { n: 6, mean: 11, stddev: 5, min: 5, max: 18 }, cost_usd: 0.3 } as unknown as T.ArmSummary,
    ].map((a) => ({
      label: a.label,
      runs: a.runs,
      errors: a.errors,
      passRate: (a as unknown as { pass_rate: T.Stats }).pass_rate,
      tokens: a.tokens,
      durationMs: (a as unknown as { duration_ms: T.Stats }).duration_ms,
      toolCalls: (a as unknown as { tool_calls: T.Stats }).tool_calls,
      costUsd: (a as unknown as { cost_usd: number }).cost_usd,
    })),
    baseline: "without_skill",
    deltas: [
      {
        arm: "with_skill",
        baseline: "without_skill",
        passRateDelta: withRate - baseRate,
        ciLow: withRate - baseRate - 0.18,
        ciHigh: withRate - baseRate + 0.16,
        pValue: verdict === "neutral" ? 0.61 : 0.012,
        tokensDelta: 12800,
        durationDeltaMs: 22_000,
        verdict,
      },
    ],
    expectations: [
      { caseId: 1, caseName: "extract-invoice-tables", text: "A CSV with the invoice line items exists", byArm: { with_skill: 1, without_skill: 0.33 }, pattern: "only_with_skill" },
      { caseId: 1, caseName: "extract-invoice-tables", text: "Totals match the PDF (1,284.50)", byArm: { with_skill: 1, without_skill: 0.33 }, pattern: "only_with_skill" },
      { caseId: 1, caseName: "extract-invoice-tables", text: "The script runs without errors", byArm: { with_skill: 1, without_skill: 1 }, pattern: "always_passes" },
      { caseId: 2, caseName: "merge-two-pdfs", text: "merged.pdf has 7 pages", byArm: { with_skill: 1, without_skill: 0.67 }, pattern: "mixed" },
      { caseId: 2, caseName: "merge-two-pdfs", text: "Page order is preserved", byArm: { with_skill: 0.67, without_skill: 0.67 }, pattern: "flaky" },
      { caseId: 3, caseName: "fill-form", text: "All 12 fields are filled", byArm: { with_skill: 0.67, without_skill: 0 }, pattern: "only_with_skill" },
      { caseId: 3, caseName: "fill-form", text: "Output is still a valid PDF", byArm: { with_skill: 1, without_skill: 1 }, pattern: "always_passes" },
    ],
    trigger: {
      total: 6,
      correct: 5,
      precision: 1,
      recall: 0.75,
      f1: 0.857,
      results: [
        { query: "Can you pull the tables out of this invoice.pdf?", shouldTrigger: true, triggered: true, chosen: skill, correct: true, raw: "{\"skill\": \"pdf\"}" },
        { query: "Merge these three PDFs into one", shouldTrigger: true, triggered: true, chosen: skill, correct: true, raw: "" },
        { query: "Summarise this contract for me (attached as PDF)", shouldTrigger: true, triggered: false, chosen: null, correct: false, raw: "" },
        { query: "Fill out the W-9 form", shouldTrigger: true, triggered: true, chosen: skill, correct: true, raw: "" },
        { query: "Write a haiku about autumn", shouldTrigger: false, triggered: false, chosen: null, correct: true, raw: "" },
        { query: "Convert this spreadsheet to a chart", shouldTrigger: false, triggered: false, chosen: "xlsx", correct: true, raw: "" },
      ],
    },
    verdict,
    notes: [
      "2 of 7 expectations pass in every arm and do not measure the skill.",
      "with_skill uses 1.45× the tokens of without_skill and takes 29% longer.",
      "\"Page order is preserved\" is flaky across repeats; tighten the assertion or the prompt.",
    ],
    generatedAt: now - day,
  };
}

function makeRun(id: string, itemName: string, status: T.RunStatus, created: number, bench: T.Benchmark | null): T.EvalRun {
  const it = byName(itemName);
  return {
    id,
    itemId: it.id,
    skillName: it.name,
    skillHash: it.contentHash,
    config: {
      itemId: it.id,
      arms: [{ label: "with_skill", kind: "with_skill" }, { label: "without_skill", kind: "without_skill" }],
      caseIds: [],
      repeats: 2,
      concurrency: 2,
      executorProviderId: "prov-claude",
      graderProviderId: "prov-anthropic",
      timeoutSecs: 600,
      includeTrigger: true,
      baselineLabel: null,
    },
    status,
    progress: status === "done" ? { total: 12, done: 12, failed: 0, message: null } : { total: 12, done: 5, failed: 0, message: "Running merge-two-pdfs (with_skill, repeat 2)" },
    workspace: `${HOME}/.skill-hive/workspaces/${it.name}/${id}`,
    summary: bench,
    error: null,
    createdAt: created,
    startedAt: created + 2000,
    finishedAt: status === "done" ? created + 11 * 60_000 : null,
  };
}

function makeCaseResults(run: T.EvalRun): T.CaseResult[] {
  const cases = [
    { id: 1, name: "extract-invoice-tables", exps: ["A CSV with the invoice line items exists", "Totals match the PDF (1,284.50)", "The script runs without errors"] },
    { id: 2, name: "merge-two-pdfs", exps: ["merged.pdf has 7 pages", "Page order is preserved"] },
    { id: 3, name: "fill-form", exps: ["All 12 fields are filled", "Output is still a valid PDF"] },
  ];
  const out: T.CaseResult[] = [];
  for (const c of cases) {
    for (const arm of ["with_skill", "without_skill"]) {
      for (const repeat of [1, 2]) {
        const expectations: T.ExpectationResult[] = c.exps.map((text, i) => {
          const passed = arm === "with_skill" ? !(c.id === 3 && i === 0 && repeat === 2) : i === c.exps.length - 1 || (c.id === 2 && repeat === 1);
          return { text, passed, evidence: passed ? `Found in transcript step ${3 + i}: output verified` : "No evidence in the transcript or outputs", source: i === 0 ? "check" : "grader" };
        });
        const passed = expectations.filter((e) => e.passed).length;
        out.push({
          id: `${run.id}-${c.id}-${arm}-${repeat}`,
          runId: run.id,
          caseId: c.id,
          caseName: c.name,
          arm,
          repeat,
          status: "graded",
          expectations,
          passed,
          failed: expectations.length - passed,
          total: expectations.length,
          passRate: passed / expectations.length,
          inputTokens: arm === "with_skill" ? 36_000 + repeat * 1000 : 24_000 + repeat * 900,
          outputTokens: arm === "with_skill" ? 5200 : 4100,
          durationMs: arm === "with_skill" ? 95_000 + c.id * 4000 : 70_000 + c.id * 6000,
          toolCalls: arm === "with_skill" ? 18 + c.id : 10 + c.id,
          costUsd: 0.07,
          outputExcerpt: arm === "with_skill" ? "I read SKILL.md and used pdfplumber to extract the tables into invoice.csv…" : "I attempted to parse the PDF with a regex over the raw bytes…",
          sandboxPath: `${run.workspace}/cases/${c.name}/${arm}/${repeat}`,
          transcriptPath: `${run.workspace}/cases/${c.name}/${arm}/${repeat}/transcript.jsonl`,
          graderNotes: null,
          error: null,
        });
      }
    }
  }
  return out;
}

function initialState(): MockState {
  const run1 = makeRun("run-1", "pdf", "done", now - day - 11 * 60_000, makeBenchmark("pdf", 0.92, 0.5, "helps_a_lot"));
  const run2 = makeRun("run-2", "xlsx", "done", now - 3 * day, makeBenchmark("xlsx", 0.8, 0.62, "helps"));
  const run3 = makeRun("run-3", "test-driven-development", "done", now - 5 * day, makeBenchmark("test-driven-development", 0.7, 0.68, "neutral"));
  const run4 = makeRun("run-4", "webapp-testing", "done", now - 9 * day, makeBenchmark("webapp-testing", 0.45, 0.6, "hurts"));
  const run0 = makeRun("run-0", "pdf", "done", now - 12 * day, makeBenchmark("pdf", 0.71, 0.48, "helps"));
  const caseResults = new Map<string, T.CaseResult[]>();
  for (const r of [run0, run1, run2, run3, run4]) caseResults.set(r.id, makeCaseResults(r));
  const suites = new Map<string, T.EvalSuite>();
  const pdf = byName("pdf");
  suites.set(pdf.id, {
    itemId: pdf.id,
    skillName: "pdf",
    path: `${pdf.path}/evals`,
    exists: true,
    cases: [
      { id: 1, name: "extract-invoice-tables", prompt: "Extract the line-item table from invoice.pdf into a CSV named invoice.csv with the same columns.", expectedOutput: "invoice.csv with 14 rows and a total of 1,284.50", files: ["evals/files/invoice.pdf"], expectations: ["A CSV with the invoice line items exists", "Totals match the PDF (1,284.50)", "The script runs without errors"], checks: [{ id: "csv", expectation: "A CSV with the invoice line items exists", kind: "file_exists", path: "invoice.csv" }] },
      { id: 2, name: "merge-two-pdfs", prompt: "Merge a.pdf and b.pdf into merged.pdf, keeping a.pdf first.", expectedOutput: "merged.pdf with 7 pages", files: ["evals/files/a.pdf", "evals/files/b.pdf"], expectations: ["merged.pdf has 7 pages", "Page order is preserved"], checks: [{ id: "merged", expectation: "merged.pdf has 7 pages", kind: "file_exists", path: "merged.pdf" }] },
      { id: 3, name: "fill-form", prompt: "Fill the attached W-9 form with the data in data.json and save it as filled.pdf.", expectedOutput: "filled.pdf with every field populated", files: ["evals/files/w9.pdf", "evals/files/data.json"], expectations: ["All 12 fields are filled", "Output is still a valid PDF"], checks: [] },
    ],
    triggerCases: [
      { query: "Can you pull the tables out of this invoice.pdf?", shouldTrigger: true },
      { query: "Merge these three PDFs into one", shouldTrigger: true },
      { query: "Summarise this contract for me (attached as PDF)", shouldTrigger: true },
      { query: "Fill out the W-9 form", shouldTrigger: true },
      { query: "Write a haiku about autumn", shouldTrigger: false },
      { query: "Convert this spreadsheet to a chart", shouldTrigger: false },
    ],
  });

  return {
    deployments: [
      deployment("pdf", "claude-code"),
      deployment("pdf", "codex", "copy", "stale"),
      deployment("xlsx", "claude-code"),
      deployment("skill-creator", "claude-code"),
      deployment("systematic-debugging", "claude-code"),
      deployment("code-reviewer", "claude-code", "copy", "in_sync"),
    ],
    loadouts: [
      { id: "lo-1", name: "Document work", description: "Everything for reports, decks and spreadsheets.", itemIds: [byName("pdf").id, byName("xlsx").id, byName("docx").id, byName("pptx").id], color: "#d97706", createdAt: now - 6 * day, updatedAt: now - day },
      { id: "lo-2", name: "Engineering baseline", description: "Debugging, TDD and commits for every repo.", itemIds: [byName("systematic-debugging").id, byName("test-driven-development").id, byName("conventional-commits").id, byName("code-reviewer").id], color: "#2563eb", createdAt: now - 10 * day, updatedAt: now - 2 * day },
    ],
    providers: [
      { id: "prov-claude", name: "Claude Code", kind: "claude_code", model: "claude-sonnet-4-5", baseUrl: null, command: "claude", args: [], env: {}, apiKeyEnv: null, hasSecret: false, roles: ["executor", "grader", "recommender"], timeoutSecs: 600, maxConcurrency: 2, lastProbe: { ok: true, latencyMs: 820, message: "claude 2.1.45", version: "2.1.45", checkedAt: now - 3600_000 }, createdAt: now - 10 * day, updatedAt: now - day },
      { id: "prov-anthropic", name: "Anthropic API", kind: "anthropic", model: "claude-sonnet-4-5", baseUrl: "https://api.anthropic.com", command: null, args: [], env: {}, apiKeyEnv: null, hasSecret: true, roles: ["grader", "recommender"], timeoutSecs: 120, maxConcurrency: 4, lastProbe: { ok: true, latencyMs: 640, message: "ok", version: null, checkedAt: now - 7200_000 }, createdAt: now - 10 * day, updatedAt: now - 2 * day },
      { id: "prov-codex", name: "Codex CLI", kind: "codex", model: null, baseUrl: null, command: "codex", args: [], env: {}, apiKeyEnv: null, hasSecret: false, roles: ["executor"], timeoutSecs: 600, maxConcurrency: 2, lastProbe: null, createdAt: now - 8 * day, updatedAt: now - 8 * day },
    ],
    settings: {
      home: `${HOME}/.skill-hive`,
      defaultLinkMode: "link",
      defaultDescriptionBudgetChars: 15000,
      defaultExecutorId: "prov-claude",
      defaultGraderId: "prov-anthropic",
      extraScanDirs: [`${HOME}/dev/team-skills`],
      githubTokenSet: false,
      theme: "system",
      overlapThreshold: 0.6,
    },
    projects: [{ path: `${HOME}/dev/acme-api`, name: "acme-api", addedAt: now - 5 * day }],
    runs: [run1, run2, run3, run4, run0],
    caseResults,
    suites,
    unmanaged: {
      "claude-code": [
        { kind: "skill", name: "my-scratch-skill", description: "Personal notes on deploying the staging stack", path: `${HOME}/.claude/skills/my-scratch-skill`, contentHash: "ffee", descriptionTokens: 11, libraryItemId: null, deploymentId: null, state: "unmanaged" },
        { kind: "skill", name: "git-commit-helper", description: byName("git-commit-helper").description, path: `${HOME}/.claude/skills/git-commit-helper`, contentHash: byName("git-commit-helper").contentHash, descriptionTokens: byName("git-commit-helper").descriptionTokens, libraryItemId: byName("git-commit-helper").id, deploymentId: null, state: "unmanaged" },
      ],
    },
  };
}

function tokensOf(text: string): string[] {
  return text.toLowerCase().split(/[^a-z0-9]+/).filter((t) => t.length > 2);
}

function cosine(a: string, b: string): number {
  const ta = tokensOf(a);
  const tb = tokensOf(b);
  const sa = new Set(ta);
  const sb = new Set(tb);
  let shared = 0;
  for (const t of sa) if (sb.has(t)) shared += 1;
  return shared / Math.sqrt(sa.size * sb.size || 1);
}

function conflictsFor(list: { id?: string | null; name: string; description: string }[], budgetChars = 15000): T.ConflictReport {
  const conflicts: T.Conflict[] = [];
  for (let i = 0; i < list.length; i++) {
    for (let j = i + 1; j < list.length; j++) {
      const sim = cosine(list[i].description, list[j].description);
      if (list[i].name === list[j].name) {
        conflicts.push({ kind: "name_collision", severity: "high", itemIds: [list[i].id, list[j].id].filter(Boolean) as string[], names: [list[i].name, list[j].name], detail: `Two skills are named \`${list[i].name}\``, similarity: null, sharedTerms: [] });
      } else if (sim >= 0.6) {
        conflicts.push({ kind: "description_overlap", severity: sim > 0.8 ? "high" : "medium", itemIds: [list[i].id, list[j].id].filter(Boolean) as string[], names: [list[i].name, list[j].name], detail: `Descriptions are ${Math.round(sim * 100)}% similar; the agent will not reliably pick the right one`, similarity: sim, sharedTerms: tokensOf(list[i].description).filter((t) => tokensOf(list[j].description).includes(t)).slice(0, 5) });
      }
    }
  }
  const chars = list.reduce((n, x) => n + x.name.length + x.description.length + 4, 0);
  if (chars > budgetChars) conflicts.push({ kind: "budget_exceeded", severity: "high", itemIds: [], names: [], detail: `Descriptions total ${chars} characters against a budget of ${budgetChars}`, similarity: chars / budgetChars, sharedTerms: [] });
  const score = Math.min(1, conflicts.reduce((s, c) => s + (c.severity === "high" ? 1 : c.severity === "medium" ? 0.5 : 0.2), 0) / Math.max(1, list.length));
  return { conflicts, score };
}

function budgetFor(list: { name: string; description: string; tokenEstimate?: number }[], budgetChars = 15000): T.BudgetSummary {
  const descriptionChars = list.reduce((n, x) => n + x.name.length + x.description.length + 4, 0);
  const descriptionTokens = Math.ceil(descriptionChars / 4);
  return { descriptionTokens, descriptionChars, bodyTokens: list.reduce((n, x) => n + (x.tokenEstimate ?? 0), 0), budgetChars, usedFraction: descriptionChars / budgetChars, overBudget: descriptionChars > budgetChars };
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function createMockBackend(): Backend {
  const state = initialState();
  const listeners = new Set<(e: T.HiveEvent) => void>();
  const emit = (e: T.HiveEvent) => listeners.forEach((l) => l(e));
  function targetStatus(targetId: string, scope: T.Scope, projectPath: string | null): T.TargetStatus {
    const spec = targetSpecs.find((t) => t.id === targetId);
    if (!spec) throw new ApiError("not_found", `target ${targetId}`);
    const deps = state.deployments.filter((d) => d.targetId === targetId && d.scope === scope && (d.projectPath ?? null) === projectPath);
    const assets: T.InstalledAsset[] = deps.map((d) => {
      const it = byId.get(d.itemId)!;
      return { kind: it.kind, name: it.name, description: it.description, path: d.path, contentHash: it.contentHash, descriptionTokens: it.descriptionTokens, libraryItemId: it.id, deploymentId: d.id, state: d.state };
    });
    if (scope === "user") assets.push(...(state.unmanaged[targetId] ?? []));
    const chars = assets.reduce((n, a) => n + a.name.length + a.description.length + 4, 0);
    return {
      spec,
      installed: ["claude-code", "codex", "cursor"].includes(targetId),
      scope,
      projectPath,
      skillsPath: scope === "user" ? `${HOME}/${spec.userSkillsDir}` : `${projectPath}/${spec.projectSkillsDir}`,
      assets,
      contextTokens: assets.reduce((n, a) => n + a.descriptionTokens, 0),
      contextChars: chars,
      budgetUsed: spec.descriptionBudgetChars ? chars / spec.descriptionBudgetChars : null,
      conflicts: conflictsFor(assets.map((a) => ({ id: a.libraryItemId, name: a.name, description: a.description }))),
    };
  }

  function detail(id: string): T.LibraryItemDetail {
    const it = byId.get(id);
    if (!it) throw new ApiError("not_found", `item ${id}`);
    const files: T.FileEntry[] = it.kind === "skill" ? [{ path: "SKILL.md", sizeBytes: it.body.length + 200, isDir: false }, { path: "scripts", sizeBytes: 0, isDir: true }, { path: "scripts/extract.py", sizeBytes: 1830, isDir: false }, { path: "reference.md", sizeBytes: 9100, isDir: false }] : [{ path: `${it.name}.md`, sizeBytes: it.body.length + 100, isDir: false }];
    const findings: T.Finding[] = [];
    if (it.validation === "warnings") findings.push({ code: "description.no_trigger", severity: "warning", message: "Consider stating when the skill should be used (\"Use when …\")", field: "description" });
    if (it.validation === "errors") findings.push({ code: "body.empty", severity: "error", message: "The skill has no instructions after the frontmatter", field: null });
    const history: T.HistoryPoint[] = state.runs
      .filter((r) => r.itemId === id && r.status === "done" && r.summary)
      .sort((a, b) => (a.finishedAt ?? 0) - (b.finishedAt ?? 0))
      .map((r) => {
        const w = r.summary!.arms.find((a) => a.label === "with_skill")!;
        const b = r.summary!.arms.find((a) => a.label === r.summary!.baseline)!;
        return { runId: r.id, finishedAt: r.finishedAt ?? r.createdAt, skillHash: r.skillHash, executor: r.summary!.executor, verdict: r.summary!.verdict, withSkillPassRate: w.passRate.mean, baselinePassRate: b.passRate.mean, delta: w.passRate.mean - b.passRate.mean, withSkillTokens: w.tokens.mean, baselineTokens: b.tokens.mean };
      });
    return {
      item: stripBody(it),
      body: it.body,
      files,
      validation: { findings },
      deployments: state.deployments.filter((d) => d.itemId === id),
      loadouts: state.loadouts.filter((l) => l.itemIds.includes(id)).map((l) => ({ id: l.id, name: l.name })),
      history,
    };
  }

  function stripBody(it: T.LibraryItem & { body?: string }): T.LibraryItem {
    const copy = { ...it } as T.LibraryItem & { body?: string };
    delete copy.body;
    copy.deploymentCount = state.deployments.filter((d) => d.itemId === it.id).length;
    return copy;
  }

  function listItems(q: T.LibraryQuery): T.LibraryItem[] {
    let list = items.slice();
    if (q.kinds?.length) list = list.filter((i) => q.kinds.includes(i.kind));
    if (q.tags?.length) list = list.filter((i) => q.tags.every((t) => i.tags.includes(t)));
    if (q.sourceKinds?.length) list = list.filter((i) => q.sourceKinds.includes(i.source.kind));
    if (q.favoritesOnly) list = list.filter((i) => i.favorite);
    if (q.deployedTo) list = list.filter((i) => state.deployments.some((d) => d.itemId === i.id && d.targetId === q.deployedTo));
    const text = q.text?.trim().toLowerCase();
    if (text) {
      list = list
        .map((i) => ({ i, s: (i.name.includes(text) ? 3 : 0) + cosine(text, `${i.name} ${i.description} ${i.tags.join(" ")}`) }))
        .filter((x) => x.s > 0)
        .sort((a, b) => b.s - a.s)
        .map((x) => x.i);
    } else if (q.sort === "updated") list.sort((a, b) => b.updatedAt - a.updatedAt);
    else if (q.sort === "effectiveness") list.sort((a, b) => (b.effectiveness?.delta ?? -9) - (a.effectiveness?.delta ?? -9));
    else list.sort((a, b) => a.name.localeCompare(b.name));
    if (q.limit) list = list.slice(0, q.limit);
    return list.map(stripBody);
  }

  function stats(): T.LibraryStats {
    const tagCounts = new Map<string, number>();
    for (const i of items) for (const t of i.tags) tagCounts.set(t, (tagCounts.get(t) ?? 0) + 1);
    return {
      total: items.length,
      skills: items.filter((i) => i.kind === "skill").length,
      agents: items.filter((i) => i.kind === "agent").length,
      commands: items.filter((i) => i.kind === "command").length,
      deployed: new Set(state.deployments.map((d) => d.itemId)).size,
      tested: items.filter((i) => i.effectiveness).length,
      withErrors: items.filter((i) => i.validation === "errors").length,
      tags: [...tagCounts.entries()].map(([tag, count]) => ({ tag, count })).sort((a, b) => b.count - a.count),
    };
  }

  function candidate(name: string, description: string, path: string, status: T.CandidateStatus, source: T.SourceRef, kind: T.AssetKind = "skill"): T.Candidate {
    return { id: nextId("cand"), kind, name, description, path, contentHash: Math.random().toString(16).slice(2), sizeBytes: 4000, tokenEstimate: 900, source, validation: { findings: status === "invalid" ? [{ code: "name.missing", severity: "error", message: "`name` is required", field: "name" }] : [] }, status, libraryItemId: status === "in_library" || status === "modified" || status === "managed" ? items.find((i) => i.name === name)?.id ?? null : null };
  }

  async function simulateRun(run: T.EvalRun) {
    const results: T.CaseResult[] = [];
    state.caseResults.set(run.id, results);
    const all = makeCaseResults(run);
    run.status = "running";
    run.startedAt = Date.now();
    run.progress = { total: all.length, done: 0, failed: 0, message: "Preparing sandboxes" };
    emit({ type: "run_updated", run_id: run.id, status: run.status, progress: run.progress });
    const current = () => (run as { status: T.RunStatus }).status;
    for (const r of all) {
      if (current() === "cancelled") return;
      await sleep(700);
      results.push(r);
      run.progress = { total: all.length, done: results.length, failed: 0, message: `Graded ${r.caseName} (${r.arm}, repeat ${r.repeat})` };
      emit({ type: "case_finished", run_id: run.id, result: r });
      emit({ type: "run_updated", run_id: run.id, status: run.status, progress: run.progress });
    }
    if ((run as { status: T.RunStatus }).status === "cancelled") return;
    run.status = "done";
    run.finishedAt = Date.now();
    run.summary = makeBenchmark(run.skillName, 0.9, 0.52, "helps_a_lot");
    const it = byId.get(run.itemId!);
    if (it) it.effectiveness = { runId: run.id, verdict: run.summary.verdict, withSkillPassRate: 0.9, baselinePassRate: 0.52, delta: 0.38, finishedAt: run.finishedAt };
    emit({ type: "run_updated", run_id: run.id, status: run.status, progress: run.progress });
    emit({ type: "library_changed" });
  }

  const handlers: Record<string, (args: Record<string, unknown>) => unknown | Promise<unknown>> = {
    app_info: () => ({ version: "0.1.0", platform: "mock", arch: "wasm" }),
    dashboard: () => ({
      stats: stats(),
      targets: ["claude-code", "codex", "cursor"].map((t) => targetStatus(t, "user", null)),
      recentRuns: state.runs.slice(0, 5),
      recentItems: listItems({ text: null, kinds: [], tags: [], sourceKinds: [], favoritesOnly: false, deployedTo: null, sort: "updated", limit: 6 }),
      loadouts: state.loadouts,
      providers: state.providers,
      detectedClis: [{ kind: "claude_code", command: "claude", path: "/usr/local/bin/claude", version: "2.1.45" }, { kind: "codex", command: "codex", path: "/opt/homebrew/bin/codex", version: "0.52.0" }],
    }),
    settings_get: () => state.settings,
    settings_update: ({ patch }) => {
      const p = patch as T.SettingsPatch;
      if (p.defaultLinkMode) state.settings.defaultLinkMode = p.defaultLinkMode;
      if (p.defaultDescriptionBudgetChars) state.settings.defaultDescriptionBudgetChars = p.defaultDescriptionBudgetChars;
      if (p.defaultExecutorId !== undefined && p.defaultExecutorId !== null) state.settings.defaultExecutorId = p.defaultExecutorId;
      if (p.defaultGraderId !== undefined && p.defaultGraderId !== null) state.settings.defaultGraderId = p.defaultGraderId;
      if (p.extraScanDirs) state.settings.extraScanDirs = p.extraScanDirs;
      if (p.githubToken !== undefined && p.githubToken !== null) state.settings.githubTokenSet = p.githubToken.length > 0;
      if (p.theme) state.settings.theme = p.theme;
      if (p.overlapThreshold) state.settings.overlapThreshold = p.overlapThreshold;
      return state.settings;
    },
    doctor: () => ({ home: state.settings.home, dbOk: true, libraryItems: items.length, orphanDirs: [], missingDirs: [], brokenDeployments: [], gitAvailable: true, detectedClis: [{ kind: "claude_code", command: "claude", path: "/usr/local/bin/claude", version: "2.1.45" }], notes: ["secrets backend: os-keychain"] }),

    library_list: ({ query }) => listItems(query as T.LibraryQuery),
    library_stats: () => stats(),
    library_get: ({ id }) => detail(id as string),
    library_read_file: ({ id, path }) => {
      const it = byId.get(id as string);
      if (!it) throw new ApiError("not_found", "item");
      if ((path as string).endsWith("SKILL.md") || (path as string).endsWith(".md")) return `---\nname: ${it.name}\ndescription: ${it.description}\n---\n\n${it.body}`;
      if ((path as string).endsWith(".py")) return "#!/usr/bin/env python3\nimport sys, pdfplumber\n\nwith pdfplumber.open(sys.argv[1]) as pdf:\n    for page in pdf.pages:\n        print(page.extract_text())\n";
      return "(binary or large file)";
    },
    library_import: async ({ request }) => {
      await sleep(600);
      const req = request as T.ImportRequest;
      const imported: T.LibraryItem[] = [];
      const skipped: T.ImportSkip[] = [];
      for (const p of req.paths) {
        const name = (p.split("/").pop() ?? "imported").replace(/\.md$/, "");
        const existing = items.find((i) => i.name === name);
        if (existing && !req.overwrite) {
          skipped.push({ path: p, reason: `'${name}' is already in the library (enable overwrite to replace it)` });
          continue;
        }
        if (existing) {
          items.splice(items.indexOf(existing), 1);
          byId.delete(existing.id);
        }
        const unmanagedHit = Object.values(state.unmanaged).flat().find((a) => a.path === p);
        const kind: T.AssetKind = unmanagedHit?.kind ?? (p.includes("/agents/") ? "agent" : p.includes("/commands/") ? "command" : "skill");
        const description = unmanagedHit?.description || `Imported from ${p}. Use when working on ${name.replace(/-/g, " ")}.`;
        const it = item({ name, kind, description, source: { ...req.source, locator: req.source.kind === "agent_dir" ? p : req.source.locator }, tags: req.tags }, "# Imported\n");
        items.push(it);
        byId.set(it.id, it);
        imported.push(stripBody(it));
        if (unmanagedHit) unmanagedHit.libraryItemId = it.id;
      }
      emit({ type: "library_changed" });
      if (imported.length) emit({ type: "deployments_changed" });
      return { imported, skipped } satisfies T.ImportResult;
    },
    library_remove: ({ id }) => {
      const idx = items.findIndex((i) => i.id === id);
      if (idx < 0) throw new ApiError("not_found", "item");
      const [removed] = items.splice(idx, 1);
      byId.delete(removed.id);
      state.deployments = state.deployments.filter((d) => d.itemId !== removed.id);
      emit({ type: "library_changed" });
      emit({ type: "deployments_changed" });
      return stripBody(removed);
    },
    library_set_tags: ({ id, tags }) => {
      const it = byId.get(id as string)!;
      it.tags = (tags as string[]).map((t) => t.trim().toLowerCase()).filter(Boolean);
      return stripBody(it);
    },
    library_set_favorite: ({ id, favorite }) => {
      const it = byId.get(id as string)!;
      it.favorite = favorite as boolean;
      return stripBody(it);
    },
    library_set_notes: ({ id, notes }) => {
      const it = byId.get(id as string)!;
      it.notes = (notes as string | null) || null;
      return stripBody(it);
    },
    library_create_skill: ({ name, description, body, tags }) => {
      if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(name as string)) throw new ApiError("invalid", "name must be 1–64 lowercase letters, digits or single hyphens");
      const it = item({ name: name as string, description: description as string, tags: tags as string[], source: src("authored", "") }, body as string);
      items.push(it);
      byId.set(it.id, it);
      emit({ type: "library_changed" });
      return stripBody(it);
    },
    library_write_document: ({ id, text }) => {
      const it = byId.get(id as string)!;
      const m = /^---\n([\s\S]*?)\n---\n?([\s\S]*)$/.exec(text as string);
      if (!m) throw new ApiError("invalid", "SKILL.md has no frontmatter (expected `---` on the first line)");
      const desc = /description:\s*(.*)/.exec(m[1])?.[1];
      if (desc) it.description = desc.trim();
      it.body = m[2];
      it.updatedAt = Date.now();
      it.contentHash = Math.random().toString(16).slice(2);
      emit({ type: "library_changed" });
      return stripBody(it);
    },
    library_refresh: ({ id }) => stripBody(byId.get(id as string)!),
    library_analyze: ({ ids, targetId }) =>
      conflictsFor(
        (ids as string[]).map((id) => byId.get(id)!).filter(Boolean),
        targetSpecs.find((t) => t.id === targetId)?.descriptionBudgetChars ?? state.settings.defaultDescriptionBudgetChars,
      ),

    scan_local: async () => {
      await sleep(900);
      const agentSrc = (p: string): T.SourceRef => ({ ...src("agent_dir", p), targetId: "claude-code" });
      return {
        candidates: [
          candidate("git-commit-helper", byName("git-commit-helper").description, `${HOME}/.claude/skills/git-commit-helper`, "in_library", agentSrc(`${HOME}/.claude/skills/git-commit-helper`)),
          candidate("my-scratch-skill", "Personal notes on deploying the staging stack", `${HOME}/.claude/skills/my-scratch-skill`, "new", agentSrc(`${HOME}/.claude/skills/my-scratch-skill`)),
          candidate("pdf", byName("pdf").description, `${HOME}/.claude/skills/pdf`, "managed", agentSrc(`${HOME}/.claude/skills/pdf`)),
          candidate("terraform-review", "Review Terraform plans for risky changes before apply. Use when the user shares a plan output.", `${HOME}/.cursor/skills/terraform-review`, "new", { ...src("agent_dir", `${HOME}/.cursor/skills/terraform-review`), targetId: "cursor" }),
          candidate("release-notes", "Draft release notes from merged PRs since the last tag.", `${HOME}/.codex/skills/release-notes`, "new", { ...src("agent_dir", `${HOME}/.codex/skills/release-notes`), targetId: "codex" }),
          candidate("xlsx", `${byName("xlsx").description} Also converts CSV files.`, `${HOME}/.codex/skills/xlsx`, "modified", { ...src("agent_dir", `${HOME}/.codex/skills/xlsx`), targetId: "codex" }),
          candidate("broken", "", `${HOME}/dev/team-skills/broken`, "invalid", src("local_path", `${HOME}/dev/team-skills/broken`)),
          candidate("security-auditor", "Audits dependencies and configs for known vulnerabilities. Delegate for security reviews.", `${HOME}/.claude/agents/security-auditor.md`, "new", agentSrc(`${HOME}/.claude/agents/security-auditor.md`), "agent"),
        ],
        rootsScanned: [`${HOME}/.claude/skills`, `${HOME}/.claude/agents`, `${HOME}/.codex/skills`, `${HOME}/.cursor/skills`, `${HOME}/dev/team-skills`],
        errors: [`${HOME}/dev/team-skills/broken/SKILL.md: frontmatter is not closed`],
        durationMs: 412,
      } satisfies T.ScanReport;
    },
    scan_paths: async ({ paths }) => {
      await sleep(500);
      const p = (paths as string[])[0] ?? "/picked";
      return { candidates: [candidate("picked-skill", "A skill found in the folder you picked. Use when exploring.", `${p}/picked-skill`, "new", src("local_path", p))], rootsScanned: paths as string[], errors: [], durationMs: 40 } satisfies T.ScanReport;
    },
    source_parse: ({ input }) => parseSpec(input as string),
    source_preview: async ({ input }) => {
      await sleep(1100);
      const spec = parseSpec(input as string);
      const base = spec.locator;
      const repo = (sub: string): T.SourceRef => ({ kind: "github", locator: base, subpath: sub, reference: "main", revision: "ca45fc496a34", targetId: null, fetchedAt: Date.now() });
      const cands = base.includes("superpowers")
        ? [candidate("brainstorming", "Turn a vague idea into a design through structured questions. Use before writing a plan.", `/cache/${base}/skills/brainstorming`, "new", repo("skills/brainstorming")), candidate("systematic-debugging", byName("systematic-debugging").description, `/cache/${base}/skills/systematic-debugging`, "in_library", repo("skills/systematic-debugging")), candidate("writing-plans", "Write an implementation plan with bounded tasks and checks. Use when a change spans several files.", `/cache/${base}/skills/writing-plans`, "new", repo("skills/writing-plans"))]
        : [candidate("pdf", byName("pdf").description, `/cache/${base}/skills/pdf`, "in_library", repo("skills/pdf")), candidate("mcp-builder", "Build MCP servers that expose tools to agents. Use when the user wants to integrate an API as tools.", `/cache/${base}/skills/mcp-builder`, "new", repo("skills/mcp-builder")), candidate("frontend-design", "Design distinctive, production-grade web interfaces. Use when building UI from scratch.", `/cache/${base}/skills/frontend-design`, "new", repo("skills/frontend-design")), candidate("canvas-design", "Create visual art and posters on a canvas. Use for poster or artwork requests.", `/cache/${base}/skills/canvas-design`, "new", repo("skills/canvas-design")), candidate("broken-example", "", `/cache/${base}/skills/broken`, "invalid", repo("skills/broken"))];
      return { spec, revision: "ca45fc496a34", candidates: cands, readme: `# ${base}\n\nA collection of agent skills.\n\n- Skills live under \`skills/\`.\n- Each has a \`SKILL.md\`.\n`, warnings: cands.some((c) => c.status === "invalid") ? ["1 item(s) fail validation and will be skipped on import"] : [] } satisfies T.SourcePreview;
    },
    source_import: async ({ input, paths, tags }) => {
      await sleep(800);
      const spec = parseSpec(input as string);
      const list = (paths as string[] | null) ?? ["/cache/x/skills/mcp-builder"];
      const imported: T.LibraryItem[] = [];
      for (const p of list) {
        const name = p.split("/").pop() ?? "imported";
        if (items.some((i) => i.name === name)) continue;
        const it = item({ name, description: `Imported from ${spec.locator}. Use when the task involves ${name.replace(/-/g, " ")}.`, source: { kind: spec.kind, locator: spec.locator, subpath: `skills/${name}`, reference: "main", revision: "ca45fc496a34", targetId: null, fetchedAt: Date.now() }, tags: tags as string[] }, `# ${name}\n\nImported body.\n`);
        items.push(it);
        byId.set(it.id, it);
        imported.push(stripBody(it));
      }
      emit({ type: "library_changed" });
      return { imported, skipped: [] } satisfies T.ImportResult;
    },
    source_check_updates: async ({ ids }) => {
      await sleep(700);
      const list = (ids as string[] | null)?.map((i) => byId.get(i)!).filter(Boolean) ?? items.filter((i) => i.source.kind === "github");
      return list.map((i) => ({ itemId: i.id, currentRevision: i.source.revision ?? null, latestRevision: i.name === "pdf" ? "9f0e1d2c3b4a" : (i.source.revision ?? null), updateAvailable: i.name === "pdf", contentChanged: false, error: null } satisfies T.UpdateCheck));
    },
    source_update_item: async ({ id }) => {
      await sleep(900);
      const it = byId.get(id as string)!;
      it.source = { ...it.source, revision: "9f0e1d2c3b4a", fetchedAt: Date.now() };
      it.updatedAt = Date.now();
      it.contentHash = Math.random().toString(16).slice(2);
      emit({ type: "library_changed" });
      emit({ type: "deployments_changed" });
      return stripBody(it);
    },
    catalog_list: () => CATALOG,
    github_search: async ({ query }) => {
      await sleep(600);
      const q = (query as string).toLowerCase();
      if (q.includes("ratelimit")) throw new ApiError("rate_limited", "GitHub API rate limit exceeded (60 requests per hour without a token). Add a GitHub token in Settings to raise the limit, or try again in a few minutes.");
      return CATALOG.filter((c) => !q || `${c.name} ${c.description} ${c.tags.join(" ")}`.toLowerCase().includes(q)).map((c, i) => ({ fullName: c.source, description: c.description, url: c.homepage ?? "", stars: 12000 - i * 900, topics: c.tags, updatedAt: new Date(now - i * day).toISOString() } satisfies T.RepoSearchResult));
    },
    source_clear_cache: () => 184_000_000,

    targets_list: () => targetSpecs,
    targets_installed: () => ["claude-code", "codex", "cursor"].map((t) => targetStatus(t, "user", null)),
    target_status: ({ targetId, scope, projectPath }) => targetStatus(targetId as string, scope as T.Scope, (projectPath as string | null) ?? null),
    target_add_custom: ({ spec }) => {
      const s = { ...(spec as T.TargetSpec), custom: true };
      if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(s.id)) throw new ApiError("invalid", "id must be lowercase letters, digits and single hyphens");
      if (targetSpecs.some((t) => t.id === s.id)) throw new ApiError("already_exists", `target '${s.id}' already exists`);
      targetSpecs.push(s);
      emit({ type: "deployments_changed" });
      return s;
    },
    target_remove_custom: ({ id }) => {
      const i = targetSpecs.findIndex((t) => t.id === id && t.custom);
      if (i >= 0) targetSpecs.splice(i, 1);
      emit({ type: "deployments_changed" });
    },
    target_set_skills_dir: ({ id, dir }) => {
      const spec = targetSpecs.find((t) => t.id === id);
      if (!spec) throw new ApiError("not_found", `target ${id}`);
      if (!defaultSkillsDirs.has(spec.id)) defaultSkillsDirs.set(spec.id, spec.userSkillsDir);
      spec.userSkillsDir = (dir as string | null) ?? defaultSkillsDirs.get(spec.id) ?? spec.userSkillsDir;
    },
    projects_list: () => state.projects,
    project_add: ({ path }) => {
      const p = { path: path as string, name: (path as string).split("/").pop() ?? "project", addedAt: Date.now() };
      state.projects.push(p);
      return p;
    },
    project_remove: ({ path }) => {
      state.projects = state.projects.filter((p) => p.path !== path);
    },
    deploy: async ({ request }) => {
      await sleep(400);
      const req = request as T.DeployRequest;
      const deployed: T.Deployment[] = [];
      const failed: T.DeployFailure[] = [];
      for (const id of req.itemIds) {
        const it = byId.get(id);
        if (!it) {
          failed.push({ itemId: id, reason: "not found" });
          continue;
        }
        const spec = targetSpecs.find((t) => t.id === req.targetId)!;
        if (it.kind !== "skill" && !spec.userAgentsDir) {
          failed.push({ itemId: id, reason: `${spec.name} has no ${it.kind} directory at ${req.scope} scope` });
          continue;
        }
        const existing = state.deployments.find((d) => d.itemId === id && d.targetId === req.targetId && d.scope === req.scope && (d.projectPath ?? null) === (req.projectPath ?? null));
        if (existing) {
          deployed.push(existing);
          continue;
        }
        const d = deployment(it.name, req.targetId, req.mode ?? state.settings.defaultLinkMode, (req.mode ?? state.settings.defaultLinkMode) === "link" ? "linked" : "in_sync");
        d.scope = req.scope;
        d.projectPath = req.projectPath ?? null;
        if (req.scope === "project") d.path = `${req.projectPath}/${spec.projectSkillsDir}/${it.name}`;
        state.deployments.push(d);
        deployed.push(d);
      }
      emit({ type: "deployments_changed" });
      return { deployed, failed } satisfies T.DeployResult;
    },
    undeploy: ({ deploymentId }) => {
      const i = state.deployments.findIndex((d) => d.id === deploymentId);
      if (i < 0) throw new ApiError("not_found", "deployment");
      const [d] = state.deployments.splice(i, 1);
      emit({ type: "deployments_changed" });
      return d;
    },
    undeploy_item: ({ itemId }) => {
      const removed = state.deployments.filter((d) => d.itemId === itemId);
      state.deployments = state.deployments.filter((d) => d.itemId !== itemId);
      emit({ type: "deployments_changed" });
      return removed;
    },
    resync: async ({ deploymentId }) => {
      await sleep(300);
      const d = state.deployments.find((x) => x.id === deploymentId)!;
      d.state = d.mode === "link" ? "linked" : "in_sync";
      d.updatedAt = Date.now();
      emit({ type: "deployments_changed" });
      return d;
    },
    deployments_list: () => state.deployments,
    remove_installed_path: ({ path }) => {
      for (const k of Object.keys(state.unmanaged)) state.unmanaged[k] = state.unmanaged[k].filter((a) => a.path !== path);
      emit({ type: "deployments_changed" });
    },

    loadouts_list: () => state.loadouts,
    loadout_get: ({ id }) => state.loadouts.find((l) => l.id === id) ?? (() => { throw new ApiError("not_found", "loadout"); })(),
    loadout_create: ({ input }) => {
      const i = input as T.LoadoutInput;
      if (state.loadouts.some((l) => l.name.toLowerCase() === i.name.toLowerCase())) throw new ApiError("already_exists", `loadout '${i.name}'`);
      const l: T.Loadout = { id: nextId("lo"), name: i.name, description: i.description, itemIds: [...new Set(i.itemIds)], color: i.color ?? null, createdAt: Date.now(), updatedAt: Date.now() };
      state.loadouts.push(l);
      return l;
    },
    loadout_update: ({ id, input }) => {
      const l = state.loadouts.find((x) => x.id === id);
      if (!l) throw new ApiError("not_found", "loadout");
      const i = input as T.LoadoutInput;
      Object.assign(l, { name: i.name, description: i.description, itemIds: [...new Set(i.itemIds)], color: i.color ?? null, updatedAt: Date.now() });
      return l;
    },
    loadout_delete: ({ id }) => {
      state.loadouts = state.loadouts.filter((l) => l.id !== id);
    },
    loadout_plan: ({ request }) => planLoadout(request as T.ApplyLoadoutRequest),
    loadout_apply: async ({ request }) => {
      await sleep(700);
      const req = request as T.ApplyLoadoutRequest;
      const plan = planLoadout(req);
      const removed: string[] = [];
      for (const r of plan.removes) {
        const idx = state.deployments.findIndex((d) => d.path === r.path);
        if (idx >= 0) {
          removed.push(state.deployments[idx].path);
          state.deployments.splice(idx, 1);
        }
      }
      const deployed: T.Deployment[] = [];
      for (const a of plan.adds) {
        const it = byId.get(a.itemId!)!;
        const d = deployment(it.name, req.targetId, req.mode ?? state.settings.defaultLinkMode);
        d.scope = req.scope;
        d.projectPath = req.projectPath ?? null;
        state.deployments.push(d);
        deployed.push(d);
      }
      emit({ type: "deployments_changed" });
      return { plan, deployed, removed, failed: [] } satisfies T.ApplyLoadoutResult;
    },
    recommend: async ({ request }) => {
      await sleep(500);
      const req = request as T.RecommendRequest;
      const scored = items
        .filter((i) => !req.kinds.length || req.kinds.includes(i.kind))
        .filter((i) => !req.excludeIds.includes(i.id))
        .map((i) => {
          const s = cosine(req.task, `${i.name.replace(/-/g, " ")} ${i.description} ${i.tags.join(" ")}`);
          const reasons: string[] = [];
          const matched = tokensOf(req.task).filter((t) => tokensOf(`${i.name} ${i.description}`).includes(t));
          if (matched.length) reasons.push("Full-text match");
          if (tokensOf(i.name.replace(/-/g, " ")).some((t) => tokensOf(req.task).includes(t))) reasons.push(`Name matches: ${i.name}`);
          if (i.effectiveness && i.effectiveness.delta > 0.05) reasons.push(`Measured: +${Math.round(i.effectiveness.delta * 100)}% pass rate with this skill`);
          if (i.validation === "errors") reasons.push("Has validation errors");
          if (state.deployments.some((d) => d.itemId === i.id)) reasons.push("Already deployed");
          return { i, s: s + (i.effectiveness?.delta ?? 0) * 0.3 - (i.validation === "errors" ? 0.2 : 0), reasons, matched };
        })
        .filter((x) => x.s > 0.05)
        .sort((a, b) => b.s - a.s)
        .slice(0, req.limit ?? 10);
      const top = scored[0]?.s ?? 1;
      const report = conflictsFor(scored.map((x) => x.i));
      const conflictPairs = new Map<string, Set<string>>();
      for (const c of report.conflicts) for (const a of c.itemIds) for (const b of c.itemIds) if (a !== b) (conflictPairs.get(a) ?? conflictPairs.set(a, new Set()).get(a)!).add(b);
      const selected: string[] = [];
      for (const x of scored) {
        if (x.i.validation === "errors" || selected.length >= 5 || x.s < top * 0.25) continue;
        if (selected.some((sid) => conflictPairs.get(x.i.id)?.has(sid))) continue;
        selected.push(x.i.id);
      }
      return {
        results: scored.map((x) => ({ item: stripBody(x.i), score: x.s / top, reasons: x.reasons, matchedTerms: x.matched.slice(0, 8), selected: selected.includes(x.i.id), conflictsWith: [...(conflictPairs.get(x.i.id) ?? [])] })),
        suggestedIds: selected,
        conflicts: report,
        budget: budgetFor(selected.map((id) => byId.get(id)!)),
        llmNotes: req.useLlm ? "Re-ranked by Claude: the pdf skill covers extraction and the xlsx skill covers the spreadsheet output; the git skills are unrelated to this task." : null,
      } satisfies T.RecommendResponse;
    },

    providers_list: () => state.providers,
    provider_save: ({ id, input }) => {
      const i = input as T.ProviderInput;
      const existing = id ? state.providers.find((p) => p.id === id) : undefined;
      const p: T.ProviderConfig = {
        id: existing?.id ?? nextId("prov"),
        name: i.name,
        kind: i.kind,
        model: i.model ?? null,
        baseUrl: i.baseUrl ?? null,
        command: i.command ?? null,
        args: i.args ?? [],
        env: i.env ?? {},
        apiKeyEnv: i.apiKeyEnv ?? null,
        hasSecret: i.apiKey ? i.apiKey.length > 0 : (existing?.hasSecret ?? false),
        roles: i.roles,
        timeoutSecs: i.timeoutSecs ?? 600,
        maxConcurrency: i.maxConcurrency ?? 2,
        lastProbe: existing?.lastProbe ?? null,
        createdAt: existing?.createdAt ?? Date.now(),
        updatedAt: Date.now(),
      };
      if (existing) Object.assign(existing, p);
      else state.providers.push(p);
      return p;
    },
    provider_delete: ({ id }) => {
      state.providers = state.providers.filter((p) => p.id !== id);
    },
    provider_probe: async ({ id }) => {
      await sleep(900);
      const p = state.providers.find((x) => x.id === id)!;
      p.lastProbe = { ok: p.kind !== "codex", latencyMs: 540, message: p.kind === "codex" ? "codex: command not found" : "ok", version: p.kind === "claude_code" ? "2.1.45" : null, checkedAt: Date.now() };
      return p.lastProbe;
    },
    providers_detect: () => [{ kind: "claude_code", command: "claude", path: "/usr/local/bin/claude", version: "2.1.45" }],
    providers_ensure_defaults: () => state.providers,

    eval_suite_get: ({ itemId }) => {
      const it = byId.get(itemId as string);
      if (!it) throw new ApiError("not_found", "item");
      return state.suites.get(it.id) ?? ({ itemId: it.id, skillName: it.name, cases: [], triggerCases: [], path: `${it.path}/evals`, exists: false } satisfies T.EvalSuite);
    },
    eval_suite_save: ({ suite }) => {
      const s = suite as T.EvalSuite;
      const saved = { ...s, exists: true };
      state.suites.set(s.itemId, saved);
      emit({ type: "library_changed" });
      return saved;
    },
    eval_suite_draft: ({ itemId }) => {
      const it = byId.get(itemId as string)!;
      const topic = it.name.replace(/-/g, " ");
      return {
        itemId: it.id,
        skillName: it.name,
        path: `${it.path}/evals`,
        exists: false,
        cases: [
          { id: 1, name: `basic-${it.name}`, prompt: `Help me with ${topic}: ${it.description.split(".")[0].toLowerCase()}.`, expectedOutput: null, files: [], expectations: ["The response follows the skill's instructions", "The output is complete and usable"], checks: [] },
          { id: 2, name: `edge-case-${it.name}`, prompt: `I have an unusual ${topic} situation with missing inputs. What do you do?`, expectedOutput: null, files: [], expectations: ["The agent asks for or infers the missing input", "No fabricated results"], checks: [] },
        ],
        triggerCases: [
          { query: `Can you help with ${topic}?`, shouldTrigger: true },
          { query: it.description.split(".")[0], shouldTrigger: true },
          { query: "Write a haiku about autumn", shouldTrigger: false },
          { query: "What is the capital of France?", shouldTrigger: false },
        ],
      } satisfies T.EvalSuite;
    },
    eval_default_config: ({ itemId }) => ({ itemId: itemId as string, arms: [{ label: "with_skill", kind: "with_skill" }, { label: "without_skill", kind: "without_skill" }], caseIds: [], repeats: 1, concurrency: 2, executorProviderId: state.settings.defaultExecutorId ?? "prov-claude", graderProviderId: state.settings.defaultGraderId ?? null, timeoutSecs: 600, includeTrigger: true, baselineLabel: null } satisfies T.RunConfig),
    eval_run_start: ({ config }) => {
      const c = config as T.RunConfig;
      const it = byId.get(c.itemId);
      if (!it) throw new ApiError("not_found", "item");
      const run = makeRun(nextId("run"), it.name, "queued", Date.now(), null);
      run.config = c;
      run.progress = { total: 0, done: 0, failed: 0, message: "Queued" };
      state.runs.unshift(run);
      void simulateRun(run);
      return run;
    },
    eval_run_cancel: ({ runId }) => {
      const run = state.runs.find((r) => r.id === runId);
      if (run && !["done", "failed", "cancelled"].includes(run.status)) {
        run.status = "cancelled";
        run.finishedAt = Date.now();
        emit({ type: "run_updated", run_id: run.id, status: run.status, progress: run.progress });
      }
    },
    eval_runs_list: ({ itemId, limit }) => state.runs.filter((r) => !itemId || r.itemId === itemId).slice(0, (limit as number) ?? 50),
    eval_run_detail: ({ runId }) => {
      const run = state.runs.find((r) => r.id === runId);
      if (!run) throw new ApiError("not_found", "run");
      return { run, results: state.caseResults.get(run.id) ?? [] } satisfies T.EvalRunDetail;
    },
    eval_run_delete: ({ runId }) => {
      state.runs = state.runs.filter((r) => r.id !== runId);
      state.caseResults.delete(runId as string);
    },
    eval_history: ({ itemId }) => detail(itemId as string).history,
    eval_benchmark_markdown: ({ runId }) => {
      const run = state.runs.find((r) => r.id === runId);
      const b = run?.summary;
      if (!b) return "No benchmark yet.";
      return `# ${b.skillName} benchmark\n\n| Arm | Pass rate | Tokens | Duration |\n|---|---|---|---|\n${b.arms.map((a) => `| ${a.label} | ${(a.passRate.mean * 100).toFixed(0)}% ± ${(a.passRate.stddev * 100).toFixed(0)} | ${Math.round(a.tokens.mean)} | ${Math.round(a.durationMs.mean / 1000)} s |`).join("\n")}\n\nVerdict: **${b.verdict}**\n\n${b.notes.map((n) => `- ${n}`).join("\n")}\n`;
    },
    eval_case_transcript: ({ caseResultId }) => `{"type":"system","subtype":"init","cwd":"/sandbox"}\n{"type":"assistant","message":{"content":[{"type":"text","text":"I'll read the skill instructions first."},{"type":"tool_use","name":"Read","input":{"file_path":".claude/skills/pdf/SKILL.md"}}]}}\n{"type":"user","message":{"content":[{"type":"tool_result","content":"# PDF Processing Guide..."}]}}\n{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"python3 scripts/extract.py invoice.pdf > invoice.csv"}}]}}\n{"type":"result","subtype":"success","result":"Extracted 14 rows into invoice.csv (total 1,284.50).","duration_ms":94210,"usage":{"input_tokens":36120,"output_tokens":5203}}\n# case ${caseResultId}\n`,
  };

  function planLoadout(req: T.ApplyLoadoutRequest): T.LoadoutPlan {
    const l = state.loadouts.find((x) => x.id === req.loadoutId);
    if (!l) throw new ApiError("not_found", "loadout");
    const status = targetStatus(req.targetId, req.scope, req.projectPath ?? null);
    const wanted = l.itemIds.map((id) => byId.get(id)!).filter(Boolean);
    const planned = (it: T.LibraryItem, path: string | null): T.PlannedItem => ({ itemId: it.id, name: it.name, kind: it.kind, descriptionTokens: it.descriptionTokens, path });
    const adds: T.PlannedItem[] = [];
    const keeps: T.PlannedItem[] = [];
    for (const it of wanted) {
      const a = status.assets.find((x) => x.libraryItemId === it.id && x.deploymentId);
      if (a) keeps.push(planned(it, a.path));
      else adds.push(planned(it, `${status.skillsPath}/${it.name}`));
    }
    const removes: T.PlannedItem[] = req.strategy === "exclusive" ? status.assets.filter((a) => a.deploymentId && !l.itemIds.includes(a.libraryItemId ?? "")).map((a) => ({ itemId: a.libraryItemId, name: a.name, kind: a.kind, descriptionTokens: a.descriptionTokens, path: a.path })) : [];
    const unmanaged: T.PlannedItem[] = status.assets.filter((a) => !a.deploymentId).map((a) => ({ itemId: a.libraryItemId, name: a.name, kind: a.kind, descriptionTokens: a.descriptionTokens, path: a.path }));
    const resulting = [...wanted, ...status.assets.filter((a) => !removes.some((r) => r.path === a.path) && !wanted.some((w) => w.id === a.libraryItemId))];
    return { adds, removes, keeps, unmanaged, conflicts: conflictsFor(resulting.map((x) => ("id" in x ? { id: x.id, name: x.name, description: x.description } : { id: x.libraryItemId, name: x.name, description: x.description }))), budget: budgetFor(resulting) };
  }

  return {
    async call<R>(cmd: string, args: Record<string, unknown> = {}): Promise<R> {
      const handler = handlers[cmd];
      if (!handler) throw new ApiError("not_found", `mock: unknown command ${cmd}`);
      await sleep(cmd.endsWith("_list") || cmd === "dashboard" ? 120 : 40);
      return (await handler(args)) as R;
    },
    onEvent(handler) {
      listeners.add(handler);
      return () => listeners.delete(handler);
    },
  };
}

function parseSpec(input: string): T.SourceSpec {
  const raw = input.trim();
  if (!raw) throw new ApiError("invalid", "source is empty");
  const gh = /^(?:https?:\/\/)?(?:www\.)?github\.com\/([^/]+)\/([^/#?]+)(?:\/tree\/([^/]+)\/(.+))?/.exec(raw);
  if (gh) return { kind: "github", locator: `${gh[1]}/${gh[2].replace(/\.git$/, "")}`, cloneUrl: `https://github.com/${gh[1]}/${gh[2]}.git`, subpath: gh[4] ?? null, reference: gh[3] ?? null, input: raw };
  if (raw.startsWith("/") || raw.startsWith("~") || raw.startsWith(".")) return { kind: "local_path", locator: raw, cloneUrl: null, subpath: null, reference: null, input: raw };
  if (/^(git@|ssh:\/\/|https?:\/\/)/.test(raw)) return { kind: raw.endsWith(".zip") ? "url" : "git", locator: raw.replace(/^https?:\/\//, "").replace(/\.git$/, ""), cloneUrl: raw, subpath: null, reference: null, input: raw };
  const short = /^([\w.-]+)\/([\w.-]+)(?:\/(.+?))?(?:@([\w./-]+))?$/.exec(raw);
  if (short) return { kind: "github", locator: `${short[1]}/${short[2]}`, cloneUrl: `https://github.com/${short[1]}/${short[2]}.git`, subpath: short[3] ?? null, reference: short[4] ?? null, input: raw };
  throw new ApiError("invalid", `could not understand source '${raw}'. Use owner/repo, a GitHub or git URL, an archive URL or a local path`);
}

const CATALOG: T.CatalogEntry[] = [
  { id: "anthropics-skills", name: "Anthropic Skills", description: "Anthropic's official skill collection: document skills (docx, pptx, xlsx, pdf), skill-creator, mcp-builder, frontend-design, webapp-testing and more.", source: "anthropics/skills", homepage: "https://github.com/anthropics/skills", tags: ["official", "documents", "authoring"], publisher: "Anthropic", approxCount: 20 },
  { id: "openai-skills", name: "OpenAI Codex Skills", description: "OpenAI's curated and system skills for Codex: skill-creator, deploy helpers, Figma, Notion, Playwright and more.", source: "openai/skills", homepage: "https://github.com/openai/skills", tags: ["official", "codex", "integrations"], publisher: "OpenAI", approxCount: 44 },
  { id: "obra-superpowers", name: "Superpowers", description: "Engineering workflow skills: brainstorming, writing plans, TDD, systematic debugging, code review.", source: "obra/superpowers", homepage: "https://github.com/obra/superpowers", tags: ["workflow", "engineering", "tdd"], publisher: "obra", approxCount: 15 },
  { id: "vercel-agent-skills", name: "Vercel Agent Skills", description: "Skills for web development with React, Next.js and the Vercel platform.", source: "vercel-labs/agent-skills", homepage: "https://github.com/vercel-labs/agent-skills", tags: ["web", "react", "nextjs"], publisher: "Vercel", approxCount: null },
  { id: "huggingface-skills", name: "Hugging Face Skills", description: "Model training, datasets, Hub operations and evaluation.", source: "huggingface/skills", homepage: "https://github.com/huggingface/skills", tags: ["ml", "training"], publisher: "Hugging Face", approxCount: null },
  { id: "trailofbits-skills", name: "Trail of Bits Security Skills", description: "Static analysis, fuzzing, code auditing and secure development practices.", source: "trailofbits/skills", homepage: "https://github.com/trailofbits/skills", tags: ["security", "audit"], publisher: "Trail of Bits", approxCount: null },
  { id: "supabase-agent-skills", name: "Supabase Agent Skills", description: "Postgres best practices, auth, storage and edge functions.", source: "supabase/agent-skills", homepage: "https://github.com/supabase/agent-skills", tags: ["database", "postgres"], publisher: "Supabase", approxCount: null },
  { id: "microsoft-skills", name: "Microsoft Skills", description: "Azure services, developer tooling and Microsoft platforms.", source: "microsoft/skills", homepage: "https://github.com/microsoft/skills", tags: ["azure", "cloud"], publisher: "Microsoft", approxCount: null },
];
