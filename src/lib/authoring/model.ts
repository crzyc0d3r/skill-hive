export type Profile = "portable" | "claude-code";
export type Severity = "error" | "warning" | "info";
export type VersionSource = "manual" | "autosave" | "generated" | "optimized" | "restored";
export type GradingResult = "baseline" | "won" | "lost" | "tie";

export interface Finding {
  ruleId: string;
  severity: Severity;
  path: string;
  line: number;
  message: string;
  fixId?: string;
}

export interface SkillFile {
  path: string;
  content: string;
  executable: boolean;
}

export interface Deployment {
  agent: string;
  mode: "symlink" | "copy";
}

export interface SkillDraft {
  id: string;
  name: string;
  dir: string;
  profile: Profile;
  files: SkillFile[];
  deployments: Deployment[];
  shellPreview: boolean;
}

export interface DraftVersion {
  id: string;
  parentId: string | null;
  label: string;
  source: VersionSource;
  hash: string;
  files: Record<string, string>;
  pinnedRunIds: string[];
  hidden: boolean;
  gradingResult: GradingResult | null;
  expectationPassRate: number | null;
  isCurrentBest: boolean;
}

export interface AgentDraft {
  id: string;
  name: string;
  text: string;
  plugin: boolean;
  exported: string | null;
  disk: string | null;
}

export const EDITOR_SHORTCUTS = [
  { id: "new-skill", keys: "Mod+N", action: "New skill" },
  { id: "new-agent", keys: "Mod+Shift+N", action: "New agent" },
  { id: "generate-skill", keys: "Mod+G", action: "Generate skill" },
  { id: "generate-agent", keys: "Mod+Shift+G", action: "Generate agent" },
  { id: "save", keys: "Mod+S", action: "Save" },
  { id: "snapshot", keys: "Mod+Shift+S", action: "Snapshot version" },
  { id: "cycle", keys: "Mod+J", action: "Cycle Problems, Preview, and Versions" },
  { id: "run-tests", keys: "Mod+Enter", action: "Run tests" },
  { id: "rename", keys: "F2", action: "Rename file" },
  { id: "palette", keys: "Mod+K", action: "Command palette" },
] as const;

const PORTABLE_KEYS = new Set(["name", "description", "license", "compatibility", "metadata", "allowed-tools"]);
const CLAUDE_KEYS = new Set([
  "argument-hint",
  "when_to_use",
  "arguments",
  "disable-model-invocation",
  "user-invocable",
  "disallowed-tools",
  "model",
  "effort",
  "context",
  "agent",
  "background",
  "hooks",
  "paths",
  "shell",
]);
const RESERVED = new Set(["con", "prn", "aux", "nul", "com1", "lpt1"]);
const NEAR_MISS: Record<string, string> = {
  disallowed_tools: "disallowedTools",
  permission_mode: "permissionMode",
  max_turns: "maxTurns",
  mcp_servers: "mcpServers",
  omit_claude_md: "omitClaudeMd",
  initial_prompt: "initialPrompt",
};

export function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

export function sanitizeSkillName(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function skillMarkdown(draft: SkillDraft): string {
  return draft.files.find((file) => file.path === "SKILL.md" || file.path === "skill.md")?.content ?? "";
}

function finding(ruleId: string, severity: Severity, path: string, line: number, message: string, fixId?: string): Finding {
  return { ruleId, severity, path, line, message, fixId };
}

interface Frontmatter {
  opened: boolean;
  closed: boolean;
  keys: { name: string; value: string; line: number }[];
  body: string;
}

export function parseFrontmatter(text: string): Frontmatter {
  const lines = text.split("\n");
  if ((lines[0] ?? "").trim() !== "---") return { opened: false, closed: false, keys: [], body: text };
  let end = -1;
  for (let index = 1; index < lines.length; index += 1) {
    if (lines[index].trim() === "---") {
      end = index;
      break;
    }
  }
  if (end < 0) return { opened: true, closed: false, keys: [], body: "" };
  const keys = [];
  for (let index = 1; index < end; index += 1) {
    const match = /^([A-Za-z0-9_-]+):\s*(.*)$/.exec(lines[index]);
    if (match) keys.push({ name: match[1], value: match[2], line: index + 1 });
  }
  return { opened: true, closed: true, keys, body: lines.slice(end + 1).join("\n") };
}

export function setFrontmatterString(markdown: string, key: string, value: string): string {
  const lines = markdown.split("\n");
  const index = lines.findIndex((line) => line.startsWith(`${key}:`));
  if (index >= 0) lines[index] = `${key}: ${value}`;
  return lines.join("\n");
}

export function validateSkill(draft: SkillDraft): Finding[] {
  const findings: Finding[] = [];
  const marker = draft.files.find((file) => file.path === "SKILL.md");
  const legacy = draft.files.find((file) => file.path === "skill.md");
  if (!marker && !legacy) {
    findings.push(finding("S-01", "error", draft.dir, 1, "SKILL.md is missing", "rename-skill-md"));
    return findings;
  }
  if (!marker && legacy) {
    findings.push(finding("S-01", "warning", "skill.md", 1, "rename skill.md to SKILL.md", "rename-skill-md"));
  }
  const file = marker ?? legacy;
  if (!file) return findings;
  const doc = parseFrontmatter(file.content);
  if (!doc.opened) findings.push(finding("S-02", "error", file.path, 1, "frontmatter must start with ---"));
  else if (!doc.closed) findings.push(finding("S-03", "error", file.path, 1, "frontmatter is not closed"));
  const name = doc.keys.find((entry) => entry.name === "name")?.value ?? "";
  const description = doc.keys.find((entry) => entry.name === "description")?.value ?? "";
  if (!name) findings.push(finding("S-04", "error", file.path, 1, "name is required"));
  if (name && name !== draft.name) findings.push(finding("S-13", "warning", file.path, 1, "name differs from the directory"));
  if (name && sanitizeSkillName(name) !== name) findings.push(finding("S-14", "error", file.path, 1, "sanitize_skill_name would change this name"));
  if (RESERVED.has(name.toLowerCase())) findings.push(finding("S-15", "error", file.path, 1, "name is reserved"));
  if (!description.trim()) findings.push(finding("S-16", "error", file.path, 1, "description must be non-empty"));
  if (description.length > 1024) findings.push(finding("S-17", "error", file.path, 1, "description is longer than 1024"));
  if (/[<>]/.test(description)) findings.push(finding("S-18", "error", file.path, 1, "description must not contain < or >"));
  for (const entry of doc.keys) {
    const allowed = draft.profile === "portable" ? PORTABLE_KEYS : new Set([...PORTABLE_KEYS, ...CLAUDE_KEYS]);
    if (!allowed.has(entry.name)) {
      findings.push(
        finding(
          "S-23",
          draft.profile === "portable" ? "error" : "warning",
          file.path,
          entry.line,
          `${entry.name} is not allowed in this profile`,
          "remove-unknown-key",
        ),
      );
    }
  }
  const lines = doc.body.split("\n").length;
  const tokens = estimateTokens(doc.body);
  if (lines >= 500) findings.push(finding("S-27", "warning", file.path, 1, "body is at least 500 lines"));
  if (tokens >= 5000) findings.push(finding("S-28", "warning", file.path, 1, "body is at least 5000 tokens"));
  const referenced = [...doc.body.matchAll(/(?:scripts|references|assets|evals)\/[A-Za-z0-9_./-]+/g)].map((match) => match[0]);
  for (const path of referenced) {
    if (!draft.files.some((item) => item.path === path)) {
      findings.push(finding("S-30", "warning", file.path, 1, `${path} is referenced but missing`, "create-missing-file"));
    }
  }
  for (const item of draft.files) {
    if (item.path.startsWith("scripts/") && !item.executable) {
      findings.push(finding("S-32", "warning", item.path, 1, "script is not executable", "set-executable"));
    }
  }
  const evals = draft.files.find((item) => item.path === "evals/evals.json");
  if (evals) {
    try {
      const parsed = JSON.parse(evals.content) as { skill_name?: string };
      if (parsed.skill_name && parsed.skill_name !== draft.name) {
        findings.push(finding("S-33", "warning", "evals/evals.json", 1, "skill_name does not match the skill", "sync-skill-name"));
      }
    } catch {
      findings.push(finding("S-33", "error", "evals/evals.json", 1, "evals.json is not JSON"));
    }
  }
  if (/!`[^`]+`/.test(doc.body)) findings.push(finding("S-31", "info", file.path, 1, "shell substitution is informational in preview"));
  return findings;
}

export function validateAgentText(text: string, plugin: boolean): Finding[] {
  const findings: Finding[] = [];
  const doc = parseFrontmatter(text);
  if (!doc.opened || !doc.closed) {
    findings.push(finding("A-02", "error", "agent.md", 1, "frontmatter must open on line 1; a late --- is skipped"));
    return findings;
  }
  const name = doc.keys.find((entry) => entry.name === "name")?.value ?? "";
  const description = doc.keys.find((entry) => entry.name === "description")?.value ?? "";
  if (!name) findings.push(finding("A-02", "error", "agent.md", 1, "name is required; Claude Code skips the file"));
  if (name.startsWith("-") || name.includes(":") || name.length > 256) {
    findings.push(finding("A-04", "error", "agent.md", 1, "name is invalid"));
  }
  if (!description.trim()) findings.push(finding("A-03", "error", "agent.md", 1, "description is required"));
  for (const entry of doc.keys) {
    const fixed = NEAR_MISS[entry.name];
    if (fixed) findings.push(finding("A-05", "warning", "agent.md", entry.line, `rename ${entry.name} to ${fixed}`, "rename-key"));
  }
  const tools = doc.keys.find((entry) => entry.name === "tools")?.value ?? "";
  if (tools.split(",").map((item) => item.trim()).includes("Skill")) {
    findings.push(finding("A-06", "warning", "agent.md", 1, "list skills in skills, not tools", "move-skill-tool"));
  }
  if (plugin) {
    for (const key of ["hooks", "mcpServers", "permissionMode", "initialPrompt"]) {
      if (doc.keys.some((entry) => entry.name === key)) findings.push(finding("A-15", "warning", "agent.md", 1, `${key} has no effect on a plugin agent`));
    }
  }
  if (!doc.body.trim()) findings.push(finding("A-16", "warning", "agent.md", 1, "the system prompt body is empty"));
  return findings;
}

export function bodyStats(draft: SkillDraft): { lines: number; tokens: number; amber: boolean } {
  const body = parseFrontmatter(skillMarkdown(draft)).body;
  const lines = body.split("\n").length;
  const tokens = estimateTokens(body);
  return { lines, tokens, amber: lines >= 500 || tokens >= 5000 };
}

export function errorCount(findings: Finding[]): number {
  return findings.filter((item) => item.severity === "error").length;
}

export function warningCount(findings: Finding[]): number {
  return findings.filter((item) => item.severity === "warning").length;
}

export function applySkillFix(draft: SkillDraft, fixId: string): SkillDraft {
  const files = draft.files.map((file) => ({ ...file }));
  if (fixId === "rename-skill-md") {
    const legacy = files.find((file) => file.path === "skill.md");
    if (legacy) legacy.path = "SKILL.md";
  }
  if (fixId === "remove-unknown-key") {
    const skill = files.find((file) => file.path === "SKILL.md");
    if (skill) {
      const allowed = draft.profile === "portable" ? PORTABLE_KEYS : new Set([...PORTABLE_KEYS, ...CLAUDE_KEYS]);
      skill.content = skill.content
        .split("\n")
        .filter((line) => {
          const match = /^([A-Za-z0-9_-]+):/.exec(line);
          return !match || allowed.has(match[1]);
        })
        .join("\n");
    }
  }
  if (fixId === "set-executable") {
    for (const file of files) if (file.path.startsWith("scripts/")) file.executable = true;
  }
  if (fixId === "create-missing-file") {
    const missing = validateSkill({ ...draft, files }).filter((item) => item.fixId === "create-missing-file");
    for (const item of missing) {
      const path = item.message.split(" ")[0];
      if (!files.some((file) => file.path === path)) files.push({ path, content: "# created\n", executable: false });
    }
  }
  if (fixId === "sync-skill-name") {
    const evals = files.find((file) => file.path === "evals/evals.json");
    if (evals) {
      const parsed = JSON.parse(evals.content) as { skill_name?: string };
      parsed.skill_name = draft.name;
      evals.content = JSON.stringify(parsed);
    }
  }
  return { ...draft, files };
}

export function applyAgentFix(text: string, fixId: string): string {
  if (fixId !== "rename-key") return text;
  return Object.entries(NEAR_MISS).reduce((current, [from, to]) => current.replace(`${from}:`, `${to}:`), text);
}

export function renderLoaded(draft: SkillDraft): { text: string; commands: string[]; showRun: boolean } {
  const body = parseFrontmatter(skillMarkdown(draft)).body;
  const text = body.replaceAll("${CLAUDE_SKILL_DIR}", draft.dir).replaceAll("${CLAUDE_PROJECT_DIR}", "/project");
  const commands = [...body.matchAll(/!`([^`]+)`/g)].map((match) => match[1]);
  return { text, commands, showRun: draft.shellPreview };
}

export function planPublish(draft: SkillDraft, confirmed: boolean): { kind: "confirm"; agents: string[] } | { kind: "published"; centerChanged: string[] } {
  const symlinks = draft.deployments.filter((item) => item.mode === "symlink").map((item) => item.agent);
  if (symlinks.length > 0 && !confirmed) return { kind: "confirm", agents: symlinks };
  return { kind: "published", centerChanged: draft.deployments.filter((item) => item.mode === "copy").map((item) => item.agent) };
}

export function resolveInside(root: string, relative: string): { ok: true; path: string } | { ok: false; error: "invalid_input"; message: string } {
  if (relative.split(/[/\\]/).includes("..") || relative.startsWith("/") || /^[A-Za-z]:/.test(relative)) {
    return { ok: false, error: "invalid_input", message: "invalid_input path escapes the skill directory" };
  }
  return { ok: true, path: `${root}/${relative}` };
}

function skillFile(name: string, description: string, body: string): string {
  return `---\nname: ${name}\ndescription: ${description}\n---\n\n${body}\n`;
}

export function instantiateTemplate(templateId: string, name: string): Record<string, string> {
  const description = `Draft a skill named ${name}.`;
  if (templateId === "workflow") {
    return { "SKILL.md": skillFile(name, description, "# Steps\n\n1. Read the request.\n2. Write the result.\n\n## Output format\n\n- finding\n") };
  }
  if (templateId === "reference") {
    return {
      "SKILL.md": skillFile(name, description, "# Overview\n\nRead references/overview.md when the domain is unclear.\n"),
      "references/overview.md": "# Overview\n\n## Contents\n\n- Start here\n",
    };
  }
  if (templateId === "script") {
    return {
      "SKILL.md": skillFile(name, description, "# Run\n\nUse scripts/run.py when a deterministic step is required.\n"),
      "scripts/run.py": "import sys\n\ndef main() -> None:\n    if len(sys.argv) < 2:\n        sys.exit(1)\n\nif __name__ == '__main__':\n    main()\n",
    };
  }
  if (templateId === "variant") {
    return {
      "SKILL.md": skillFile(name, description, "# Variants\n\nRead references/default.md for the default provider.\n"),
      "references/default.md": "# Default\n",
    };
  }
  if (templateId === "document") {
    return { "SKILL.md": skillFile(name, description, "# Document\n\n## Examples\n\n- example\n") };
  }
  return { "SKILL.md": skillFile(name, description, `# ${name}\n`) };
}

export const SKILL_TEMPLATES = [
  { id: "blank", labelKey: "editor.template.blank" },
  { id: "workflow", labelKey: "editor.template.workflow" },
  { id: "reference", labelKey: "editor.template.reference" },
  { id: "script", labelKey: "editor.template.script" },
  { id: "variant", labelKey: "editor.template.variant" },
  { id: "document", labelKey: "editor.template.document" },
] as const;

export function templateDraft(templateId: string, name: string): SkillDraft {
  const files = Object.entries(instantiateTemplate(templateId, name)).map(([path, content]) => ({
    path,
    content,
    executable: path.startsWith("scripts/"),
  }));
  return { id: name, name, dir: `/skills/${name}`, profile: "portable", files, deployments: [], shellPreview: false };
}

export function hashFiles(files: Record<string, string>): string {
  return JSON.stringify(Object.keys(files).sort().map((key) => [key, files[key]]));
}

export function filesRecord(draft: SkillDraft): Record<string, string> {
  return Object.fromEntries(draft.files.map((file) => [file.path, file.content]));
}

export function saveAutosave(versions: DraftVersion[], files: Record<string, string>): DraftVersion[] {
  const hash = hashFiles(files);
  const latest = versions.find((version) => !version.hidden);
  if (latest && latest.hash === hash) return versions;
  const next: DraftVersion = {
    id: `v${versions.length}`,
    parentId: latest?.id ?? null,
    label: "",
    source: "autosave",
    hash,
    files: { ...files },
    pinnedRunIds: [],
    hidden: false,
    gradingResult: versions.length === 0 ? "baseline" : null,
    expectationPassRate: null,
    isCurrentBest: !versions.some((version) => version.isCurrentBest),
  };
  return [next, ...versions];
}

export function snapshotVersion(versions: DraftVersion[], files: Record<string, string>, label: string): DraftVersion[] {
  const saved = saveAutosave(versions, files);
  const latest = saved.find((version) => !version.hidden);
  if (!latest) return saved;
  return saved.map((version) => (version.id === latest.id ? { ...version, label, source: "manual" } : version));
}

export function hideVersion(versions: DraftVersion[], id: string): { versions: DraftVersion[]; error?: string } {
  const target = versions.find((version) => version.id === id);
  if (!target) return { versions, error: "invalid_input missing version" };
  if (target.pinnedRunIds.length > 0) {
    return { versions, error: `invalid_input ${target.pinnedRunIds.join(",")}` };
  }
  return { versions: versions.map((version) => (version.id === id ? { ...version, hidden: true } : version)) };
}

export function restoreVersion(versions: DraftVersion[], id: string, current: Record<string, string>): { versions: DraftVersion[]; files: Record<string, string> } {
  const target = versions.find((version) => version.id === id);
  if (!target) return { versions, files: current };
  const kept = saveAutosave(versions, current);
  const restored: DraftVersion = {
    id: `v${kept.length}`,
    parentId: target.id,
    label: "",
    source: "restored",
    hash: target.hash,
    files: { ...target.files },
    pinnedRunIds: [],
    hidden: false,
    gradingResult: null,
    expectationPassRate: null,
    isCurrentBest: false,
  };
  return { versions: [restored, ...kept], files: { ...target.files } };
}

export function exportHistory(skillName: string, versions: DraftVersion[]): {
  started_at: string;
  skill_name: string;
  current_best: string | null;
  iterations: { version: string; parent: string | null; expectation_pass_rate: number | null; grading_result: GradingResult | null; is_current_best: boolean }[];
} {
  const visible = [...versions].reverse();
  const best = visible.find((version) => version.isCurrentBest);
  return {
    started_at: "2026-10-14T00:00:00Z",
    skill_name: skillName,
    current_best: best?.id ?? null,
    iterations: visible.map((version) => ({
      version: version.id,
      parent: version.parentId,
      expectation_pass_rate: version.expectationPassRate,
      grading_result: version.gradingResult,
      is_current_best: version.isCurrentBest,
    })),
  };
}

export function renderAgentFile(text: string): string {
  return text.endsWith("\n") ? text : `${text}\n`;
}

export function renderAgentsCliJson(text: string): Record<string, { prompt: string; description?: string; tools?: string; model?: string }> {
  const doc = parseFrontmatter(text);
  const name = doc.keys.find((entry) => entry.name === "name")?.value || "agent";
  const fields: { prompt: string; description?: string; tools?: string; model?: string } = { prompt: doc.body.trim() };
  for (const entry of doc.keys) {
    if (entry.name === "description") fields.description = entry.value;
    if (entry.name === "tools") fields.tools = entry.value;
    if (entry.name === "model") fields.model = entry.value;
  }
  return { [name]: fields };
}

export function exportStatus(current: string, exported: string | null, disk: string | null): "missing" | "current" | "stale" | "modified" {
  if (exported == null) return "missing";
  if (disk != null && disk !== exported) return "modified";
  if (current !== exported) return "stale";
  return "current";
}

export function descriptionBudget(descriptions: string[]): { tokens: number; amber: boolean } {
  const tokens = estimateTokens(descriptions.join("\n"));
  return { tokens, amber: tokens > 15000 };
}

export function warningsFixture(): SkillDraft {
  return {
    id: "warnings",
    name: "warnings",
    dir: "/skills/warnings",
    profile: "portable",
    shellPreview: false,
    deployments: [],
    files: [
      {
        path: "SKILL.md",
        executable: false,
        content: "---\nname: warnings\ndescription: Use <files> carefully\nunknown-key: yes\n---\n# Title\n\nSee scripts/run.py and references/missing.md\n",
      },
      { path: "scripts/run.py", content: "print(1)\n", executable: false },
      { path: "evals/evals.json", content: '{"skill_name":"other"}', executable: false },
    ],
  };
}

export function longBodyFixture(): SkillDraft {
  const body = Array.from({ length: 500 }, () => "line").join("\n");
  return {
    id: "gauge",
    name: "gauge",
    dir: "/skills/gauge",
    profile: "portable",
    shellPreview: false,
    deployments: [],
    files: [{ path: "SKILL.md", executable: false, content: `---\nname: gauge\ndescription: A long body used to cross the line gauge.\n---\n${body}\n` }],
  };
}

export function previewFixture(): SkillDraft {
  return {
    id: "preview-skill",
    name: "preview-skill",
    dir: "/skills/preview-skill",
    profile: "portable",
    shellPreview: false,
    deployments: [],
    files: [
      {
        path: "SKILL.md",
        executable: false,
        content: "---\nname: preview-skill\ndescription: Preview substitutions for the loaded view.\n---\nWork in ${CLAUDE_SKILL_DIR}.\n\nRun !`echo hi` when asked.\n",
      },
    ],
  };
}

export function deployFixture(id: string, mode: "symlink" | "copy"): SkillDraft {
  const draft = templateDraft("blank", id);
  draft.deployments = [{ agent: "Claude Code", mode }];
  return draft;
}

export function reviewerFixture(): AgentDraft {
  return {
    id: "reviewer",
    name: "reviewer",
    plugin: false,
    exported: null,
    disk: null,
    text: "---\nname: reviewer\ndescription: Reviews code.\ndisallowed_tools: Bash\ntools: Read\nmodel: sonnet\n---\nYou review.\n",
  };
}
