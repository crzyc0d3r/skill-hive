export interface SkillFile {
  path: string;
  local: string;
  remote: string;
}

export interface SkillCard {
  id: string;
  name: string;
  description: string;
  source: string;
  tags: string[];
  enabled: boolean;
  body: string;
  sourceBody: string;
  agents: string[];
  files?: SkillFile[];
  draft?: boolean;
}

export type LibraryFilter = "all" | "enabled" | "available";

export interface DiffRow {
  kind: "same" | "add" | "del";
  text: string;
}

export function filterSkills(
  skills: SkillCard[],
  query: string,
  filter: LibraryFilter,
  source: string | null,
  tag: string | null,
): SkillCard[] {
  const needle = query.trim().toLowerCase();
  return skills.filter((skill) => {
    if (filter === "enabled" && !skill.enabled) return false;
    if (filter === "available" && skill.enabled) return false;
    if (source && skill.source !== source) return false;
    if (tag === "untagged" && skill.tags.length > 0) return false;
    if (tag && tag !== "untagged" && !skill.tags.includes(tag)) return false;
    if (!needle) return true;
    return (
      skill.name.toLowerCase().includes(needle) ||
      skill.description.toLowerCase().includes(needle) ||
      skill.tags.some((item) => item.toLowerCase().includes(needle))
    );
  });
}

export function lcsDiff(left: string, right: string): DiffRow[] {
  const a = left.split("\n");
  const b = right.split("\n");
  const rows = a.length;
  const cols = b.length;
  const table: number[][] = Array.from({ length: rows + 1 }, () => Array(cols + 1).fill(0));
  for (let i = rows - 1; i >= 0; i -= 1) {
    for (let j = cols - 1; j >= 0; j -= 1) {
      table[i][j] = a[i] === b[j] ? table[i + 1][j + 1] + 1 : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }
  const out: DiffRow[] = [];
  let i = 0;
  let j = 0;
  while (i < rows && j < cols) {
    if (a[i] === b[j]) {
      out.push({ kind: "same", text: a[i] });
      i += 1;
      j += 1;
    } else if (table[i + 1][j] >= table[i][j + 1]) {
      out.push({ kind: "del", text: a[i] });
      i += 1;
    } else {
      out.push({ kind: "add", text: b[j] });
      j += 1;
    }
  }
  while (i < rows) {
    out.push({ kind: "del", text: a[i] });
    i += 1;
  }
  while (j < cols) {
    out.push({ kind: "add", text: b[j] });
    j += 1;
  }
  return out;
}

export type PresetFill = "active" | "partial" | "inactive";

export function presetFill(synced: number, total: number): PresetFill {
  if (total > 0 && synced === total) return "active";
  if (synced > 0) return "partial";
  return "inactive";
}

export function moveItem<T>(items: T[], from: number, to: number): T[] {
  if (from < 0 || to < 0 || from >= items.length || to >= items.length || from === to) return items;
  const next = items.slice();
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

export function moveById<T>(items: T[], fromId: string, toId: string, idOf: (item: T) => string): T[] {
  return moveItem(items, items.findIndex((item) => idOf(item) === fromId), items.findIndex((item) => idOf(item) === toId));
}

export function fileDiffs(files: SkillFile[]): { path: string; rows: DiffRow[] }[] {
  return files.map((file) => ({ path: file.path, rows: lcsDiff(file.local, file.remote) }));
}

export function presetLabel(name: string, synced: number, total: number): string {
  const fill = presetFill(synced, total);
  if (fill === "active") return `${name} check`;
  if (fill === "partial") return `${name} ${synced}/${total}`;
  return name;
}
