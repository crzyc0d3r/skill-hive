export interface TimelineStep {
  id: number;
  tool: string;
  useTs: number;
  resultTs: number;
}

export interface HeatCell {
  id: string;
  label: string;
  passRate: number;
}
export const FIXTURE_CELLS: HeatCell[] = [
  { id: "with-1", label: "with 1", passRate: 1 },
  { id: "without-1", label: "without 1", passRate: 0 },
  { id: "with-2", label: "with 2", passRate: 1 },
  { id: "without-2", label: "without 2", passRate: 0.5 },
];

export const FIXTURE_STEPS: TimelineStep[] = [
  { id: 2, tool: "Write", useTs: 10, resultTs: 40 },
  { id: 3, tool: "Read", useTs: 40, resultTs: 55 },
];

export function stepDuration(step: TimelineStep): number {
  return step.resultTs - step.useTs;
}

export function evidenceAnchor(query: string): string | null {
  const trimmed = query.trim();
  if (/^\d+$/.test(trimmed)) return `step-${trimmed}`;
  if (trimmed.startsWith("outputs/")) return trimmed;
  return null;
}

export function moveCell(index: number, key: string, count: number): number {
  if (key === "ArrowRight") return Math.min(index + 1, count - 1);
  if (key === "ArrowLeft") return Math.max(index - 1, 0);
  if (key === "ArrowDown") return Math.min(index + 2, count - 1);
  if (key === "ArrowUp") return Math.max(index - 2, 0);
  return index;
}
