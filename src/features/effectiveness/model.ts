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
};

export function radarPoint(index: number, count: number, value: number): { x: number; y: number } {
  const angle = -Math.PI / 2 + (index * 2 * Math.PI) / Math.max(count, 1);
  const radius = (Math.max(0, Math.min(100, value)) / 100) * 70;
  return { x: 100 + Math.cos(angle) * radius, y: 100 + Math.sin(angle) * radius };
}
