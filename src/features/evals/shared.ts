// Labels, tones and formatting shared by the eval screens.

import type * as T from "@/lib/api/bindings";
import { formatPercent, titleCase } from "@/lib/utils";

export type Tone = "ok" | "warn" | "danger" | "info" | "neutral";

export const RUN_STATUS_META: Record<T.RunStatus, { label: string; tone: Tone }> = {
  queued: { label: "Queued", tone: "neutral" },
  running: { label: "Running", tone: "info" },
  grading: { label: "Grading", tone: "info" },
  done: { label: "Done", tone: "ok" },
  failed: { label: "Failed", tone: "danger" },
  cancelled: { label: "Cancelled", tone: "warn" },
};

export const CASE_STATUS_META: Record<T.CaseStatus, { label: string; tone: Tone }> = {
  queued: { label: "Queued", tone: "neutral" },
  running: { label: "Running", tone: "info" },
  executed: { label: "Executed", tone: "info" },
  graded: { label: "Graded", tone: "ok" },
  failed: { label: "Failed", tone: "danger" },
  timeout: { label: "Timed out", tone: "warn" },
  cancelled: { label: "Cancelled", tone: "warn" },
};

export const PATTERN_META: Record<T.ExpectationPattern, { label: string; tone: Tone; explain: string }> = {
  only_with_skill: { label: "Only with skill", tone: "ok", explain: "Passes only when the skill is available: clear value from these instructions." },
  only_baseline: { label: "Only baseline", tone: "warn", explain: "Passes only without the skill: the instructions get in the way here." },
  always_passes: { label: "Always passes", tone: "neutral", explain: "Passes in every arm: it does not measure the skill. Tighten it or drop it." },
  always_fails: { label: "Always fails", tone: "danger", explain: "Fails in every arm: the task is too hard or the expectation is wrong." },
  flaky: { label: "Flaky", tone: "warn", explain: "Inconsistent across repeats of the same arm: the prompt or the assertion is unstable." },
  mixed: { label: "Mixed", tone: "info", explain: "Passes more in some arms than in others without a clean split." },
};

export const CHECK_KIND_LABEL: Record<T.CheckKind["kind"], string> = {
  file_exists: "File exists",
  file_contains: "File contains text",
  file_matches: "File matches regex",
  output_contains: "Output contains text",
  output_matches: "Output matches regex",
  json_pointer: "JSON value equals",
  command: "Command succeeds",
};

export const CHECK_KINDS = Object.keys(CHECK_KIND_LABEL) as T.CheckKind["kind"][];

export function isActiveRun(status: T.RunStatus): boolean {
  return status === "queued" || status === "running" || status === "grading";
}

/** Human label for an arm. */
export function armLabel(label: string): string {
  if (label === "with_skill") return "With skill";
  if (label === "without_skill") return "Without skill";
  return titleCase(label);
}

/** Series colour per arm: fixed by identity, never by position in a filtered list. */
export function armColor(label: string, index = 0): string {
  if (label === "with_skill") return "var(--chart-1)";
  if (label === "without_skill") return "var(--chart-2)";
  const extra = ["var(--chart-3)", "var(--chart-4)", "var(--chart-5)"];
  return extra[Math.max(0, index) % extra.length];
}

/** Colour for the n-th arm in a benchmark (variants get chart-3…5 in order). */
export function armColors(labels: string[]): Record<string, string> {
  const out: Record<string, string> = {};
  let variant = 0;
  for (const l of labels) {
    out[l] = l === "with_skill" || l === "without_skill" ? armColor(l) : armColor(l, variant++);
  }
  return out;
}

export function formatPValue(p: number | null | undefined): string {
  if (p === null || p === undefined || !Number.isFinite(p)) return "p n/a";
  if (p < 0.001) return "p < 0.001";
  // Two significant figures below 0.1 (0.012, 0.0045), two decimals above (0.61).
  return `p = ${p < 0.1 ? parseFloat(p.toPrecision(2)) : p.toFixed(2)}`;
}

export function formatPts(fraction: number): string {
  const v = Math.round(fraction * 100);
  return `${v > 0 ? "+" : ""}${v}`;
}

export function formatCI(low: number, high: number): string {
  return `[${formatPts(low)}, ${formatPts(high)}] pts`;
}

/** Mean ± stddev, both as percentages. */
export function formatRateStats(s: T.Stats): string {
  return `${formatPercent(s.mean)} ± ${Math.round(s.stddev * 100)}`;
}

/** One-line reading of a benchmark for people who do not want to parse the numbers. */
export function interpretBenchmark(b: T.Benchmark): string {
  const w = b.arms.find((a) => a.label === "with_skill");
  const base = b.arms.find((a) => a.label === b.baseline);
  const delta = b.deltas.find((d) => d.arm === "with_skill" && d.baseline === b.baseline) ?? b.deltas[0];
  if (!w || !base) return "No baseline arm: nothing to compare against.";
  const wp = formatPercent(w.passRate.mean);
  const bp = formatPercent(base.passRate.mean);
  const significant = delta?.pValue !== null && delta?.pValue !== undefined && delta.pValue < 0.05;
  switch (b.verdict) {
    case "helps_a_lot":
    case "helps":
      return `The skill raised the pass rate from ${bp} to ${wp}${significant ? "; the difference is statistically significant." : "; more repeats would firm up the estimate."}`;
    case "neutral":
      return `Pass rates are similar with (${wp}) and without (${bp}) the skill: it makes little measurable difference on this suite.`;
    case "hurts":
    case "hurts_a_lot":
      return `The skill lowered the pass rate from ${bp} to ${wp}: these instructions get in the way on this suite.`;
    default:
      return `Too little data to tell: ${wp} with the skill vs ${bp} without, and the confidence interval spans zero. Add repeats or cases.`;
  }
}

/** Diverging ok ↔ danger fill for a pass fraction, with a near-transparent midpoint at 50 %. */
export function passFractionFill(fraction: number): string {
  const f = Math.max(0, Math.min(1, fraction));
  if (f >= 0.5) {
    const t = (f - 0.5) * 2;
    return `color-mix(in oklab, var(--ok) ${Math.round(10 + t * 50)}%, transparent)`;
  }
  const t = (0.5 - f) * 2;
  return `color-mix(in oklab, var(--danger) ${Math.round(10 + t * 50)}%, transparent)`;
}

/** Quote-aware split of a command line into arguments. */
export function splitArgs(text: string): string[] {
  const out: string[] = [];
  const re = /"([^"]*)"|'([^']*)'|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) out.push(m[1] ?? m[2] ?? m[3]);
  return out;
}

export function joinArgs(args: string[]): string {
  return args.map((a) => (/\s/.test(a) ? `"${a}"` : a)).join(" ");
}
