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
const PROJECT = `${HOME}/dev/acme-api`;
const INSTRUCTION_BACKUPS = `${HOME}/.skill-hive/backups/instructions`;
/** Agents the mock machine has installed. */
const installedTargetIds = ["claude-code", "codex", "cursor"];

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

/** Where one agent looks for standing instructions (relative to the home or the project root). */
interface InstructionSpec {
  targetId: string;
  scope: T.Scope;
  rel: string;
  label: string;
  kind: T.InstructionKind;
  /** File-name suffix for members of a rules folder. */
  pattern?: string;
  note?: string;
}

const ifile = (targetId: string, scope: T.Scope, rel: string, label: string, note?: string): InstructionSpec => ({ targetId, scope, rel, label, kind: "file", note });
const idir = (targetId: string, scope: T.Scope, rel: string, label: string, pattern: string, note?: string): InstructionSpec => ({ targetId, scope, rel, label, kind: "directory", pattern, note });
const LOCAL_NOTE = "Meant for your own machine; usually kept out of git.";
const SHARED_AGENTS_NOTE = "The AGENTS.md convention: one file read by several agents.";
const LEGACY_NOTE = "Older single-file format; the rules folder is preferred.";

/** Mirrors `INSTRUCTION_SPECS` in crates/hive-core/src/instructions.rs for the agents the mock knows. */
const INSTRUCTION_SPECS: InstructionSpec[] = [
  ifile("claude-code", "user", ".claude/CLAUDE.md", "User memory", "Loaded in every project."),
  ifile("claude-code", "project", "CLAUDE.md", "Project instructions", "Committed with the project; everyone on it sees it."),
  ifile("claude-code", "project", ".claude/CLAUDE.md", "Project instructions (.claude)", "Alternative location for the project file."),
  ifile("claude-code", "project", "CLAUDE.local.md", "Local instructions", LOCAL_NOTE),
  ifile("codex", "user", ".codex/AGENTS.md", "Global instructions", "Loaded in every project."),
  ifile("codex", "project", "AGENTS.md", "Project instructions", SHARED_AGENTS_NOTE),
  ifile("gemini-cli", "user", ".gemini/GEMINI.md", "Global context", "Loaded in every project."),
  ifile("gemini-cli", "project", "GEMINI.md", "Project context"),
  idir("cursor", "project", ".cursor/rules", "Rules", ".mdc", "One rule per file; frontmatter sets when it applies."),
  ifile("cursor", "project", ".cursorrules", "Rules (legacy)", LEGACY_NOTE),
  ifile("cursor", "project", "AGENTS.md", "Project instructions", SHARED_AGENTS_NOTE),
  ifile("github-copilot", "project", ".github/copilot-instructions.md", "Repository instructions"),
  idir("github-copilot", "project", ".github/instructions", "Path-specific instructions", ".instructions.md", "Each file names the paths it applies to in its frontmatter."),
  ifile("github-copilot", "project", "AGENTS.md", "Project instructions", SHARED_AGENTS_NOTE),
  ifile("windsurf", "user", ".codeium/windsurf/memories/global_rules.md", "Global rules"),
  idir("windsurf", "project", ".windsurf/rules", "Rules", ".md"),
  ifile("windsurf", "project", ".windsurfrules", "Rules (legacy)", LEGACY_NOTE),
  ifile("opencode", "user", ".config/opencode/AGENTS.md", "Global instructions"),
  ifile("opencode", "project", "AGENTS.md", "Project instructions", SHARED_AGENTS_NOTE),
  ifile("agents-shared", "project", "AGENTS.md", "Project instructions", SHARED_AGENTS_NOTE),
];

/** Does `name` fit a rules folder's suffix and sit directly inside it? */
const memberNameOk = (name: string, pattern: string) => name.length > pattern.length && !name.startsWith(".") && !name.includes("/") && !name.includes("\\") && name.endsWith(pattern);

const USER_CLAUDE_MD = `# Preferences

## Working style

- Ask before making changes that touch more than a handful of files.
- Prefer small, reviewable commits with a one-line summary and a short body.
- When a task is ambiguous, state the assumption you are making and carry on.

## Code

- TypeScript strict mode everywhere; no \`any\` without a comment saying why.
- Rust: run \`cargo clippy --all-targets\` before declaring something done.
- Tests next to the code they cover; name them after the behaviour, not the function.

## Tools

- Use \`rg\` rather than \`grep\`, and \`fd\` rather than \`find\`.
- Package managers: \`npm\` for JS, \`cargo\` for Rust, \`uv\` for Python.

## Communication

- Short answers. Lead with the conclusion, then the evidence.
- No emojis in commit messages, comments or docs.
- British spelling in prose; American spelling in identifiers.
`;

const PROJECT_CLAUDE_MD = `# acme-api

REST API for the Acme storefront: Rust (axum) with Postgres.

## Commands

- \`cargo run\` starts the API on :8080 against the dev database.
- \`cargo test\` runs unit and integration tests (needs \`docker compose up db\`).
- \`sqlx migrate run\` applies migrations.

## Conventions

- Handlers live in \`src/routes/\`, one module per resource.
- Errors go through \`ApiError\`; never return a bare string.
- Every endpoint gets an integration test in \`tests/\`.
`;

/** Instruction files on the mock machine's disk: absolute path, text and modification time. */
const INSTRUCTION_SEED: [path: string, text: string, modifiedAt: number][] = [
  [`${HOME}/.claude/CLAUDE.md`, USER_CLAUDE_MD, now - 2 * day],
  [`${HOME}/.codex/AGENTS.md`, "# Global instructions\n\nRun the test suite before reporting a task as done. Prefer editing existing files to creating new ones.\n", now - 20 * day],
  [`${PROJECT}/CLAUDE.md`, PROJECT_CLAUDE_MD, now - 4 * day],
  [`${PROJECT}/AGENTS.md`, "# Agents\n\nRead CLAUDE.md for the project conventions. Keep diffs small and run `cargo test` before finishing.\n", now - 6 * day],
  [`${PROJECT}/.cursor/rules/style.mdc`, "---\ndescription: Formatting and naming conventions\nalwaysApply: true\n---\n\n- Four-space indentation; rustfmt defaults.\n- Snake case for functions and modules, camel case for types.\n", now - 8 * day],
  [`${PROJECT}/.cursor/rules/testing.mdc`, "---\ndescription: How to write and run tests\nglobs: tests/**,src/**/*_test.rs\nalwaysApply: false\n---\n\n- Integration tests use the `test_app()` fixture from `tests/common.rs`.\n- Never hit the real Stripe API; use the mock in `tests/stripe_mock.rs`.\n", now - 3 * day],
];

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
  /** Instruction files (and their backup copies under the Hive home): absolute path → text. */
  instructions: Map<string, string>;
  /** Backups per instruction file, oldest first. */
  instructionBackups: Map<string, T.InstructionBackup[]>;
  instructionModified: Map<string, number>;
  /** Rules folders that exist even when empty. */
  instructionDirs: Set<string>;
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

const SKILL_DIR = ".claude/skills/pdf";

type StepSeed = [kind: T.TraceStepKind, summary: string, chars: number, tool?: string, category?: T.ToolCategory, isError?: boolean];

/** The timeline of one mock execution: what the agent thought, read, ran and wrote. */
function traceSteps(arm: string, caseId: number, repeat: number): { steps: StepSeed[]; opensSkill: boolean } {
  const task = caseId === 1 ? "invoice.pdf" : caseId === 2 ? "a.pdf and b.pdf" : "w9.pdf";
  const script = caseId === 1 ? "extract.py" : caseId === 2 ? "merge.py" : "fill.py";
  const output = caseId === 1 ? "invoice.csv" : caseId === 2 ? "merged.pdf" : "filled.pdf";
  // One with_skill run never opened the skill (and is the run that failed an expectation).
  const opensSkill = arm === "with_skill" && !(caseId === 3 && repeat === 2);
  const steps: StepSeed[] = [];
  if (opensSkill) {
    steps.push(
      ["thinking", `The pdf skill is installed for this task. Let me read the skill instructions in SKILL.md before touching ${task}.`, 420],
      ["tool_use", `${SKILL_DIR}/SKILL.md`, 64, "Read", "read"],
      ["tool_result", "# PDF skill\n\nUse pdfplumber for tables and pypdf for merging and forms. Scripts in scripts/ are tested…", 3840],
      ["tool_use", `${SKILL_DIR}/scripts/${caseId === 1 ? "extract_tables.py" : "pdf_tools.py"}`, 72, "Read", "read"],
      ["tool_result", "#!/usr/bin/env python3\nimport sys\nimport pdfplumber…", 2210],
      ["thinking", `The skill says to use ${caseId === 1 ? "pdfplumber with the lines strategy" : "pypdf"} and to verify the output afterwards. I'll follow the bundled script.`, 310],
    );
  } else {
    steps.push(["thinking", `I need to work on ${task}. There are no instructions for PDFs here, so I'll look at what is in the sandbox and parse the PDF directly. PDF files are object streams; a regex over the raw bytes may be enough for simple files, otherwise I will need a library.`, 440 + repeat * 60 + caseId * 30]);
  }
  steps.push(
    ["tool_use", "*.pdf", 24, "Glob", "search"],
    ["tool_result", caseId === 2 ? "a.pdf\nb.pdf" : task, 40],
    ["tool_use", `ls -la && python3 -c "import ${opensSkill ? "pdfplumber, pypdf" : "re"}"`, 58, "Bash", "command"],
    ["tool_result", "total 48\n-rw-r--r-- 1 user user 18234 invoice.pdf…", 310],
  );
  if (!opensSkill) {
    steps.push(["tool_use", `head -c 400 ${task.split(" ")[0]}`, 32, "Bash", "read"], ["tool_result", "%PDF-1.7\n%âãÏÓ\n1 0 obj\n<</Type/Catalog…", 400]);
    if (caseId === 2) steps.push(["tool_use", "/Type /Page in a.pdf", 30, "Grep", "search"], ["tool_result", "a.pdf: binary file matches", 28]);
    steps.push(["thinking", "The objects are in compressed streams, so a regex will not see the content. I'll write a small parser with zlib and fall back to a library if that fails.", 330 + repeat * 40]);
  }
  steps.push(["text", opensSkill ? `I'll write ${script} following the skill's approach.` : `I'll write ${script} and parse the raw PDF objects.`, 96], ["tool_use", script, opensSkill ? 1480 : 1120, "Write", "write"], ["tool_result", `File written: ${script}`, 24]);
  const fails = arm === "with_skill" && caseId === 2 && repeat === 1;
  if (fails) {
    steps.push(["tool_use", `python3 ${script}`, 18, "Bash", "command"], ["tool_result", "Traceback (most recent call last):\n  File \"merge.py\", line 2, in <module>\nModuleNotFoundError: No module named 'pypdf'", 142, undefined, undefined, true], ["thinking", "pypdf is not installed in this sandbox; install it and rerun.", 90], ["tool_use", "pip install -q pypdf", 20, "Bash", "command"], ["tool_result", "", 0]);
  }
  steps.push(["tool_use", `python3 ${script}`, 18, "Bash", "command"], ["tool_result", `Wrote ${output}`, 60], ["tool_use", `head -5 ${output}`, 22, "Bash", "read"], ["tool_result", caseId === 1 ? "description,qty,unit,amount\nConsulting,12,95.00,1140.00…" : "%PDF-1.7…", 220], ["text", arm === "with_skill" ? `Done: ${output} was produced following the skill's procedure and verified.` : `Done: ${output} was produced by parsing the PDF directly.`, 140]);
  return { steps, opensSkill };
}

/** An execution trace consistent with `traceSteps` and with the case result's token figures. */
function makeTrace(arm: string, caseId: number, repeat: number, durationMs: number, inputTokens: number, outputTokens: number): T.ExecutionTrace {
  const { steps: seeds, opensSkill } = traceSteps(arm, caseId, repeat);
  let at = 900;
  const steps: T.TraceStep[] = seeds.map(([kind, summary, chars, tool, category, isError]) => {
    at += kind === "tool_result" ? 400 + Math.round(chars / 8) : kind === "thinking" ? 1800 : kind === "text" ? 700 : 1200;
    return { atMs: at, kind, tool: tool ?? null, category: category ?? null, summary, chars, isError: isError ?? false };
  });
  const uses = steps.filter((s) => s.kind === "tool_use");
  const by = (c: T.ToolCategory) => uses.filter((s) => s.category === c).length;
  const tools: Record<string, number> = {};
  for (const s of uses) tools[s.tool!] = (tools[s.tool!] ?? 0) + 1;
  const reads = uses.filter((s) => s.category === "read");
  const filesRead = [...new Set(reads.map((s) => s.summary.replace(/^head -c 400 |^head -5 /, "")))];
  const filesWritten = [...new Set(uses.filter((s) => s.category === "write").map((s) => s.summary))];
  const skillReads = reads.filter((s) => s.summary.startsWith(SKILL_DIR)).length;
  const sum = (kind: T.TraceStepKind) => steps.filter((s) => s.kind === kind).reduce((n, s) => n + s.chars, 0);
  const thinkingChars = sum("thinking");
  const textChars = sum("text");
  const toolInputChars = sum("tool_use");
  const toolResultChars = sum("tool_result");
  const outChars = thinkingChars + textChars + toolInputChars;
  const thinkingTokens = Math.round((outputTokens * thinkingChars) / outChars);
  const textTokens = Math.round((outputTokens * textChars) / outChars);
  const toolInputTokens = outputTokens - thinkingTokens - textTokens;
  const toolResultTokens = Math.ceil(toolResultChars / 4);
  const cacheReadTokens = Math.round(inputTokens * (opensSkill ? 0.82 : 0.74));
  const cacheCreationTokens = Math.round(inputTokens * 0.11);
  const first = steps.find((s) => s.kind === "tool_use");
  return {
    turns: uses.length + 1,
    toolCalls: uses.length,
    reads: reads.length,
    searches: by("search"),
    writes: by("write"),
    commands: by("command"),
    web: 0,
    subagents: 0,
    otherTools: 0,
    toolErrors: steps.filter((s) => s.isError).length,
    tools,
    filesRead,
    filesWritten,
    filesReadCount: filesRead.length,
    filesWrittenCount: filesWritten.length,
    skillReads,
    skillInContext: false,
    subagentTurns: 0,
    subagentToolCalls: 0,
    thinkingBlocks: steps.filter((s) => s.kind === "thinking").length,
    thinkingChars,
    textBlocks: steps.filter((s) => s.kind === "text").length,
    textChars,
    toolInputChars,
    toolResultChars,
    thinkingTokens,
    textTokens,
    toolInputTokens,
    toolResultTokens,
    ioShare: (toolInputTokens + toolResultTokens) / (toolInputTokens + toolResultTokens + thinkingTokens + textTokens),
    cacheReadTokens,
    cacheCreationTokens,
    uncachedInputTokens: inputTokens - cacheReadTokens - cacheCreationTokens,
    modelTimeMs: Math.round(durationMs * 0.68),
    toolTimeMs: Math.round(durationMs * 0.21),
    firstToolMs: first?.atMs ?? null,
    steps,
    stepsTruncated: 0,
  };
}

function statsOf(values: number[]): T.Stats {
  if (values.length === 0) return { n: 0, mean: 0, stddev: 0, min: 0, max: 0 };
  const mean = values.reduce((a, b) => a + b, 0) / values.length;
  const variance = values.length > 1 ? values.reduce((a, b) => a + (b - mean) ** 2, 0) / (values.length - 1) : 0;
  return { n: values.length, mean, stddev: Math.sqrt(variance), min: Math.min(...values), max: Math.max(...values) };
}

/** The traces `makeCaseResults` produces for one arm (3 cases × 2 repeats). */
function armTraces(arm: string): T.ExecutionTrace[] {
  return [1, 2, 3].flatMap((caseId) => [1, 2].map((repeat) => makeTrace(arm, caseId, repeat, caseDuration(arm, caseId), caseInputTokens(arm, repeat), caseOutputTokens(arm))));
}

/** Per-arm activity aggregated over the traces `makeCaseResults` produces for that arm. */
function makeActivity(arm: string): T.ArmActivity {
  const traces = armTraces(arm);
  const of = (f: (t: T.ExecutionTrace) => number) => statsOf(traces.map(f));
  const timed = (f: (t: T.ExecutionTrace) => number | null | undefined) => statsOf(traces.map(f).filter((v): v is number => typeof v === "number"));
  return {
    traced: traces.length,
    turns: of((t) => t.turns),
    toolCalls: of((t) => t.toolCalls),
    reads: of((t) => t.reads),
    searches: of((t) => t.searches),
    writes: of((t) => t.writes),
    commands: of((t) => t.commands),
    web: of((t) => t.web),
    toolErrors: of((t) => t.toolErrors),
    filesRead: of((t) => t.filesReadCount),
    filesWritten: of((t) => t.filesWrittenCount),
    thinkingTokens: of((t) => t.thinkingTokens),
    textTokens: of((t) => t.textTokens),
    toolInputTokens: of((t) => t.toolInputTokens),
    toolResultTokens: of((t) => t.toolResultTokens),
    ioShare: of((t) => t.ioShare),
    cacheReadTokens: of((t) => t.cacheReadTokens),
    freshInputTokens: of((t) => t.cacheCreationTokens + t.uncachedInputTokens),
    modelTimeMs: timed((t) => t.modelTimeMs),
    toolTimeMs: timed((t) => t.toolTimeMs),
    skillOpened: traces.filter((t) => t.skillReads > 0).length / traces.length,
  };
}

const caseDuration = (arm: string, caseId: number) => (arm === "with_skill" ? 95_000 + caseId * 4000 : 70_000 + caseId * 6000);
const caseInputTokens = (arm: string, repeat: number) => (arm === "with_skill" ? 36_000 + repeat * 1000 : 24_000 + repeat * 900);
const caseOutputTokens = (arm: string) => (arm === "with_skill" ? 5200 : 4100);

function makeBenchmark(skill: string, withRate: number, baseRate: number, verdict: T.Verdict): T.Benchmark {
  const stats = (mean: number, sd: number, n: number): T.Stats => ({ n, mean, stddev: sd, min: Math.max(0, mean - sd * 1.5), max: Math.min(1, mean + sd * 1.5) });
  return {
    skillName: skill,
    skillHash: "b3349684c1d2e4f5",
    executor: "Claude Code (claude-sonnet-4-5)",
    grader: "Anthropic API (claude-sonnet-4-5)",
    arms: [
      { label: "with_skill", runs: 6, errors: 0, pass_rate: stats(withRate, 0.08, 6), tokens: { n: 6, mean: 41200, stddev: 6100, min: 32000, max: 50100 }, duration_ms: { n: 6, mean: 98_000, stddev: 21_000, min: 70_000, max: 130_000 }, cost_usd: 0.42 } as unknown as T.ArmSummary,
      { label: "without_skill", runs: 6, errors: 0, pass_rate: stats(baseRate, 0.14, 6), tokens: { n: 6, mean: 28400, stddev: 9000, min: 17000, max: 41000 }, duration_ms: { n: 6, mean: 76_000, stddev: 30_000, min: 40_000, max: 120_000 }, cost_usd: 0.3 } as unknown as T.ArmSummary,
    ].map((a) => ({
      label: a.label,
      runs: a.runs,
      errors: a.errors,
      passRate: (a as unknown as { pass_rate: T.Stats }).pass_rate,
      tokens: a.tokens,
      durationMs: (a as unknown as { duration_ms: T.Stats }).duration_ms,
      toolCalls: statsOf(armTraces(a.label).map((t) => t.toolCalls)),
      costUsd: (a as unknown as { cost_usd: number }).cost_usd,
      activity: makeActivity(a.label),
      // The pdf skill's run-0 predates traces in the mock history (see initialState).
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
      "with_skill opened the skill in only 5 of 6 run(s): the other run(s) never read it, so its instructions could not help there.",
      "with_skill reads 1.4× as many files as without_skill (2.8 vs 2.0 per run) and spends 69% of its traffic on tool IO vs 54%.",
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
        const inputTokens = caseInputTokens(arm, repeat);
        const outputTokens = caseOutputTokens(arm);
        const durationMs = caseDuration(arm, c.id);
        const trace = makeTrace(arm, c.id, repeat, durationMs, inputTokens, outputTokens);
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
          inputTokens,
          outputTokens,
          durationMs,
          toolCalls: trace.toolCalls,
          costUsd: 0.07,
          outputExcerpt: trace.skillReads > 0 ? "I read SKILL.md and used pdfplumber to extract the tables into invoice.csv…" : arm === "with_skill" ? "I filled the form with pypdf directly and saved it as filled.pdf…" : "I attempted to parse the PDF with a regex over the raw bytes…",
          sandboxPath: `${run.workspace}/cases/${c.name}/${arm}/${repeat}`,
          transcriptPath: `${run.workspace}/cases/${c.name}/${arm}/${repeat}/transcript.jsonl`,
          graderNotes: null,
          error: null,
          trace,
          graderInputTokens: 1900 + c.id * 120,
          graderOutputTokens: 240,
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
    projects: [{ path: PROJECT, name: "acme-api", addedAt: now - 5 * day }],
    runs: [run1, run2, run3, run4, run0],
    caseResults,
    suites,
    instructions: new Map(INSTRUCTION_SEED.map(([path, text]) => [path, text])),
    instructionBackups: new Map(),
    instructionModified: new Map(INSTRUCTION_SEED.map(([path, , modifiedAt]) => [path, modifiedAt])),
    instructionDirs: new Set([`${PROJECT}/.cursor/rules`]),
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

const NAME_RE = /^[a-z0-9]+(-[a-z0-9]+)*$/;
const slugify = (s: string) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64);

/** Mock figure for the merged `SKILL.md` body and the model's token counts. */
const tokensIn = (text: string) => Math.ceil(text.length / 4);

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
      installed: installedTargetIds.includes(targetId),
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

  /** The skills a merge request names, in order, with the checks the Rust core makes. */
  function mergeSources(ids: string[]): (T.LibraryItem & { body: string })[] {
    const distinct = [...new Set(ids)];
    if (distinct.length < 2) throw new ApiError("invalid", "pick at least two skills to merge");
    return distinct.map((id) => {
      const it = byId.get(id);
      if (!it) throw new ApiError("not_found", `item ${id}`);
      if (it.kind !== "skill") throw new ApiError("invalid", `${it.name} is ${it.kind === "agent" ? "an agent" : "a command"}; only skills can be merged`);
      return it;
    });
  }

  // --- instruction files -------------------------------------------------------------------

  type InstructionTargets = { ids: string[]; names: string[] };
  /** The location a path belongs to, when it is one the mock knows. */
  interface ResolvedInstruction {
    spec: InstructionSpec;
    scope: T.Scope;
    project: string | null;
    /** For a rules-folder member: the folder. */
    parent: string | null;
    targets: InstructionTargets;
  }

  function instructionRoot(scope: T.Scope, projectPath: string | null): string {
    if (scope === "user") return HOME;
    if (!projectPath) throw new ApiError("invalid", "project scope needs a project path");
    return projectPath.replace(/\/+$/, "");
  }

  const instructionDirExists = (path: string) => state.instructionDirs.has(path) || [...state.instructions.keys()].some((p) => p.startsWith(`${path}/`));

  /** Describe one path at a location. `parent` is set for a member of a rules folder. */
  function instructionEntry(spec: InstructionSpec, scope: T.Scope, project: string | null, targets: InstructionTargets, path: string, display: string, parent: string | null): T.InstructionFile {
    const kind: T.InstructionKind = parent ? "file" : spec.kind;
    const text = state.instructions.get(path);
    const exists = kind === "directory" ? instructionDirExists(path) : text !== undefined;
    const body = exists && kind === "file" ? (text ?? "") : "";
    return {
      path,
      display,
      scope,
      projectPath: project,
      targetIds: targets.ids,
      targetNames: targets.names,
      label: spec.label,
      kind,
      pattern: kind === "directory" ? (spec.pattern ?? ".md") : null,
      parent,
      exists,
      sizeBytes: body.length,
      modifiedAt: exists ? (state.instructionModified.get(path) ?? null) : null,
      tokens: exists && kind === "file" ? tokensIn(body) : 0,
      lines: exists && kind === "file" ? body.split("\n").length : 0,
      note: spec.note ?? null,
    };
  }

  /** Match an absolute path against the known locations: user scope, the given project, then every registered project. */
  function resolveInstruction(path: string, projectPath: string | null): ResolvedInstruction {
    const roots: [T.Scope, string | null][] = [["user", null]];
    if (projectPath) roots.push(["project", instructionRoot("project", projectPath)]);
    for (const p of state.projects) if (!roots.some(([, r]) => r === p.path)) roots.push(["project", p.path]);
    for (const [scope, projectRoot] of roots) {
      const root = scope === "user" ? HOME : projectRoot!;
      let hit: ResolvedInstruction | null = null;
      for (const spec of INSTRUCTION_SPECS.filter((s) => s.scope === scope)) {
        const target = targetSpecs.find((t) => t.id === spec.targetId);
        if (!target) continue;
        const base = `${root}/${spec.rel}`;
        let parent: string | null = null;
        if (base === path) {
          if (spec.kind === "directory") throw new ApiError("invalid", `${spec.rel} is a rules folder; write one of its files instead`);
        } else if (spec.kind === "directory" && path.startsWith(`${base}/`) && memberNameOk(path.slice(base.length + 1), spec.pattern ?? ".md")) {
          parent = base;
        } else continue;
        if (hit) {
          hit.targets.ids.push(target.id);
          hit.targets.names.push(target.name);
        } else hit = { spec, scope, project: projectRoot, parent, targets: { ids: [target.id], names: [target.name] } };
      }
      if (hit) return hit;
    }
    throw new ApiError("invalid", `${path} is not an instruction file of a known agent (add the project under Agents first, or pass its path)`);
  }

  function resolvedInstructionEntry(path: string, r: ResolvedInstruction): T.InstructionFile {
    const base = r.scope === "user" ? `~/${r.spec.rel}` : r.spec.rel;
    const display = r.parent ? `${base}/${path.split("/").pop() ?? ""}` : base;
    return instructionEntry(r.spec, r.scope, r.project, r.targets, path, display, r.parent);
  }

  /** Keep a copy of `path` under the Hive home before it changes; the 20 newest are kept. */
  function backupInstruction(path: string): T.InstructionBackup | null {
    const text = state.instructions.get(path);
    if (text === undefined) return null;
    const key = path.replace(/^\/+/, "").replace(/[^A-Za-z0-9.-]/g, "_");
    const savedAt = Date.now();
    const ext = /\.[^./]+$/.exec(path)?.[0] ?? ".txt";
    let dest = `${INSTRUCTION_BACKUPS}/${key}/${savedAt}${ext}`;
    for (let n = 1; state.instructions.has(dest); n++) dest = `${INSTRUCTION_BACKUPS}/${key}/${savedAt}-${n}${ext}`;
    state.instructions.set(dest, text);
    state.instructionModified.set(dest, savedAt);
    const backup: T.InstructionBackup = { path: dest, savedAt, sizeBytes: text.length };
    const all = state.instructionBackups.get(path) ?? [];
    all.push(backup);
    while (all.length > 20) {
      const oldest = all.shift()!;
      state.instructions.delete(oldest.path);
      state.instructionModified.delete(oldest.path);
    }
    state.instructionBackups.set(path, all);
    return backup;
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
    ui_report: () => null,
    dashboard: () => ({
      stats: stats(),
      targets: installedTargetIds.map((t) => targetStatus(t, "user", null)),
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
    merge_propose: async ({ request }) => {
      await sleep(900);
      const req = request as T.MergeRequest;
      const sources = mergeSources(req.itemIds);
      const provider = req.providerId
        ? state.providers.find((p) => p.id === req.providerId) ?? (() => { throw new ApiError("not_found", `provider ${req.providerId}`); })()
        : state.providers.find((p) => p.roles.includes("recommender")) ?? state.providers.find((p) => p.roles.includes("grader")) ?? state.providers[0];
      if (!provider) throw new ApiError("not_found", "no provider is configured; add one in Settings → Providers");
      const [first, second] = sources;
      const name = req.name ?? slugify(`${first.name}-${second.name}`);
      const firstSentence = (s: string) => s.split(/(?<=\.)\s+/)[0].replace(/\.$/, "");
      const topics = sources.map((s) => s.name.replace(/-/g, " "));
      const description = `${firstSentence(first.description)}. ${firstSentence(second.description)}. Use when the task involves ${topics.slice(0, -1).join(", ")} or ${topics[topics.length - 1]}.`;
      const body = [
        `# ${name}`,
        "",
        `Combines ${sources.map((s) => `\`${s.name}\``).join(" and ")} into one skill. Read the section that matches the task; the shared conventions at the end apply to all of them.${req.guidance ? ` Guidance applied: ${req.guidance.trim()}` : ""}`,
        "",
        ...sources.flatMap((s) => [`## ${s.name.replace(/-/g, " ").replace(/^\w/, (c) => c.toUpperCase())}`, "", s.body.replace(/^#\s[^\n]*\n+/, "").trim() || "_No instructions in the original._", ""]),
        "## Resolved differences",
        "",
        `- Where ${first.name} and ${second.name} described the same step differently, the more specific wording was kept.`,
        "- Tool recommendations that appeared in only one source are kept with the section they came from.",
        "- The combined \"Use when\" clause covers every trigger of the originals.",
        "",
      ].join("\n");
      const files = sources.flatMap((s, i): T.MergeFile[] =>
        i === 0
          ? [
              { fromItemId: s.id, fromName: s.name, path: "scripts/extract.py", dest: "scripts/extract.py", sizeBytes: 1830, keep: true },
              { fromItemId: s.id, fromName: s.name, path: "reference.md", dest: "reference.md", sizeBytes: 9100, keep: true },
            ]
          : [{ fromItemId: s.id, fromName: s.name, path: "scripts/extract.py", dest: `${s.slug}/scripts/extract.py`, sizeBytes: 1830, keep: false }],
      );
      const taken = items.find((i) => i.kind === "skill" && i.name === name && !sources.includes(i));
      const findings: T.Finding[] = taken ? [{ code: "name.taken", severity: "error", message: `A ${taken.kind} named \`${name}\` is already in the library; pick another name or merge it in too`, field: "name" }] : [];
      const warnings = req.name ? [] : [`The model proposed "${first.name} + ${second.name}"; it was normalised to \`${name}\` to fit the skill name rules.`];
      const evalCases = sources.reduce((n, s) => n + (state.suites.get(s.id)?.cases.length ?? 0), 0);
      const inputTokens = sources.reduce((n, s) => n + tokensIn(s.body) + s.descriptionTokens, 0) + 640;
      return {
        sources: sources.map((s) => ({ item: stripBody(s), fileCount: detail(s.id).files.filter((f) => !f.isDir && f.path !== "SKILL.md").length, evalCases: state.suites.get(s.id)?.cases.length ?? 0, deployments: state.deployments.filter((d) => d.itemId === s.id) })),
        conflicts: conflictsFor(sources),
        name,
        description,
        body,
        frontmatter: { metadata: { "merged-from": sources.map((s) => s.name).join(", ") }, ...(first.frontmatter.license ? { license: first.frontmatter.license } : {}) },
        files,
        tags: [...new Set(sources.flatMap((s) => s.tags))],
        notes: `Kept ${first.name}'s overview as the opening because it states the trigger more precisely, and folded ${second.name}'s steps in as their own section. The reference files of both are identical, so only one copy is carried over.`,
        dropped: [`${second.name}'s closing note about keeping the file under 500 lines (already covered by the shared conventions).`],
        warnings,
        validation: { findings },
        bodyTokensBefore: sources.reduce((n, s) => n + s.tokenEstimate, 0),
        bodyTokensAfter: tokensIn(body),
        descriptionTokensBefore: sources.reduce((n, s) => n + s.descriptionTokens, 0),
        descriptionTokensAfter: tokensIn(description),
        evalCases,
        providerId: provider.id,
        providerName: provider.name,
        model: provider.model ?? null,
        inputTokens,
        outputTokens: tokensIn(body) + tokensIn(description) + 120,
        latencyMs: 900,
      } satisfies T.MergeProposal;
    },
    merge_apply: ({ commit }) => {
      const c = commit as T.MergeCommit;
      if (!NAME_RE.test(c.name) || c.name.length > 64) throw new ApiError("invalid", "name must be 1–64 lowercase letters, digits or single hyphens");
      const sources = mergeSources(c.sourceIds);
      const taken = items.find((i) => i.kind === "skill" && i.name === c.name && !sources.includes(i));
      if (taken) throw new ApiError("already_exists", `a ${taken.kind} named '${c.name}' is already in the library`);
      const replaced = sources.find((s) => s.name === c.name) ?? null;
      const frontmatter = { ...c.frontmatter };
      let merged: (typeof items)[number];
      if (replaced) {
        merged = replaced;
        Object.assign(merged, { description: c.description, body: c.body, frontmatter: { name: c.name, description: c.description, ...frontmatter }, tags: [...new Set(c.tags)], updatedAt: Date.now(), contentHash: Math.random().toString(16).slice(2).padEnd(40, "0"), tokenEstimate: tokensIn(c.body) + 300, descriptionTokens: tokensIn(c.description) });
      } else {
        merged = item({ name: c.name, description: c.description, tags: [...new Set(c.tags)], source: src("authored", ""), frontmatter }, c.body);
        items.push(merged);
        byId.set(merged.id, merged);
      }
      const retired: T.LibraryItem[] = [];
      const deployed: T.Deployment[] = [];
      if (c.retireSources) {
        for (const s of sources) {
          if (s === merged) continue;
          items.splice(items.indexOf(s), 1);
          byId.delete(s.id);
          retired.push(stripBody(s));
          for (const d of state.deployments.filter((x) => x.itemId === s.id)) {
            const duplicate = state.deployments.some((x) => x.itemId === merged.id && x.targetId === d.targetId && x.scope === d.scope && (x.projectPath ?? null) === (d.projectPath ?? null));
            if (duplicate) {
              state.deployments = state.deployments.filter((x) => x !== d);
              continue;
            }
            Object.assign(d, { itemId: merged.id, itemName: merged.name, path: d.path.replace(new RegExp(`/${s.name}$`), `/${merged.name}`), deployedHash: merged.contentHash, updatedAt: Date.now() });
            deployed.push(d);
          }
        }
      }
      let evalCases = 0;
      if (c.carryEvals) {
        const suites = sources.map((s) => state.suites.get(s.id)).filter((s): s is T.EvalSuite => !!s);
        const cases = suites.flatMap((s) => s.cases).map((k, i) => ({ ...k, id: i + 1 }));
        const seen = new Set<string>();
        const triggerCases = suites.flatMap((s) => s.triggerCases).filter((t) => !seen.has(t.query) && seen.add(t.query));
        state.suites.set(merged.id, { itemId: merged.id, skillName: merged.name, path: `${merged.path}/evals`, exists: cases.length > 0, cases, triggerCases });
        evalCases = cases.length;
      }
      emit({ type: "library_changed" });
      emit({ type: "deployments_changed" });
      return { item: stripBody(merged), retired, replacedId: replaced?.id ?? null, deployed, failed: [], evalCases, warnings: [] } satisfies T.MergeResult;
    },

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
    targets_installed: () => installedTargetIds.map((t) => targetStatus(t, "user", null)),
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

    instructions_list: ({ scope, projectPath }) => {
      const sc = scope as T.Scope;
      const root = instructionRoot(sc, (projectPath as string | null) ?? null);
      const project = sc === "project" ? root : null;
      // One group per path, in the table's order, naming the installed agents that read it. An
      // agent that is not installed is only named for a file that exists and no installed agent
      // reads, so the seeded AGENTS.md stays "Codex CLI · Cursor" rather than every known reader.
      const groups: { path: string; spec: InstructionSpec; targets: InstructionTargets }[] = [];
      const specs = INSTRUCTION_SPECS.filter((s) => s.scope === sc);
      const add = (spec: InstructionSpec, target: T.TargetSpec, path: string) => {
        const group = groups.find((g) => g.path === path);
        if (group) {
          group.targets.ids.push(target.id);
          group.targets.names.push(target.name);
        } else groups.push({ path, spec, targets: { ids: [target.id], names: [target.name] } });
      };
      for (const installedPass of [true, false]) {
        for (const spec of specs) {
          const target = targetSpecs.find((t) => t.id === spec.targetId);
          if (!target || installedTargetIds.includes(target.id) !== installedPass) continue;
          const path = `${root}/${spec.rel}`;
          const onDisk = spec.kind === "directory" ? instructionDirExists(path) : state.instructions.has(path);
          if (installedPass || (onDisk && !groups.some((g) => g.path === path))) add(spec, target, path);
        }
      }
      groups.sort((a, b) => specs.findIndex((s) => s === a.spec) - specs.findIndex((s) => s === b.spec));
      const out: T.InstructionFile[] = [];
      for (const { path, spec, targets } of groups) {
        const display = sc === "user" ? `~/${spec.rel}` : spec.rel;
        const top = instructionEntry(spec, sc, project, targets, path, display, null);
        out.push(top);
        if (top.kind === "directory" && top.exists) {
          const pattern = spec.pattern ?? ".md";
          const names = [...state.instructions.keys()]
            .filter((p) => p.startsWith(`${path}/`))
            .map((p) => p.slice(path.length + 1))
            .filter((n) => memberNameOk(n, pattern))
            .sort();
          for (const name of names) out.push(instructionEntry(spec, sc, project, targets, `${path}/${name}`, `${display}/${name}`, path));
        }
      }
      return out;
    },
    instruction_read: ({ path, projectPath }) => {
      const p = path as string;
      if (p.startsWith(`${INSTRUCTION_BACKUPS}/`)) {
        const text = state.instructions.get(p);
        if (text === undefined) throw new ApiError("not_found", `${p} does not exist`);
        const file: T.InstructionFile = { path: p, display: p.split("/").pop() ?? "", scope: "user", projectPath: null, targetIds: [], targetNames: [], label: "Backup", kind: "file", pattern: null, parent: null, exists: true, sizeBytes: text.length, modifiedAt: state.instructionModified.get(p) ?? null, tokens: tokensIn(text), lines: text.split("\n").length, note: null };
        return { file, text } satisfies T.InstructionDocument;
      }
      const r = resolveInstruction(p, (projectPath as string | null) ?? null);
      const file = resolvedInstructionEntry(p, r);
      return { file, text: file.exists ? (state.instructions.get(p) ?? "") : "" } satisfies T.InstructionDocument;
    },
    instruction_write: ({ path, text, projectPath }) => {
      const p = path as string;
      const t = text as string;
      const r = resolveInstruction(p, (projectPath as string | null) ?? null);
      if (t.length > 2 * 1024 * 1024) throw new ApiError("invalid", "the file is larger than 2 MB");
      backupInstruction(p);
      state.instructions.set(p, t);
      state.instructionModified.set(p, Date.now());
      if (r.parent) state.instructionDirs.add(r.parent);
      emit({ type: "instructions_changed" });
      return { file: resolvedInstructionEntry(p, r), text: t } satisfies T.InstructionDocument;
    },
    instruction_delete: ({ path, projectPath }) => {
      const p = path as string;
      const r = resolveInstruction(p, (projectPath as string | null) ?? null);
      if (!state.instructions.has(p)) throw new ApiError("not_found", `${p} does not exist`);
      backupInstruction(p);
      state.instructions.delete(p);
      state.instructionModified.delete(p);
      emit({ type: "instructions_changed" });
      return resolvedInstructionEntry(p, r);
    },
    instruction_backups: ({ path }) => state.instructionBackups.get(path as string) ?? [],

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
