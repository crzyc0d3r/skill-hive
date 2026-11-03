import type * as T from "@/lib/api/bindings";
import { formatDate, formatPercent } from "@/lib/utils";

/** Compact SVG line chart of with-skill vs baseline pass rate over runs. */
export function HistorySparkline({ points, height = 64 }: { points: T.HistoryPoint[]; height?: number }) {
  if (points.length === 0) return null;
  const w = 320;
  const pad = 6;
  const n = points.length;
  const x = (i: number) => (n === 1 ? w / 2 : pad + (i / (n - 1)) * (w - pad * 2));
  const y = (v: number) => height - pad - v * (height - pad * 2);
  const path = (key: "withSkillPassRate" | "baselinePassRate") => points.map((p, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(p[key]).toFixed(1)}`).join(" ");
  return (
    <svg viewBox={`0 0 ${w} ${height}`} className="h-16 w-full" role="img" aria-label="Pass rate history">
      <line x1={pad} x2={w - pad} y1={y(0.5)} y2={y(0.5)} stroke="var(--line)" strokeDasharray="3 3" />
      <path d={path("baselinePassRate")} fill="none" stroke="var(--fg-subtle)" strokeWidth={1.5} />
      <path d={path("withSkillPassRate")} fill="none" stroke="var(--accent)" strokeWidth={2} />
      {points.map((p, i) => (
        <g key={p.runId}>
          <circle cx={x(i)} cy={y(p.withSkillPassRate)} r={3} fill="var(--accent)">
            <title>{`${formatDate(p.finishedAt)} · with skill ${formatPercent(p.withSkillPassRate)} · baseline ${formatPercent(p.baselinePassRate)}`}</title>
          </circle>
          <circle cx={x(i)} cy={y(p.baselinePassRate)} r={2.5} fill="var(--fg-subtle)" />
        </g>
      ))}
    </svg>
  );
}
