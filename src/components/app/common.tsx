// App-level building blocks shared by every screen.

import * as React from "react";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  Cloud,
  FolderGit2,
  GitFork,
  HardDrive,
  Minus,
  PenLine,
  Puzzle,
  Sparkles,
  TerminalSquare,
  TrendingDown,
  TrendingUp,
  XCircle,
  type LucideIcon,
} from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { useUi } from "@/lib/store";
import { cn, formatDelta, formatPercent, titleCase } from "@/lib/utils";
import { Badge, Button, Progress, Tooltip } from "@/components/ui";

export const KIND_ICON: Record<T.AssetKind, LucideIcon> = { skill: Puzzle, agent: Bot, command: TerminalSquare };
export const KIND_LABEL: Record<T.AssetKind, string> = { skill: "Skill", agent: "Agent", command: "Command" };

export function KindIcon({ kind, className }: { kind: T.AssetKind; className?: string }) {
  const Icon = KIND_ICON[kind];
  return <Icon className={cn("size-4", className)} />;
}

export function KindBadge({ kind }: { kind: T.AssetKind }) {
  return (
    <Badge tone="outline">
      <KindIcon kind={kind} className="size-3" />
      {KIND_LABEL[kind]}
    </Badge>
  );
}

const SOURCE_ICON: Record<T.SourceKind, LucideIcon> = {
  local_path: HardDrive,
  agent_dir: Bot,
  git: FolderGit2,
  github: GitFork,
  catalog: Sparkles,
  url: Cloud,
  authored: PenLine,
};
export const SOURCE_LABEL: Record<T.SourceKind, string> = {
  local_path: "Local",
  agent_dir: "Agent folder",
  git: "Git",
  github: "GitHub",
  catalog: "Catalog",
  url: "URL",
  authored: "Authored",
};

export function sourceDisplay(source: T.SourceRef): string {
  if (source.kind === "authored") return "Written in Skill Hive";
  let s = source.locator;
  if (source.subpath) s += `/${source.subpath}`;
  if (source.reference) s += `@${source.reference}`;
  return s;
}

export function SourceBadge({ source, withText = true }: { source: T.SourceRef; withText?: boolean }) {
  const Icon = SOURCE_ICON[source.kind];
  return (
    <Tooltip content={sourceDisplay(source)}>
      <span className="inline-flex items-center gap-1 text-[11px] text-muted">
        <Icon className="size-3" />
        {withText ? SOURCE_LABEL[source.kind] : null}
      </span>
    </Tooltip>
  );
}

export const VERDICT_META: Record<T.Verdict, { label: string; tone: "ok" | "warn" | "danger" | "neutral" | "info"; icon: LucideIcon }> = {
  helps_a_lot: { label: "Helps a lot", tone: "ok", icon: TrendingUp },
  helps: { label: "Helps", tone: "ok", icon: TrendingUp },
  neutral: { label: "Neutral", tone: "neutral", icon: Minus },
  hurts: { label: "Hurts", tone: "warn", icon: TrendingDown },
  hurts_a_lot: { label: "Hurts a lot", tone: "danger", icon: TrendingDown },
  inconclusive: { label: "Inconclusive", tone: "info", icon: AlertTriangle },
};

export function VerdictBadge({ verdict, delta }: { verdict: T.Verdict; delta?: number }) {
  const meta = VERDICT_META[verdict];
  const Icon = meta.icon;
  return (
    <Badge tone={meta.tone}>
      <Icon />
      {meta.label}
      {delta !== undefined ? <span className="num">{formatDelta(delta)}</span> : null}
    </Badge>
  );
}

export function EffectivenessBadge({ e }: { e: T.EffectivenessSummary | null | undefined }) {
  if (!e) return null;
  return (
    <Tooltip content={`With skill ${formatPercent(e.withSkillPassRate)} vs baseline ${formatPercent(e.baselinePassRate)}`}>
      <span>
        <VerdictBadge verdict={e.verdict} delta={e.delta} />
      </span>
    </Tooltip>
  );
}

export function ValidationBadge({ status }: { status: T.ValidationStatus }) {
  if (status === "ok") return null;
  if (status === "warnings")
    return (
      <Badge tone="warn">
        <AlertTriangle />
        Warnings
      </Badge>
    );
  if (status === "errors")
    return (
      <Badge tone="danger">
        <XCircle />
        Invalid
      </Badge>
    );
  return null;
}

export const INSTALL_STATE_META: Record<T.InstallState, { label: string; tone: "ok" | "warn" | "danger" | "neutral" | "info" }> = {
  linked: { label: "Linked", tone: "ok" },
  in_sync: { label: "In sync", tone: "ok" },
  stale: { label: "Stale copy", tone: "warn" },
  modified: { label: "Edited locally", tone: "warn" },
  unmanaged: { label: "Unmanaged", tone: "neutral" },
  broken: { label: "Broken link", tone: "danger" },
};

export function InstallStateBadge({ state }: { state: T.InstallState }) {
  const m = INSTALL_STATE_META[state];
  return <Badge tone={m.tone}>{m.label}</Badge>;
}

export const SEVERITY_TONE: Record<T.ConflictSeverity, "danger" | "warn" | "info"> = { high: "danger", medium: "warn", low: "info" };
export const CONFLICT_LABEL: Record<T.ConflictKind, string> = {
  name_collision: "Name collision",
  duplicate_content: "Duplicate content",
  description_overlap: "Overlapping descriptions",
  trigger_overlap: "Same trigger",
  tool_overlap: "Tool overlap",
  budget_exceeded: "Context budget",
  contradictory_directive: "Contradictory instructions",
};

/** Conflicts a model can resolve by combining the skills into one. */
const MERGEABLE_CONFLICTS: ReadonlySet<T.ConflictKind> = new Set(["description_overlap", "duplicate_content", "trigger_overlap", "tool_overlap", "contradictory_directive"]);

export function canMergeConflict(c: T.Conflict): boolean {
  return MERGEABLE_CONFLICTS.has(c.kind) && c.itemIds.length >= 2;
}

export function ConflictList({ report, compact, className }: { report: T.ConflictReport; compact?: boolean; className?: string }) {
  if (report.conflicts.length === 0) {
    return (
      <div className={cn("flex items-center gap-2 text-xs text-ok", className)}>
        <CheckCircle2 className="size-3.5" />
        No conflicts detected
      </div>
    );
  }
  return (
    <ul className={cn("flex flex-col gap-1.5", className)}>
      {report.conflicts.map((c, i) => (
        <li key={i} className={cn("rounded-lg px-3 py-2 text-xs", compact ? "bg-sunken" : "hairline bg-elevated")}>
          <div className="flex flex-wrap items-center gap-1.5">
            <Badge tone={SEVERITY_TONE[c.severity]}>{titleCase(c.severity)}</Badge>
            <span className="font-medium">{CONFLICT_LABEL[c.kind]}</span>
            {c.names.length ? <span className="text-muted">· {c.names.join(" ↔ ")}</span> : null}
            {canMergeConflict(c) ? (
              <Button size="sm" variant="subtle" className="ml-auto" aria-label={`Merge ${c.names.join(" and ")} with AI`} onClick={() => useUi.getState().openMerge(c.itemIds)}>
                <Sparkles /> Merge…
              </Button>
            ) : null}
          </div>
          <p className="mt-1 text-muted">{c.detail}</p>
          {c.sharedTerms?.length ? (
            <p className="mt-1 text-[11px] text-subtle">
              Shared terms: {(c.sharedTerms ?? []).map((t) => (
                <code key={t} className="mono mr-1 rounded bg-sunken px-1">
                  {t}
                </code>
              ))}
            </p>
          ) : null}
        </li>
      ))}
    </ul>
  );
}

export function BudgetMeter({
  chars,
  budgetChars,
  tokens,
  label = "Description budget",
  className,
}: {
  chars: number;
  budgetChars: number | null | undefined;
  tokens?: number;
  label?: string;
  className?: string;
}) {
  const budget = budgetChars ?? 15000;
  const frac = chars / budget;
  const tone = frac > 1 ? "danger" : frac > 0.8 ? "warn" : "ok";
  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-0.5 text-[11px]">
        <span className="shrink-0 whitespace-nowrap text-subtle font-medium uppercase tracking-wide">{label}</span>
        <span className={cn("num ml-auto text-right", tone === "danger" ? "text-danger" : tone === "warn" ? "text-warn" : "text-muted")}>
          {chars.toLocaleString()} / {budget.toLocaleString()} chars
          {tokens !== undefined ? ` · ~${tokens.toLocaleString()} tokens` : ""}
        </span>
      </div>
      <Progress value={Math.min(1, frac)} tone={tone} size="sm" />
    </div>
  );
}

export function PageHeader({
  title,
  description,
  actions,
  children,
  className,
}: {
  title: React.ReactNode;
  description?: React.ReactNode;
  actions?: React.ReactNode;
  children?: React.ReactNode;
  className?: string;
}) {
  return (
    <header className={cn("flex flex-col gap-3 px-6 pt-5 pb-3", className)}>
      <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
        <div className="min-w-64 flex-1">
          <h1 className="text-lg font-semibold tracking-[-0.015em]">{title}</h1>
          {description ? <p className="mt-0.5 text-xs text-muted text-balance">{description}</p> : null}
        </div>
        {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
      </div>
      {children}
    </header>
  );
}

export function Page({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("flex h-full min-h-0 flex-col animate-fade-in", className)}>{children}</div>;
}

export function PageBody({ children, className, scroll = true }: { children: React.ReactNode; className?: string; scroll?: boolean }) {
  return <div className={cn("min-h-0 flex-1 px-6 pb-6", scroll && "scroll-y", className)}>{children}</div>;
}

export function TagChip({ tag, onClick, active }: { tag: string; onClick?: () => void; active?: boolean }) {
  const cls = cn("rounded-full px-2 py-0.5 text-[11px] transition-colors", active ? "bg-accent text-accent-fg" : "bg-sunken text-muted", onClick && "hover:text-fg");
  return onClick ? (
    <button type="button" aria-pressed={active} onClick={onClick} className={cls}>
      {tag}
    </button>
  ) : (
    <span className={cls}>{tag}</span>
  );
}

export function InlineCode({ children, className, title }: { children: React.ReactNode; className?: string; title?: string }) {
  return (
    <code className={cn("mono rounded bg-sunken px-1 py-0.5 text-[11.5px] selectable", className)} title={title}>
      {children}
    </code>
  );
}

export function ErrorNote({ error, className }: { error: unknown; className?: string }) {
  if (!error) return null;
  const message = error instanceof Error ? error.message : String(error);
  return (
    <div className={cn("flex items-start gap-2 rounded-lg bg-danger-soft px-3 py-2 text-xs text-danger", className)}>
      <XCircle className="mt-0.5 size-3.5 shrink-0" />
      <span className="min-w-0 break-words selectable">{message}</span>
    </div>
  );
}
