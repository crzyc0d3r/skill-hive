export interface RegressionRow {
  skillId: string;
  skill: string;
  rule: string;
  message: string;
}

export interface HistoryRow {
  version: string;
  parent: string;
  rate: string;
  result: string;
  best: boolean;
}

export interface Phase5Seed {
  tested: string[];
  regressed: string[];
  regressions: RegressionRow[];
}

export const EMPTY_SEED: Phase5Seed = { tested: [], regressed: [], regressions: [] };

export const HISTORY_ROWS: HistoryRow[] = [
  { version: "v1", parent: "", rate: "0.92", result: "baseline", best: false },
  { version: "v2", parent: "v1", rate: "0.97", result: "won", best: false },
  { version: "v2-optimized", parent: "v2", rate: "0.97", result: "tie", best: true },
];

export function parseSeed(raw: string | null): Phase5Seed {
  if (!raw) return EMPTY_SEED;
  try {
    const parsed = JSON.parse(raw) as Partial<Phase5Seed>;
    return {
      tested: parsed.tested ?? [],
      regressed: parsed.regressed ?? [],
      regressions: parsed.regressions ?? [],
    };
  } catch {
    return EMPTY_SEED;
  }
}

export function badgeFor(seed: Phase5Seed, skillId: string): "regressed" | "tested" | null {
  if (seed.regressed.includes(skillId)) return "regressed";
  if (seed.tested.includes(skillId)) return "tested";
  return null;
}
