export interface RegressionRow {
  skillId: string;
  skill: string;
  rule: string;
  message: string;
}
export interface GenerateSeed {
  tested: string[];
  regressed: string[];
  regressions: RegressionRow[];
}

export const EMPTY_SEED: GenerateSeed = { tested: [], regressed: [], regressions: [] };
export function parseSeed(raw: string | null): GenerateSeed {
  if (!raw) return EMPTY_SEED;
  try {
    const parsed = JSON.parse(raw) as Partial<GenerateSeed>;
    return {
      tested: parsed.tested ?? [],
      regressed: parsed.regressed ?? [],
      regressions: parsed.regressions ?? [],
    };
  } catch {
    return EMPTY_SEED;
  }
}

export function badgeFor(seed: GenerateSeed, skillId: string): "regressed" | "tested" | null {
  if (seed.regressed.includes(skillId)) return "regressed";
  if (seed.tested.includes(skillId)) return "tested";
  return null;
}
