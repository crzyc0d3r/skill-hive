export type VerdictKind = "helps-lot" | "helps" | "neutral" | "bit-worse" | "much-worse";

/** Approved cutoffs: a clear help starts at +5 points, a large one at +20. */
export const VERDICT_HELP = 5;
export const VERDICT_HELP_LOT = 20;

export function verdictFromDelta(delta: number | null | undefined): VerdictKind {
  if (delta === null || delta === undefined || Number.isNaN(delta)) return "neutral";
  if (delta >= VERDICT_HELP_LOT) return "helps-lot";
  if (delta >= VERDICT_HELP) return "helps";
  if (delta > -VERDICT_HELP) return "neutral";
  if (delta > -VERDICT_HELP_LOT) return "bit-worse";
  return "much-worse";
}

export interface ScoreAxis {
  name: string;
  value: number | null;
}

/** Value-added axis is the point delta. Without-skill is with-skill minus that delta. */
export function scoreDelta(axes: ScoreAxis[]): { delta: number | null; withScore: number | null; withoutScore: number | null } {
  const added = axes.find((axis) => axis.name.toLowerCase().startsWith("value"));
  const withSkill = axes.find((axis) => axis.name.toLowerCase().startsWith("with"));
  const delta = added?.value ?? null;
  const withScore = withSkill?.value ?? null;
  const withoutScore = withScore !== null && delta !== null ? withScore - delta : null;
  return { delta, withScore, withoutScore };
}
