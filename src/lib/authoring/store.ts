import { useSyncExternalStore } from "react";
import {
  exportAgentDraft,
  fixSkillDraft,
  publishSkillDraft,
  restoreSkillDraft,
  saveAgentDraft,
  saveSkillDraft,
  snapshotSkillDraft,
} from "@/lib/ipc/actions";
import type { SkillCard } from "@/features/library/model";
import { DEMO, demoEnabled } from "@/lib/demo";
import {
  applyAgentFix,
  applySkillFix,
  deployFixture,
  descriptionBudget,
  errorCount,
  exportStatus,
  filesRecord,
  hideVersion,
  longBodyFixture,
  previewFixture,
  renderAgentFile,
  restoreVersion,
  reviewerFixture,
  saveAutosave,
  snapshotVersion,
  templateDraft,
  type AgentDraft,
  type DraftVersion,
  type SkillDraft,
  validateSkill,
  warningCount,
  warningsFixture,
} from "./model";

interface AuthoringState {
  skills: Record<string, SkillDraft>;
  versions: Record<string, DraftVersion[]>;
  agents: Record<string, AgentDraft>;
  centerChanged: string[];
  counts: { errors: number; warnings: number } | null;
  picker: "skill" | "agent" | null;
  later: "skill" | "agent" | null;
  notice: string | null;
}

let state: AuthoringState = { skills: {}, versions: {}, agents: {}, centerChanged: [], counts: null, picker: null, later: null, notice: null };
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of listeners) listener();
}

function update(patch: Partial<AuthoringState>): void {
  state = { ...state, ...patch };
  emit();
}

export function resetAuthoring(): void {
  state = { skills: {}, versions: {}, agents: {}, centerChanged: [], counts: null, picker: null, later: null, notice: null };
  emit();
}

export function authoringSnapshot(): AuthoringState {
  return state;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useAuthoring(): AuthoringState {
  return useSyncExternalStore(subscribe, authoringSnapshot, authoringSnapshot);
}

export function openPicker(kind: "skill" | "agent"): void {
  update({ picker: kind });
}

export function closePicker(): void {
  update({ picker: null });
}

export function openLater(kind: "skill" | "agent"): void {
  update({ later: kind, picker: null });
}

export function closeLater(): void {
  update({ later: null });
}

export function setNotice(notice: string | null): void {
  update({ notice });
}

export function setValidationCounts(findings: { severity: string }[] | null): void {
  if (!findings) {
    update({ counts: null });
    return;
  }
  update({
    counts: {
      errors: findings.filter((item) => item.severity === "error").length,
      warnings: findings.filter((item) => item.severity === "warning").length,
    },
  });
}

function demoSkillDraft(card: SkillCard): SkillDraft {
  return {
    id: card.id,
    name: card.name,
    dir: `/skills/${card.name}`,
    profile: "portable",
    shellPreview: false,
    deployments: [],
    files: [
      {
        path: "SKILL.md",
        executable: false,
        content: `---\nname: ${card.name}\ndescription: ${card.description}\n---\n${card.body}\n`,
      },
    ],
  };
}

function seed(id: string): SkillDraft | null {
  if (id === "warnings") return warningsFixture();
  if (id === "gauge") return longBodyFixture();
  if (id === "preview-skill") return previewFixture();
  if (id === "symlink-skill") return deployFixture(id, "symlink");
  if (id === "copy-skill") return deployFixture(id, "copy");
  if (demoEnabled()) {
    const card = DEMO.skills.find((skill) => skill.id === id);
    if (card) return demoSkillDraft(card);
  }
  return null;
}

export function ensureSkill(id: string): SkillDraft | null {
  if (state.skills[id]) return state.skills[id];
  const drafted = seed(id);
  if (!drafted) return null;
  update({ skills: { ...state.skills, [id]: drafted } });
  return drafted;
}

function adoptServerVersion(id: string, serverId: unknown, label?: string): void {
  if (typeof serverId !== "string" || serverId.length === 0) return;
  const versions = state.versions[id] ?? [];
  const target = label
    ? versions.find((version) => version.label === label && !version.hidden)
    : versions.find((version) => !version.hidden);
  if (!target || target.id === serverId) return;
  update({
    versions: {
      ...state.versions,
      [id]: versions.map((version) => (version.id === target.id ? { ...version, id: serverId } : version)),
    },
  });
}

function hiveFixArgs(draft: SkillDraft, fixId: string): { path: string; message: string } {
  const finding = validateSkill(draft).find((item) => item.fixId === fixId);
  if (fixId === "remove-unknown-key") {
    const key = finding?.message.split(" ")[0] ?? "";
    return { path: "SKILL.md", message: `key \`${key}\` is not allowed in the portable profile` };
  }
  if (fixId === "create-missing-file") {
    const missing = finding?.message.split(" ")[0] ?? "";
    return { path: "SKILL.md", message: `referenced path \`${missing}\` is missing or deeper than one directory` };
  }
  return { path: finding?.path ?? "SKILL.md", message: finding?.message ?? "" };
}

export function saveSkill(draft: SkillDraft): void {
  const versions = saveAutosave(state.versions[draft.id] ?? [], filesRecord(draft));
  update({ skills: { ...state.skills, [draft.id]: draft }, versions: { ...state.versions, [draft.id]: versions } });
  void saveSkillDraft(draft.id, draft.files.map((file) => ({ path: file.path, content: file.content }))).then((serverId) =>
    adoptServerVersion(draft.id, serverId),
  );
}

export function snapshotSkill(id: string, label: string): void {
  const draft = state.skills[id];
  if (!draft) return;
  const versions = snapshotVersion(state.versions[id] ?? [], filesRecord(draft), label);
  update({ versions: { ...state.versions, [id]: versions } });
  void snapshotSkillDraft(id, label).then((serverId) => adoptServerVersion(id, serverId, label));
}

export function restoreSkill(id: string, versionId: string): void {
  const draft = state.skills[id];
  if (!draft) return;
  const restored = restoreVersion(state.versions[id] ?? [], versionId, filesRecord(draft));
  const files = draft.files.map((file) => ({ ...file, content: restored.files[file.path] ?? file.content }));
  for (const [path, content] of Object.entries(restored.files)) {
    if (!files.some((file) => file.path === path)) files.push({ path, content, executable: false });
  }
  update({
    skills: { ...state.skills, [id]: { ...draft, files } },
    versions: { ...state.versions, [id]: restored.versions },
  });
  void restoreSkillDraft(id, versionId);
}

export function hideSkillVersion(id: string, versionId: string): string | null {
  const result = hideVersion(state.versions[id] ?? [], versionId);
  update({ versions: { ...state.versions, [id]: result.versions }, notice: result.error ?? null });
  return result.error ?? null;
}

export function publishSkill(id: string, confirmed: boolean): { kind: "confirm"; agents: string[] } | { kind: "published" } {
  const draft = state.skills[id];
  if (!draft) return { kind: "published" };
  const symlinks = draft.deployments.filter((item) => item.mode === "symlink").map((item) => item.agent);
  if (symlinks.length > 0 && !confirmed) return { kind: "confirm", agents: symlinks };
  const names = draft.deployments.filter((item) => item.mode === "copy").map(() => draft.name);
  update({ centerChanged: [...new Set([...state.centerChanged, ...names])] });
  void publishSkillDraft(
    id,
    confirmed,
    draft.deployments.map((item) => ({ agent: item.agent, mode: item.mode, target: draft.dir })),
  );
  return { kind: "published" };
}

export function createSkill(templateId: string, name: string): SkillDraft {
  const draft = templateDraft(templateId, name);
  update({ skills: { ...state.skills, [draft.id]: draft }, picker: null });
  return draft;
}

export function updateSkill(draft: SkillDraft): void {
  update({ skills: { ...state.skills, [draft.id]: draft } });
}

export function skillFindings(id: string) {
  const draft = state.skills[id];
  if (!draft) return [];
  return validateSkill(draft);
}

export function fixSkill(id: string, fixId: string): void {
  const draft = state.skills[id];
  if (!draft) return;
  const next = applySkillFix(draft, fixId);
  const fix = hiveFixArgs(draft, fixId);
  update({ skills: { ...state.skills, [id]: next } });
  setValidationCounts(validateSkill(next));
  void fixSkillDraft(id, fixId, fix.path, fix.message);
}

export function ensureAgent(id: string): AgentDraft | null {
  if (state.agents[id]) return state.agents[id];
  if (id !== "reviewer") return null;
  const agent = reviewerFixture();
  update({ agents: { ...state.agents, [id]: agent } });
  return agent;
}

export function saveAgent(agent: AgentDraft): void {
  update({ agents: { ...state.agents, [agent.id]: agent } });
  void saveAgentDraft(agent.id, agent.text);
}

export function fixAgent(id: string): void {
  const agent = state.agents[id];
  if (!agent) return;
  saveAgent({ ...agent, text: applyAgentFix(agent.text, "rename-key") });
}

export function exportAgent(id: string): void {
  const agent = state.agents[id];
  if (!agent) return;
  const rendered = renderAgentFile(agent.text);
  saveAgent({ ...agent, exported: rendered, disk: rendered });
  void exportAgentDraft(id, rendered);
}

export function agentState(id: string): ReturnType<typeof exportStatus> {
  const agent = state.agents[id];
  if (!agent) return "missing";
  return exportStatus(renderAgentFile(agent.text), agent.exported, agent.disk);
}

export function importAgent(name: string, mode: "replace" | "copy"): AgentDraft {
  const existing = Object.values(state.agents).find((agent) => agent.name === name);
  if (mode === "copy" || !existing) {
    const id = mode === "copy" && existing ? `${name}-copy` : name;
    const agent: AgentDraft = { ...reviewerFixture(), id, name: id, text: reviewerFixture().text.replace("name: reviewer", `name: ${id}`) };
    update({ agents: { ...state.agents, [id]: agent } });
    return agent;
  }
  return existing;
}

export function agentBudget(): { tokens: number; amber: boolean } {
  return descriptionBudget(Object.values(state.agents).map((agent) => agent.text));
}

export function draftSkillCards(): SkillCard[] {
  return Object.values(state.skills).map((draft) => ({
    id: draft.id,
    name: draft.name,
    description: "Draft",
    source: "draft",
    tags: [],
    enabled: true,
    body: "",
    sourceBody: "",
    agents: draft.deployments.map((item) => item.agent),
    draft: true,
  }));
}

export function centerChangedNames(): string[] {
  return state.centerChanged;
}

export function skillErrorCount(id: string): number {
  const draft = state.skills[id];
  return draft ? errorCount(validateSkill(draft)) : 0;
}

export function skillWarningCount(id: string): number {
  const draft = state.skills[id];
  return draft ? warningCount(validateSkill(draft)) : 0;
}
