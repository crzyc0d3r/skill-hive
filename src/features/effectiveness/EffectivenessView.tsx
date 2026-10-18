import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import { t } from "../../lib/i18n";
import { demoEnabled } from "@/lib/demo";
import { silent } from "@/lib/ipc/actions";
import { SAMPLE_REPORT, radarPoint, type EffectivenessReport, type InfluenceEdge } from "@/features/effectiveness/model";

const VERDICTS = ["hijacked", "cofired", "none", "strong", "value", "install", "healthy"] as const;

function verdictKey(code: string): (typeof VERDICTS)[number] {
  return VERDICTS.find((item) => item === code) ?? "healthy";
}

function verdictText(code: string): string {
  if (code === "hijacked") return t("effect.verdict.hijacked");
  if (code === "cofired") return t("effect.verdict.cofired");
  if (code === "none") return t("effect.verdict.none");
  if (code === "strong") return t("effect.verdict.strong");
  if (code === "value") return t("effect.verdict.value");
  if (code === "install") return t("effect.verdict.install");
  return t("effect.verdict.healthy");
}

function actionText(code: string): string {
  if (code === "hijacked") return t("effect.action.hijacked");
  if (code === "cofired") return t("effect.action.cofired");
  if (code === "none") return t("effect.action.none");
  if (code === "strong") return t("effect.action.strong");
  if (code === "value") return t("effect.action.value");
  if (code === "install") return t("effect.action.install");
  return t("effect.action.healthy");
}

export function EffectivenessView() {
  const demo = demoEnabled();
  const [report, setReport] = useState<EffectivenessReport | null>(() => (demo ? SAMPLE_REPORT : null));
  const [open, setOpen] = useState(false);
  const [edge, setEdge] = useState<InfluenceEdge | null>(null);
  const [node, setNode] = useState(0);

  useEffect(() => {
    if (demo) return;
    void silent<string>("get_effectiveness_report").then((raw) => {
      if (!raw) return;
      try {
        const parsed = JSON.parse(raw) as EffectivenessReport;
        if (parsed.grade) setReport(parsed);
      } catch {
        /* a report that is not JSON stays on the empty state */
      }
    });
  }, [demo]);

  if (!report) {
    return (
      <section data-testid="effectiveness" className="space-y-3">
        <h1 className="font-mono text-xl font-semibold">{t("effect.title")}</h1>
        <p>{t("effect.empty")}</p>
      </section>
    );
  }

  const code = verdictKey(report.verdict);
  const nodes = report.influence?.nodes ?? [];
  const edges = report.influence?.edges ?? [];
  const selected = nodes[node] ?? nodes[0];
  const axes = report.axes ?? [];
  const points = axes
    .map((axis, index) => (axis.value == null ? null : radarPoint(index, axes.length, axis.value)))
    .filter((point): point is { x: number; y: number } => point != null);
  const polygon = points.map((point) => `${point.x},${point.y}`).join(" ");

  return (
    <section data-testid="effectiveness" className="min-w-0 space-y-4">
      <h1 className="font-mono text-xl font-semibold">{t("effect.title")}</h1>
      <p className="text-4xl font-semibold" data-testid="effect-grade">
        <span className="sr-only">{t("effect.grade")} </span>
        {report.grade}
        <span className="ml-2 text-base font-normal tabular-nums">{Math.round(report.grade_score)}</span>
      </p>
      <p data-testid="effect-verdict">{verdictText(code)}</p>
      <p data-testid="effect-action">{actionText(code)}</p>
      {report.omitted_axes?.length ? <p>{t("effect.omitted", { axes: report.omitted_axes.join(", ") })}</p> : null}
      <Button type="button" size="sm" variant="outline" onClick={() => setOpen((value) => !value)}>
        {open ? t("effect.hide") : t("effect.show")}
      </Button>
      {open ? (
        <div className="space-y-6">
          <section>
            <h2 className="text-sm font-semibold">{t("effect.radar")}</h2>
            <p>{t("effect.caption.radar", { axis: report.weakest_axis })}</p>
            <svg role="img" aria-label={t("effect.caption.radar", { axis: report.weakest_axis })} viewBox="0 0 200 200" className="h-48 w-48">
              <polygon points={polygon} fill="none" stroke="currentColor" strokeWidth="2" />
              {points.map((point, index) => (
                <circle key={axes[index]?.name ?? index} cx={point.x} cy={point.y} r="3" fill="currentColor" />
              ))}
              {points.map((point, index) => (
                <text key={`label-${axes[index]?.name ?? index}`} x={point.x} y={point.y - 6} fontSize="6">
                  {axes[index]?.value}
                </text>
              ))}
            </svg>
            <Button type="button" size="sm" variant="outline" onClick={() => document.getElementById("effect-radar-table")?.focus()}>
              {t("effect.showTable")}
            </Button>
            <table id="effect-radar-table" tabIndex={0} className="w-full text-sm">
              <caption className="sr-only">{t("effect.radar")}</caption>
              <tbody>
                {axes.map((axis) => (
                  <tr key={axis.name}>
                    <th scope="row" className="text-left font-normal">{axis.name}</th>
                    <td className="tabular-nums">{axis.value == null ? "—" : axis.value}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
          <section>
            <h2 className="text-sm font-semibold">{t("effect.map")}</h2>
            <p>{t("effect.caption.map", { arrow: report.thickest_arrow })}</p>
            <div
              role="application"
              tabIndex={0}
              data-testid="effect-map"
              className="flex gap-4"
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
              {nodes.map((item, index) => (
                <button
                  key={item.id}
                  type="button"
                  data-selected={index === node}
                  className="rounded border border-border px-3 py-2 text-left"
                  style={{ fontSize: `${12 + item.won}px` }}
                  onClick={() => setNode(index)}
                >
                  <span className="font-semibold">{item.grade}</span> {item.id}
                  <span className="block tabular-nums">{Math.round(item.score)}</span>
                </button>
              ))}
            </div>
            <ul>
              {edges.map((item) => (
                <li key={`${item.source}-${item.target}`}>
                  <Button type="button" variant="outline" size="sm" onClick={() => setEdge(item)}>
                    {item.source} → {item.target} {Math.round(item.steal_rate * 100)}%
                  </Button>
                </li>
              ))}
            </ul>
            {edge ? (
              <div data-testid="effect-stolen">
                <h3 className="text-sm font-semibold">{t("effect.queries")}</h3>
                <ul>
                  {edge.queries.map((id) => {
                    const activation = report.activations?.find((row) => row.query_id === id);
                    return (
                      <li key={id}>
                        {activation?.query ?? id}
                        {activation?.transcript ? <span className="ml-2 font-mono text-xs">{activation.transcript}</span> : null}
                      </li>
                    );
                  })}
                </ul>
              </div>
            ) : null}
            <Button type="button" size="sm" variant="outline" onClick={() => document.getElementById("effect-map-table")?.focus()}>
              {t("effect.showTable")}
            </Button>
            <table id="effect-map-table" tabIndex={0} className="w-full text-sm">
              <caption className="sr-only">{t("effect.map")}</caption>
              <thead>
                <tr>
                  <th>source</th>
                  <th>target</th>
                  <th>steal rate</th>
                  <th>queries won</th>
                  <th>grade</th>
                  <th>transcript</th>
                </tr>
              </thead>
              <tbody>
                {edges.map((item) => {
                  const target = nodes.find((entry) => entry.id === item.target);
                  const transcript = report.activations?.find((row) => item.queries.includes(row.query_id))?.transcript ?? "";
                  return (
                    <tr key={`${item.source}-${item.target}`}>
                      <td>{item.source}</td>
                      <td>{item.target}</td>
                      <td className="tabular-nums">{Math.round(item.steal_rate * 100)}%</td>
                      <td className="tabular-nums">{target?.won ?? 0}</td>
                      <td>{target?.grade}</td>
                      <td className="font-mono text-xs">{transcript}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </section>
          <section>
            <h2 className="text-sm font-semibold">{t("effect.board")}</h2>
            <p>{t("effect.caption.board", { skill: report.leader_name, factor: report.leader_factor })}</p>
            <table className="w-full text-sm">
              <caption className="sr-only">{t("effect.board")}</caption>
              <thead>
                <tr>
                  <th>rank</th>
                  <th>skill</th>
                  <th>fires</th>
                  <th>quality</th>
                  <th>score</th>
                </tr>
              </thead>
              <tbody>
                {(report.leaderboard ?? []).map((row) => (
                  <tr key={row.skill} data-testid={`board-${row.skill}`}>
                    <td className="tabular-nums">{row.rank}</td>
                    <td>{row.skill}</td>
                    <td className="tabular-nums">{row.fires}</td>
                    <td className="tabular-nums">{row.quality}</td>
                    <td className="tabular-nums">{row.score}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
          <section>
            <h2 className="text-sm font-semibold">{t("effect.conflicts")}</h2>
            <p>
              {t("effect.caption.conflicts", {
                other: report.conflicts?.[0]?.other ?? "—",
                owner: report.conflicts?.[0]?.owner ?? "—",
              })}
            </p>
            <table className="w-full text-sm">
              <caption className="sr-only">{t("effect.conflicts")}</caption>
              <thead>
                <tr>
                  <th>owner</th>
                  <th>other</th>
                  <th>stole</th>
                  <th>co-fired</th>
                  <th>neither</th>
                </tr>
              </thead>
              <tbody>
                {(report.conflicts ?? []).map((cell) => (
                  <tr key={`${cell.owner}-${cell.other}`}>
                    <td>{cell.owner}</td>
                    <td>{cell.other}</td>
                    <td className="tabular-nums">{cell.stole}</td>
                    <td className="tabular-nums">{cell.co_fired}</td>
                    <td className="tabular-nums">{cell.neither}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
          <section>
            <h2 className="text-sm font-semibold">{t("effect.bisect")}</h2>
            <p>{t("effect.caption.bisect", { skills: report.interference?.bisect.join(", ") || "—" })}</p>
            <p data-testid="effect-bisect">{report.interference?.bisect.join(", ") || "—"}</p>
          </section>
          <section>
            <h2 className="text-sm font-semibold">{t("effect.warnings")}</h2>
            <p>
              {t("effect.caption.warnings", {
                skill: report.interference?.warnings[0]?.suspect ?? "—",
              })}
            </p>
            <table className="w-full text-sm">
              <caption className="sr-only">{t("effect.warnings")}</caption>
              <thead>
                <tr>
                  <th>query</th>
                  <th>before</th>
                  <th>after</th>
                  <th>suspect</th>
                </tr>
              </thead>
              <tbody>
                {(report.interference?.warnings ?? []).map((row) => (
                  <tr key={`${row.query_id}-${row.suspect}`}>
                    <td>{row.query_id}</td>
                    <td>{row.before}</td>
                    <td>{row.after}</td>
                    <td>{row.suspect}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        </div>
      ) : null}
    </section>
  );
}
