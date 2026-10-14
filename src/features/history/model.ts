export interface HistoryRow {
  version: string;
  parent: string;
  rate: string;
  result: string;
  best: boolean;
}
export const HISTORY_ROWS: HistoryRow[] = [
  { version: "v1", parent: "", rate: "0.92", result: "baseline", best: false },
  { version: "v2", parent: "v1", rate: "0.97", result: "won", best: false },
  { version: "v2-optimized", parent: "v2", rate: "0.97", result: "tie", best: true },
];
