// Recharts views shared by the eval screens. Colours come from the --chart-n tokens so they
// hold up in both themes; grids and axes are recessive hairlines.

import * as React from "react";
import { Bar, BarChart, CartesianGrid, Cell, ErrorBar, Line, LineChart, ResponsiveContainer, Tooltip as ChartTooltip, XAxis, YAxis } from "recharts";
import type * as T from "@/lib/api/bindings";
import { formatDate, formatPercent } from "@/lib/utils";
import { ArmSwatch } from "./bits";
import { armColor, armColors, armLabel } from "./shared";

const AXIS_TICK = { fill: "var(--fg-muted)", fontSize: 11 } as const;
const AXIS_LINE = { stroke: "var(--line)" } as const;

function ChartLegend({ entries }: { entries: { label: string; color: string }[] }) {
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px] text-muted">
      {entries.map((e) => (
        <span key={e.label} className="inline-flex items-center gap-1.5">
          <ArmSwatch color={e.color} />
          {e.label}
        </span>
      ))}
    </div>
  );
}

function TipFrame({ title, rows }: { title: React.ReactNode; rows: { label: string; value: React.ReactNode; color?: string }[] }) {
  return (
    <div className="surface px-3 py-2 text-xs shadow-md">
      <div className="mb-1 font-medium">{title}</div>
      {rows.map((r) => (
        <div key={r.label} className="flex items-center justify-between gap-4">
          <span className="inline-flex items-center gap-1.5 text-muted">
            {r.color ? <ArmSwatch color={r.color} /> : null}
            {r.label}
          </span>
          <span className="num">{r.value}</span>
        </div>
      ))}
    </div>
  );
}

interface HistoryDatum {
  key: string;
  date: string;
  withSkill: number;
  baseline: number;
  point: T.HistoryPoint;
}

/** With-skill vs baseline pass rate over finished runs. Clicking a column opens that run. */
export function HistoryChart({ points, onSelect, height = 200 }: { points: T.HistoryPoint[]; onSelect?: (runId: string) => void; height?: number }) {
  const data = React.useMemo<HistoryDatum[]>(
    () =>
      [...points]
        .sort((a, b) => a.finishedAt - b.finishedAt)
        .map((p) => ({
          key: p.runId,
          date: new Date(p.finishedAt).toLocaleDateString(undefined, { month: "short", day: "numeric" }),
          withSkill: Math.round(p.withSkillPassRate * 100),
          baseline: Math.round(p.baselinePassRate * 100),
          point: p,
        })),
    [points],
  );
  const byKey = React.useMemo(() => new Map(data.map((d) => [d.key, d])), [data]);
  const withColor = armColor("with_skill");
  const baseColor = armColor("without_skill");

  return (
    <div className="flex flex-col gap-2">
      <ChartLegend entries={[{ label: "With skill", color: withColor }, { label: "Baseline", color: baseColor }]} />
      <div style={{ height, cursor: onSelect ? "pointer" : undefined }} role="img" aria-label="Pass rate history">
        <ResponsiveContainer width="100%" height="100%" initialDimension={{ width: 600, height }}>
          <LineChart
            data={data}
            margin={{ top: 8, right: 12, bottom: 0, left: 0 }}
            onClick={(state) => {
              const idx = state?.activeTooltipIndex;
              const d = typeof idx === "number" ? data[idx] : typeof idx === "string" ? data[Number(idx)] : undefined;
              if (d && onSelect) onSelect(d.key);
            }}
          >
            <CartesianGrid vertical={false} stroke="var(--line)" />
            <XAxis dataKey="key" tickFormatter={(k: string) => byKey.get(k)?.date ?? ""} tick={AXIS_TICK} axisLine={AXIS_LINE} tickLine={false} interval="preserveStartEnd" minTickGap={24} />
            <YAxis domain={[0, 100]} ticks={[0, 25, 50, 75, 100]} tickFormatter={(v: number) => `${v}%`} tick={AXIS_TICK} axisLine={false} tickLine={false} width={40} />
            <ChartTooltip
              cursor={{ stroke: "var(--line-strong)" }}
              content={({ active, payload }) => {
                const d = payload?.[0]?.payload as HistoryDatum | undefined;
                if (!active || !d) return null;
                return (
                  <TipFrame
                    title={formatDate(d.point.finishedAt)}
                    rows={[
                      { label: "With skill", value: `${d.withSkill}%`, color: withColor },
                      { label: "Baseline", value: `${d.baseline}%`, color: baseColor },
                      { label: "Executor", value: <span className="max-w-[160px] truncate">{d.point.executor}</span> },
                    ]}
                  />
                );
              }}
            />
            <Line type="monotone" dataKey="baseline" name="Baseline" stroke={baseColor} strokeWidth={2} dot={{ r: 3.5, fill: baseColor, stroke: "var(--bg-elevated)", strokeWidth: 2 }} activeDot={{ r: 5, strokeWidth: 0 }} isAnimationActive={false} />
            <Line type="monotone" dataKey="withSkill" name="With skill" stroke={withColor} strokeWidth={2} dot={{ r: 3.5, fill: withColor, stroke: "var(--bg-elevated)", strokeWidth: 2 }} activeDot={{ r: 5, strokeWidth: 0 }} isAnimationActive={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

interface ArmDatum {
  key: string;
  label: string;
  rate: number;
  sd: number;
  arm: T.ArmSummary;
}

/** Pass rate per arm with ± one standard deviation. */
export function ArmPassRateChart({ arms, height = 220 }: { arms: T.ArmSummary[]; height?: number }) {
  const colors = React.useMemo(() => armColors(arms.map((a) => a.label)), [arms]);
  const data = React.useMemo<ArmDatum[]>(
    () => arms.map((a) => ({ key: a.label, label: armLabel(a.label), rate: Math.round(a.passRate.mean * 100), sd: Math.round(a.passRate.stddev * 100), arm: a })),
    [arms],
  );
  return (
    <div className="flex flex-col gap-2">
      <ChartLegend entries={data.map((d) => ({ label: d.label, color: colors[d.key] }))} />
      <div style={{ height }} role="img" aria-label="Pass rate per arm">
        <ResponsiveContainer width="100%" height="100%" initialDimension={{ width: 400, height }}>
          <BarChart data={data} margin={{ top: 12, right: 12, bottom: 0, left: 0 }} barCategoryGap="30%">
            <CartesianGrid vertical={false} stroke="var(--line)" />
            <XAxis dataKey="label" tick={AXIS_TICK} axisLine={AXIS_LINE} tickLine={false} interval={0} />
            <YAxis domain={[0, 100]} ticks={[0, 25, 50, 75, 100]} tickFormatter={(v: number) => `${v}%`} tick={AXIS_TICK} axisLine={false} tickLine={false} width={40} />
            <ChartTooltip
              cursor={{ fill: "var(--bg-sunken)" }}
              content={({ active, payload }) => {
                const d = payload?.[0]?.payload as ArmDatum | undefined;
                if (!active || !d) return null;
                return (
                  <TipFrame
                    title={d.label}
                    rows={[
                      { label: "Pass rate", value: `${formatPercent(d.arm.passRate.mean)} ± ${d.sd}`, color: colors[d.key] },
                      { label: "Runs", value: `${d.arm.runs}${d.arm.errors ? ` (${d.arm.errors} errors)` : ""}` },
                      { label: "Range", value: `${formatPercent(d.arm.passRate.min)} – ${formatPercent(d.arm.passRate.max)}` },
                    ]}
                  />
                );
              }}
            />
            <Bar dataKey="rate" radius={[4, 4, 0, 0]} maxBarSize={44} isAnimationActive={false}>
              {data.map((d) => (
                <Cell key={d.key} fill={colors[d.key]} />
              ))}
              <ErrorBar dataKey="sd" width={6} strokeWidth={1.5} stroke="var(--fg-muted)" />
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
