import { useState } from "react";
import { cn } from "cn";
import { t } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { RADAR, axisLabelLines, gradeTone, radarLabelAnchor, radarPoint, shortAxis, type AxisScore, type EffectivenessReport, type InfluenceEdge } from "@/features/effectiveness/model";

const CX = RADAR.center;
const CY = RADAR.center;
const PLOT = RADAR.plot;

function placed(index: number, count: number, value: number, scale: number): { x: number; y: number } {
  const raw = radarPoint(index, count, value);
  return { x: CX + (raw.x - 100) * scale, y: CY + (raw.y - 100) * scale };
}

function polygon(axes: AxisScore[], scale: number): string {
  return axes
    .map((axis, index) => (axis.value == null ? null : placed(index, axes.length, axis.value, scale)))
    .filter((point): point is { x: number; y: number } => point != null)
    .map((point) => `${point.x},${point.y}`)
    .join(" ");
}

export function GradeMark({ letter, score, hero, testId }: { letter: string; score: number; hero?: boolean; testId?: string }) {
  return (
    <div data-testid={testId} className="flex items-center gap-4">
      <span
        className={cn(
          "flex items-center justify-center rounded-2xl font-mono font-semibold",
          hero ? "size-24 text-5xl" : "size-14 text-2xl",
          gradeTone(letter),
        )}
      >
        <span className="sr-only">{t("effect.grade")} </span>
        {letter}
      </span>
      <div>
        <div className={cn("font-mono font-semibold tabular-nums", hero ? "text-4xl leading-10" : "text-2xl leading-8")}>{Math.round(score)}</div>
        <div className="text-xs text-muted-foreground">{t("effect.grade")}</div>
      </div>
    </div>
  );
}

export function RadarFigure({ report }: { report: EffectivenessReport }) {
  const axes = report.axes ?? [];
  const overlay = report.overlay?.axes ?? [];
  const count = Math.max(axes.length, 1);
  const rings = [25, 50, 75, 100].map((value) =>
    Array.from({ length: count }, (_, index) => placed(index, count, value, PLOT))
      .map((point) => `${point.x},${point.y}`)
      .join(" "),
  );
  const primary = polygon(axes, PLOT);
  const overlayShape = polygon(overlay, PLOT);
  const caption = t("effect.caption.radar", { axis: report.weakest_axis });
  return (
    <figure className="flex flex-col gap-3">
      <svg
        role="img"
        aria-label={caption}
        viewBox={`0 0 ${RADAR.size} ${RADAR.size}`}
        className="mx-auto aspect-square w-full"
      >
        {rings.map((points) => (
          <polygon key={points} points={points} fill="none" stroke="var(--color-border)" strokeWidth="1" />
        ))}
        {Array.from({ length: count }, (_, index) => {
          const end = placed(index, count, 100, PLOT);
          return <line key={index} x1={CX} y1={CY} x2={end.x} y2={end.y} stroke="var(--color-border)" strokeWidth="1" />;
        })}
        {primary ? <polygon points={primary} fill="var(--color-chart-1)" fillOpacity="0.18" stroke="var(--color-chart-1)" strokeWidth="3" /> : null}
        {overlayShape ? (
          <polygon data-testid="effect-overlay" points={overlayShape} fill="none" stroke="var(--color-chart-2)" strokeWidth="3" strokeDasharray="8 5" />
        ) : null}
        {axes.map((axis, index) => {
          if (axis.value == null) return null;
          const point = placed(index, count, axis.value, PLOT);
          return <circle key={axis.name} cx={point.x} cy={point.y} r="6" fill="var(--color-chart-1)" stroke="var(--color-card)" strokeWidth="2" />;
        })}
        {overlay.map((axis, index) => {
          if (axis.value == null) return null;
          const point = placed(index, overlay.length, axis.value, PLOT);
          return <rect key={`overlay-${axis.name}`} x={point.x - 5} y={point.y - 5} width="10" height="10" fill="var(--color-card)" stroke="var(--color-chart-2)" strokeWidth="2.5" />;
        })}
        {axes.map((axis, index) => {
          const lines = axisLabelLines(shortAxis(axis.name));
          const place = radarLabelAnchor(index, count, lines.length);
          return (
            <text
              key={`label-${axis.name}`}
              textAnchor={place.anchor}
              fill="var(--color-foreground)"
              stroke="var(--color-card)"
              strokeWidth="4"
              paintOrder="stroke"
              fontSize={RADAR.fontSize}
              fontFamily="var(--font-sans)"
            >
              {lines.map((line, lineIndex) => (
                <tspan key={line} x={place.x} y={place.y + lineIndex * RADAR.lineHeight}>
                  {line}
                </tspan>
              ))}
            </text>
          );
        })}
      </svg>
      <ul aria-label={t("effect.legend")} className="flex flex-wrap items-center gap-4 text-sm">
        <li className="flex items-center gap-2">
          <span className="size-3 rounded-full bg-chart-1" aria-hidden="true" />
          {t("effect.series.skill")}
        </li>
        {report.overlay?.name ? (
          <li className="flex items-center gap-2">
            <span className="size-3 rotate-45 border-2 border-chart-2" aria-hidden="true" />
            {t("effect.overlay")}: {report.overlay.name}
          </li>
        ) : null}
      </ul>
    </figure>
  );
}

export function AxisTable({ report }: { report: EffectivenessReport }) {
  const overlay = report.overlay?.axes ?? [];
  return (
    <div className="overflow-hidden rounded-md border border-border">
      <Table id="effect-radar-table" tabIndex={0}>
        <TableCaption className="sr-only">{t("effect.radar")}</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>{t("effect.radar")}</TableHead>
            <TableHead className="text-right">{t("effect.series.skill")}</TableHead>
            {report.overlay?.name ? <TableHead className="text-right">{report.overlay.name}</TableHead> : null}
          </TableRow>
        </TableHeader>
        <TableBody>
          {(report.axes ?? []).map((axis, index) => {
            const other = overlay[index]?.value;
            return (
              <TableRow key={axis.name} className="odd:bg-muted/30">
                <TableCell className="whitespace-normal font-medium">{shortAxis(axis.name)}</TableCell>
                <TableCell>
                  <span className="flex items-center justify-end gap-3">
                    <span className="hidden h-2 w-24 overflow-hidden rounded-full bg-muted sm:block" aria-hidden="true">
                      <span className="block h-2 rounded-full bg-chart-1" style={{ width: `${axis.value ?? 0}%` }} />
                    </span>
                    <span className="w-8 text-right font-mono tabular-nums">{axis.value == null ? "—" : axis.value}</span>
                  </span>
                </TableCell>
                {report.overlay?.name ? (
                  <TableCell>
                    <span className="flex items-center justify-end gap-3">
                      <span className="hidden h-2 w-24 overflow-hidden rounded-full bg-muted sm:block" aria-hidden="true">
                        <span className="block h-2 rounded-full bg-chart-2" style={{ width: `${other ?? 0}%` }} />
                      </span>
                      <span className="w-8 text-right font-mono tabular-nums">{other == null ? "—" : other}</span>
                    </span>
                  </TableCell>
                ) : null}
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}

function nodeSpot(index: number, count: number): { x: number; y: number } {
  if (count <= 1) return { x: 320, y: 120 };
  if (count === 2) return { x: index === 0 ? 150 : 490, y: 120 };
  const angle = -Math.PI / 2 + (index * 2 * Math.PI) / count;
  return { x: 320 + Math.cos(angle) * 210, y: 120 + Math.sin(angle) * 78 };
}

export function InfluenceMap({ report }: { report: EffectivenessReport }) {
  const nodes = report.influence?.nodes ?? [];
  const edges = report.influence?.edges ?? [];
  const [node, setNode] = useState(0);
  const [edge, setEdge] = useState<InfluenceEdge | null>(null);
  const selected = nodes[node] ?? nodes[0];
  return (
    <div className="flex flex-col gap-3">
      <div
        role="application"
        tabIndex={0}
        data-testid="effect-map"
        className="relative h-56 rounded-md border border-border bg-muted/30 outline-none focus-visible:ring-2 focus-visible:ring-ring"
        onKeyDown={(event) => {
          if (event.key === "ArrowRight" || event.key === "ArrowDown") {
            event.preventDefault();
            setNode((value) => (nodes.length ? (value + 1) % nodes.length : 0));
          }
          if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
            event.preventDefault();
            setNode((value) => (nodes.length ? (value - 1 + nodes.length) % nodes.length : 0));
          }
          if (event.key === "Enter" && selected) {
            const match = edges.find((item) => item.source === selected.id || item.target === selected.id) ?? null;
            setEdge(match);
          }
        }}
      >
        <svg viewBox="0 0 640 240" className="absolute inset-0 h-full w-full" aria-hidden="true" preserveAspectRatio="none">
          <defs>
            <marker id="influence-arrow" markerWidth="8" markerHeight="8" refX="7" refY="3" orient="auto">
              <path d="M0,0 L7,3 L0,6 Z" fill="var(--color-chart-3)" />
            </marker>
          </defs>
          {edges.map((item) => {
            const from = nodeSpot(Math.max(0, nodes.findIndex((entry) => entry.id === item.source)), nodes.length);
            const to = nodeSpot(Math.max(0, nodes.findIndex((entry) => entry.id === item.target)), nodes.length);
            const dx = to.x - from.x;
            const dy = to.y - from.y;
            const length = Math.hypot(dx, dy) || 1;
            const ux = dx / length;
            const uy = dy / length;
            const x1 = from.x + ux * 78;
            const y1 = from.y + uy * 28;
            const x2 = to.x - ux * 78;
            const y2 = to.y - uy * 28;
            const midX = (x1 + x2) / 2;
            const midY = (y1 + y2) / 2 - 16;
            return (
              <g key={`${item.source}-${item.target}`}>
                <line x1={x1} y1={y1} x2={x2} y2={y2} stroke="var(--color-chart-3)" strokeWidth={2 + item.steal_rate * 8} markerEnd="url(#influence-arrow)" />
                <rect x={midX - 28} y={midY - 12} width="56" height="20" rx="6" fill="var(--color-card)" stroke="var(--color-border)" />
                <text x={midX} y={midY + 3} textAnchor="middle" fill="var(--color-foreground)" fontSize="13" fontFamily="var(--font-mono)">
                  {Math.round(item.steal_rate * 100)}%
                </text>
              </g>
            );
          })}
        </svg>
        {nodes.map((item, index) => {
          const spot = nodeSpot(index, nodes.length);
          return (
            <button
              key={item.id}
              type="button"
              data-selected={index === node}
              className={cn(
                "absolute flex w-28 -translate-x-1/2 -translate-y-1/2 flex-col items-center rounded-xl border border-border bg-card px-2 py-2 text-center shadow-sm",
                index === node && "ring-2 ring-ring",
              )}
              style={{ left: `${(spot.x / 640) * 100}%`, top: `${(spot.y / 240) * 100}%` }}
              onClick={() => setNode(index)}
            >
              <span className={cn("flex size-8 items-center justify-center rounded-lg font-mono text-base font-semibold", gradeTone(item.grade))}>{item.grade}</span>
              <span className="mt-1 truncate text-sm font-medium">{item.id}</span>
              <span className="font-mono text-xs tabular-nums text-muted-foreground">{Math.round(item.score)}</span>
            </button>
          );
        })}
      </div>
      <ul className="flex flex-wrap gap-2">
        {edges.map((item) => (
          <li key={`${item.source}-${item.target}`}>
            <Button type="button" variant="secondary" size="sm" onClick={() => setEdge(item)}>
              {item.source} → {item.target} {Math.round(item.steal_rate * 100)}%
            </Button>
          </li>
        ))}
      </ul>
      {edge ? (
        <div data-testid="effect-stolen" className="rounded-md border border-border bg-muted/30 px-3 py-2">
          <h3 className="text-sm font-medium">{t("effect.queries")}</h3>
          <ul className="mt-1 space-y-1 text-sm">
            {edge.queries.map((id) => {
              const activation = report.activations?.find((row) => row.query_id === id);
              return (
                <li key={id} className="flex flex-wrap items-baseline justify-between gap-2">
                  <span>{activation?.query ?? id}</span>
                  {activation?.transcript ? <span className="font-mono text-xs text-muted-foreground">{activation.transcript}</span> : null}
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}
      <Button type="button" size="sm" variant="secondary" className="w-fit" onClick={() => document.getElementById("effect-map-table")?.focus()}>
        {t("effect.showTable")}
      </Button>
    </div>
  );
}

export function MapTable({ report }: { report: EffectivenessReport }) {
  const nodes = report.influence?.nodes ?? [];
  const edges = report.influence?.edges ?? [];
  return (
    <div className="overflow-hidden rounded-md border border-border">
      <Table id="effect-map-table" tabIndex={0}>
        <TableCaption className="sr-only">{t("effect.map")}</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>{t("effect.col.source")}</TableHead>
            <TableHead>{t("effect.col.target")}</TableHead>
            <TableHead>{t("effect.col.steal")}</TableHead>
            <TableHead>{t("effect.col.won")}</TableHead>
            <TableHead>{t("effect.col.grade")}</TableHead>
            <TableHead>{t("effect.col.transcript")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {edges.map((item) => {
            const target = nodes.find((entry) => entry.id === item.target);
            const transcript = report.activations?.find((row) => item.queries.includes(row.query_id))?.transcript ?? "";
            return (
              <TableRow key={`${item.source}-${item.target}`} className="odd:bg-muted/30">
                <TableCell>{item.source}</TableCell>
                <TableCell>{item.target}</TableCell>
                <TableCell className="font-mono tabular-nums">{Math.round(item.steal_rate * 100)}%</TableCell>
                <TableCell className="font-mono tabular-nums">{target?.won ?? 0}</TableCell>
                <TableCell>{target?.grade}</TableCell>
                <TableCell className="whitespace-normal font-mono text-xs">{transcript}</TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}

export function Leaderboard({ report }: { report: EffectivenessReport }) {
  const rows = report.leaderboard ?? [];
  const top = Math.max(...rows.map((row) => row.score), 0.01);
  return (
    <div className="overflow-hidden rounded-md border border-border">
      <Table>
        <TableCaption className="sr-only">{t("effect.board")}</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>{t("effect.col.rank")}</TableHead>
            <TableHead>{t("effect.col.skill")}</TableHead>
            <TableHead>{t("effect.col.fires")}</TableHead>
            <TableHead>{t("effect.col.quality")}</TableHead>
            <TableHead>{t("effect.col.score")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row) => (
            <TableRow key={row.skill} data-testid={`board-${row.skill}`} className="odd:bg-muted/30">
              <TableCell className="font-mono text-base font-semibold tabular-nums">{row.rank}</TableCell>
              <TableCell>
                <div className="font-medium">{row.skill}</div>
                <div className="mt-1 h-1.5 w-full max-w-40 overflow-hidden rounded-full bg-muted" aria-hidden="true">
                  <div className="h-1.5 rounded-full bg-chart-1" style={{ width: `${Math.round((row.score / top) * 100)}%` }} />
                </div>
              </TableCell>
              <TableCell className="font-mono tabular-nums">{row.fires}</TableCell>
              <TableCell className="font-mono tabular-nums">{row.quality}</TableCell>
              <TableCell className="font-mono font-semibold tabular-nums">{row.score}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

function heatTone(value: number): string {
  if (value >= 1) return "bg-destructive/15 text-destructive";
  if (value > 0) return "bg-warning-subtle text-warning-text";
  return "bg-muted/40 text-muted-foreground";
}

export function ConflictMatrix({ report }: { report: EffectivenessReport }) {
  const cells = report.conflicts ?? [];
  return (
    <div className="flex flex-col gap-3">
      {cells.map((cell) => (
        <div key={`${cell.owner}-${cell.other}`} className="grid gap-2 sm:grid-cols-[minmax(0,10rem)_1fr_1fr_1fr]">
          <div className="flex items-center font-mono text-sm">
            {cell.owner} → {cell.other}
          </div>
          {[
            [t("effect.col.stole"), cell.stole],
            [t("effect.col.cofired"), cell.co_fired],
            [t("effect.col.neither"), cell.neither],
          ].map(([label, value]) => (
            <div key={String(label)} className={cn("flex flex-col items-start rounded-md border border-border px-3 py-2", heatTone(Number(value)))}>
              <span className="text-xs">{label}</span>
              <span className="font-mono text-2xl font-semibold tabular-nums">{value}</span>
            </div>
          ))}
        </div>
      ))}
      <div className="overflow-hidden rounded-md border border-border">
        <Table>
          <TableCaption className="sr-only">{t("effect.conflicts")}</TableCaption>
          <TableHeader>
            <TableRow>
              <TableHead>{t("effect.col.owner")}</TableHead>
              <TableHead>{t("effect.col.other")}</TableHead>
              <TableHead>{t("effect.col.stole")}</TableHead>
              <TableHead>{t("effect.col.cofired")}</TableHead>
              <TableHead>{t("effect.col.neither")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {cells.map((cell) => (
              <TableRow key={`${cell.owner}-${cell.other}-row`} className="odd:bg-muted/30">
                <TableCell>{cell.owner}</TableCell>
                <TableCell>{cell.other}</TableCell>
                <TableCell className={cn("font-mono tabular-nums", heatTone(cell.stole))}>{cell.stole}</TableCell>
                <TableCell className={cn("font-mono tabular-nums", heatTone(cell.co_fired))}>{cell.co_fired}</TableCell>
                <TableCell className={cn("font-mono tabular-nums", heatTone(cell.neither))}>{cell.neither}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}

export function BisectList({ report }: { report: EffectivenessReport }) {
  const skills = report.interference?.bisect ?? [];
  return (
    <div data-testid="effect-bisect" className="flex flex-wrap gap-2">
      {skills.length === 0 ? <span className="text-sm text-muted-foreground">—</span> : null}
      {skills.map((skill) => (
        <span key={skill} className="rounded-full border border-destructive/40 bg-destructive/15 px-3 py-1 font-mono text-sm text-destructive">
          {skill}
        </span>
      ))}
    </div>
  );
}

export function WarningTable({ report }: { report: EffectivenessReport }) {
  const rows = report.interference?.warnings ?? [];
  return (
    <div className="overflow-hidden rounded-md border border-border">
      <Table>
        <TableCaption className="sr-only">{t("effect.warnings")}</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>{t("effect.col.query")}</TableHead>
            <TableHead>{t("effect.col.before")}</TableHead>
            <TableHead>{t("effect.col.after")}</TableHead>
            <TableHead>{t("effect.col.suspect")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row) => (
            <TableRow key={`${row.query_id}-${row.suspect}`} className="odd:bg-muted/30">
              <TableCell className="font-mono">{row.query_id}</TableCell>
              <TableCell>{row.before}</TableCell>
              <TableCell>{row.after}</TableCell>
              <TableCell className="font-medium">{row.suspect}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
