import { t } from "@/lib/i18n";

export function gradeTone(letter: string): string {
  const grade = letter.trim().slice(0, 1).toUpperCase();
  if (grade === "A") return "bg-success-subtle text-success-text";
  if (grade === "B") return "bg-info-subtle text-info-text";
  if (grade === "C") return "bg-warning-subtle text-warning-text";
  return "bg-destructive/15 text-destructive";
}

export function shortAxis(name: string): string {
  if (name.startsWith("Trigger")) return t("effect.axis.trigger");
  if (name.startsWith("Negative")) return t("effect.axis.negative");
  if (name.startsWith("Survival")) return t("effect.axis.survival");
  if (name.startsWith("With")) return t("effect.axis.with");
  if (name.startsWith("Value")) return t("effect.axis.value");
  if (name.startsWith("Consistency")) return t("effect.axis.consistency");
  if (name.startsWith("Weak")) return t("effect.axis.weak");
  if (name.startsWith("Efficiency")) return t("effect.axis.efficiency");
  return name;
}

export interface AxisScore {
  name: string;
  value: number | null;
}

export interface InfluenceNode {
  id: string;
  won: number;
  grade: string;
  score: number;
}

export interface InfluenceEdge {
  source: string;
  target: string;
  steal_rate: number;
  queries: string[];
}

export interface BoardRow {
  rank: number;
  skill: string;
  fires: number;
  quality: number;
  score: number;
}

export interface Activation {
  query_id: string;
  query: string;
  outcome: string;
  transcript: string;
  culprit?: string | null;
}

export interface ConflictCell {
  owner: string;
  other: string;
  stole: number;
  co_fired: number;
  neither: number;
}

export interface InstallWarning {
  query_id: string;
  before: string;
  after: string;
  suspect: string;
}

export interface Interference {
  bisect: string[];
  warnings: InstallWarning[];
  changed: string[];
}

export interface EffectivenessReport {
  grade: string;
  grade_score: number;
  verdict: string;
  weakest_axis: string;
  thickest_arrow: string;
  leader_name: string;
  leader_factor: string;
  omitted_axes: string[];
  axes: AxisScore[];
  influence: { nodes: InfluenceNode[]; edges: InfluenceEdge[] };
  leaderboard: BoardRow[];
  activations: Activation[];
  conflicts?: ConflictCell[];
  interference?: Interference;
  overlay?: { name: string; axes: AxisScore[] };
}

export const SAMPLE_REPORT: EffectivenessReport = {
  grade: "B",
  grade_score: 82,
  verdict: "hijacked",
  weakest_axis: "Survival in a crowded library",
  thickest_arrow: "culprit -> pine",
  leader_name: "pine",
  leader_factor: "graded task quality",
  omitted_axes: [],
  axes: [
    { name: "Trigger pass rate", value: 50 },
    { name: "Negative-query pass rate", value: 100 },
    { name: "Survival in a crowded library", value: 50 },
    { name: "With-skill task pass rate", value: 100 },
    { name: "Value added over without_skill", value: 75 },
    { name: "Consistency", value: 100 },
    { name: "Weak-model performance", value: 40 },
    { name: "Efficiency", value: 90 },
  ],
  influence: {
    nodes: [
      { id: "pine", won: 1, grade: "B", score: 82 },
      { id: "culprit", won: 0, grade: "D", score: 61 },
    ],
    edges: [{ source: "culprit", target: "pine", steal_rate: 0.5, queries: ["q2"] }],
  },
  leaderboard: [
    { rank: 1, skill: "pine", fires: 0.5, quality: 1, score: 0.5 },
    { rank: 2, skill: "culprit", fires: 0.2, quality: 0.4, score: 0.08 },
  ],
  activations: [
    {
      query_id: "q2",
      query: "hijack-target the pine question",
      outcome: "hijacked",
      transcript: "triggers/library-q2.jsonl",
      culprit: "culprit",
    },
  ],
  conflicts: [{ owner: "pine", other: "culprit", stole: 1, co_fired: 0, neither: 0 }],
  interference: {
    bisect: ["culprit"],
    warnings: [{ query_id: "q2", before: "fired_alone", after: "hijacked", suspect: "culprit" }],
    changed: ["culprit"],
  },
  overlay: {
    name: "culprit",
    axes: [
      { name: "Trigger pass rate", value: 20 },
      { name: "Negative-query pass rate", value: 40 },
      { name: "Survival in a crowded library", value: 10 },
      { name: "With-skill task pass rate", value: 30 },
      { name: "Value added over without_skill", value: 15 },
      { name: "Consistency", value: 25 },
      { name: "Weak-model performance", value: 10 },
      { name: "Efficiency", value: 35 },
    ],
  },
};

export function radarPoint(index: number, count: number, value: number): { x: number; y: number } {
  const angle = -Math.PI / 2 + (index * 2 * Math.PI) / Math.max(count, 1);
  const radius = (Math.max(0, Math.min(100, value)) / 100) * 70;
  return { x: 100 + Math.cos(angle) * radius, y: 100 + Math.sin(angle) * radius };
}
