import type { SkillCard } from "@/features/library/model";

const STORAGE_KEY = "skill-hive-library-index";
const memory: { skills: SkillCard[] | null } = { skills: null };
const agentsMemory: { rows: unknown[] | null } = { rows: null };

/** Keep the index small. Bodies and files load when a skill is opened. */
export function slimSkill(skill: SkillCard): SkillCard {
  return {
    ...skill,
    description: skill.description.replace(/\s+/g, " ").trim().slice(0, 180),
    body: "",
    sourceBody: "",
    files: [],
  };
}

export function readLibraryIndex(): SkillCard[] | null {
  if (memory.skills) return memory.skills;
  if (typeof localStorage === "undefined") return null;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as SkillCard[];
    if (!Array.isArray(parsed)) return null;
    memory.skills = parsed.map(slimSkill);
    return memory.skills;
  } catch {
    return null;
  }
}

export function writeLibraryIndex(skills: SkillCard[]): SkillCard[] {
  const slim = skills.map(slimSkill);
  memory.skills = slim;
  if (typeof localStorage !== "undefined") {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(slim));
    } catch {
      /* a full disk should not block the library */
    }
  }
  return slim;
}

export function clearLibraryMemory(): void {
  memory.skills = null;
}

export function readAgentCache<T>(): T[] | null {
  return (agentsMemory.rows as T[] | null) ?? null;
}

export function writeAgentCache<T>(rows: T[]): T[] {
  agentsMemory.rows = rows;
  return rows;
}
