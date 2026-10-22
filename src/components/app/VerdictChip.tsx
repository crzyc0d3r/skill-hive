import { cn } from "cn";
import { t } from "@/lib/i18n";
import { verdictFromDelta, type VerdictKind } from "@/lib/verdict";

function verdictLabel(kind: VerdictKind): string {
  if (kind === "helps-lot") return t("verdict.helpsLot");
  if (kind === "helps") return t("verdict.helps");
  if (kind === "bit-worse") return t("verdict.bitWorse");
  if (kind === "much-worse") return t("verdict.muchWorse");
  return t("verdict.neutral");
}

const ICON: Record<VerdictKind, string> = {
  "helps-lot": "▲▲",
  helps: "▲",
  neutral: "●",
  "bit-worse": "▼",
  "much-worse": "▼▼",
};

const TOKEN: Record<VerdictKind, string> = {
  "helps-lot": "var(--verdict-helps-lot)",
  helps: "var(--verdict-helps)",
  neutral: "var(--verdict-neutral)",
  "bit-worse": "var(--verdict-worse)",
  "much-worse": "var(--verdict-much-worse)",
};

function ink(kind: VerdictKind): string {
  return kind === "helps-lot" || kind === "much-worse" ? "oklch(0.99 0 0)" : "oklch(0.2 0.03 265)";
}

export function VerdictChip({ kind, delta, large }: { kind?: VerdictKind; delta?: number | null; large?: boolean }) {
  const resolved = kind ?? verdictFromDelta(delta);
  return (
    <span
      data-testid="verdict-chip"
      data-verdict={resolved}
      className={cn(
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium",
        large && "px-3 py-1 text-sm font-semibold",
      )}
      style={{ backgroundColor: TOKEN[resolved], color: ink(resolved) }}
    >
      <span aria-hidden="true">{ICON[resolved]}</span>
      {verdictLabel(resolved)}
    </span>
  );
}

export function DeltaBar({ delta }: { delta: number | null }) {
  const value = delta ?? 0;
  const width = Math.min(50, Math.abs(value) / 2);
  const positive = value >= 0;
  const label = delta === null ? "—" : `${value > 0 ? "+" : ""}${Math.round(value)}`;
  return (
    <div className="flex min-w-0 items-center gap-2" data-testid="delta-bar">
      <div className="relative h-2 min-w-0 flex-1 rounded-full bg-muted">
        <span className="absolute top-0 bottom-0 left-1/2 w-px bg-border" aria-hidden="true" />
        <span
          className="absolute top-0 bottom-0 rounded-full"
          style={{
            width: `${width}%`,
            left: positive ? "50%" : `${50 - width}%`,
            backgroundColor: positive ? "var(--verdict-helps-lot)" : "var(--verdict-much-worse)",
          }}
        />
      </div>
      <span className="shrink-0 text-xs text-muted-foreground tabular-nums">{t("verdict.points", { points: label })}</span>
    </div>
  );
}
