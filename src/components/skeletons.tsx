import type { ReactNode } from "react";
import { cn } from "cn";
import { t } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { StatePanel } from "@/components/chrome";

export function Bone({ className }: { className?: string }) {
  return <span aria-hidden="true" className={cn("skeleton-shimmer block rounded-md bg-muted", className)} />;
}

export function LoadingRegion({ kind, label, children }: { kind: string; label?: string; children: ReactNode }) {
  return (
    <div data-testid="loading-state" data-skeleton={kind} aria-busy="true" aria-label={label} className="flex flex-col gap-3">
      {children}
    </div>
  );
}

export function RegionError({ onRetry }: { onRetry?: () => void }) {
  return (
    <StatePanel
      tone="error"
      title={t("shell.error")}
      action={
        <Button type="button" size="sm" onClick={onRetry}>
          {t("banner.retry")}
        </Button>
      }
    />
  );
}

function CardShell({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("rounded-lg border border-border bg-card px-3 py-3", className)}>{children}</div>;
}

export function SkillCardSkeleton() {
  return (
    <CardShell>
      <Bone className="h-4 w-2/3" />
      <Bone className="mt-2 h-3 w-full" />
      <Bone className="mt-1.5 h-3 w-5/6" />
      <div className="mt-3 flex gap-1.5">
        <Bone className="h-5 w-14 rounded-full" />
        <Bone className="h-5 w-16 rounded-full" />
        <Bone className="h-5 w-12 rounded-full" />
      </div>
    </CardShell>
  );
}

export function LibrarySkeleton() {
  return (
    <LoadingRegion kind="library">
      <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
        {Array.from({ length: 6 }, (_, index) => (
          <li key={index}>
            <SkillCardSkeleton />
          </li>
        ))}
      </ul>
    </LoadingRegion>
  );
}

export function DashboardSkeleton() {
  return (
    <LoadingRegion kind="dashboard">
      <div className="grid grid-cols-2 gap-3 xl:grid-cols-4">
        {Array.from({ length: 4 }, (_, index) => (
          <CardShell key={index} className="flex items-start justify-between">
            <div>
              <Bone className="h-3 w-16" />
              <Bone className="mt-2 h-8 w-14" />
            </div>
            <Bone className="size-8 rounded-md" />
          </CardShell>
        ))}
      </div>
      <div className="grid gap-3 lg:grid-cols-[minmax(0,1.45fr)_minmax(16rem,1fr)]">
        <CardShell>
          <Bone className="mb-2 h-4 w-32" />
          {Array.from({ length: 5 }, (_, index) => (
            <div key={index} className="flex items-center gap-3 border-t border-border py-2.5">
              <Bone className="size-8 shrink-0 rounded-md" />
              <div className="min-w-0 flex-1">
                <Bone className="h-3.5 w-40" />
                <Bone className="mt-1.5 h-3 w-56" />
              </div>
              <Bone className="hidden h-3 w-16 sm:block" />
            </div>
          ))}
        </CardShell>
        <div className="flex flex-col gap-3">
          <CardShell>
            <Bone className="h-3 w-24" />
            <Bone className="mt-3 h-2 w-full rounded-full" />
            <Bone className="mt-3 h-3 w-32" />
          </CardShell>
          <CardShell>
            <Bone className="h-4 w-28" />
            <Bone className="mt-3 h-8 w-24" />
          </CardShell>
          <CardShell>
            <Bone className="h-4 w-32" />
            <Bone className="mt-3 h-3 w-full" />
            <Bone className="mt-2 h-3 w-4/5" />
          </CardShell>
        </div>
      </div>
    </LoadingRegion>
  );
}

export function AgentListSkeleton() {
  return (
    <LoadingRegion kind="agents">
      <ul className="grid gap-2 sm:grid-cols-2">
        {Array.from({ length: 4 }, (_, index) => (
          <li key={index}>
            <CardShell className="flex items-center justify-between">
              <Bone className="h-4 w-32" />
              <Bone className="h-5 w-14 rounded-full" />
            </CardShell>
          </li>
        ))}
      </ul>
    </LoadingRegion>
  );
}

export function TableSkeleton({ columns = 3, rows = 5 }: { columns?: number; rows?: number }) {
  return (
    <div className="overflow-hidden rounded-md border border-border">
      <div className="grid gap-3 border-b border-border bg-muted/40 px-3 py-2" style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}>
        {Array.from({ length: columns }, (_, index) => (
          <Bone key={index} className="h-3 w-16" />
        ))}
      </div>
      {Array.from({ length: rows }, (_, row) => (
        <div key={row} className="grid gap-3 border-b border-border px-3 py-3 last:border-b-0" style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}>
          {Array.from({ length: columns }, (_, column) => (
            <Bone key={column} className={cn("h-3.5", column === 0 ? "w-24" : "w-16")} />
          ))}
        </div>
      ))}
    </div>
  );
}

export function ChartSkeleton({ className }: { className?: string }) {
  return (
    <div className={cn("rounded-lg border border-border bg-card p-4", className)}>
      <Bone className="mb-3 h-4 w-36" />
      <svg viewBox="0 0 320 168" className="h-44 w-full" aria-hidden="true">
        <polygon points="160,14 258,70 214,154 106,154 62,70" fill="var(--muted)" stroke="var(--border)" />
        <polyline points="18,140 72,102 128,112 188,62 302,84" fill="none" stroke="var(--chart-1)" strokeWidth="3" opacity="0.45" />
        <line x1="16" y1="150" x2="304" y2="150" stroke="var(--border)" />
      </svg>
    </div>
  );
}

export function InstallSkeleton() {
  return (
    <LoadingRegion kind="install">
      <CardShell className="max-w-xl">
        <Bone className="h-3 w-40" />
        <Bone className="mt-2 h-8 w-full" />
        <div className="mt-3 flex gap-2">
          <Bone className="h-8 w-28" />
          <Bone className="h-8 w-36" />
        </div>
      </CardShell>
    </LoadingRegion>
  );
}

export function SettingsSkeleton() {
  return (
    <LoadingRegion kind="settings">
      <Bone className="h-4 w-28" />
      <div className="flex gap-2">
        <Bone className="h-8 w-16" />
        <Bone className="h-8 w-16" />
        <Bone className="h-8 w-20" />
      </div>
      <div className="flex flex-wrap gap-2">
        <Bone className="h-8 w-36" />
        <Bone className="h-8 w-16" />
        <Bone className="h-8 w-20" />
        <Bone className="h-8 w-16" />
        <Bone className="h-8 w-12" />
      </div>
      <div className="flex items-center gap-2">
        <Bone className="h-3 w-16" />
        <Bone className="h-8 w-14" />
        <Bone className="h-8 w-16" />
        <Bone className="h-8 w-16" />
      </div>
    </LoadingRegion>
  );
}

export function EndpointSkeleton() {
  return (
    <LoadingRegion kind="endpoints">
      <div className="space-y-1">
        <Bone className="h-4 w-48" />
        <Bone className="h-4 w-44" />
        <Bone className="h-4 w-56" />
      </div>
      <ul className="space-y-2">
        {Array.from({ length: 3 }, (_, index) => (
          <li key={index} className="flex items-center justify-between rounded-lg border border-border px-3 py-2">
            <Bone className="h-4 w-40" />
            <Bone className="h-4 w-16" />
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap items-center gap-2">
        <Bone className="h-3 w-24" />
        <Bone className="h-8 w-48" />
        <Bone className="h-8 w-16" />
        <Bone className="h-8 w-16" />
        <Bone className="h-8 w-20" />
      </div>
      <div className="space-y-1">
        <Bone className="h-4 w-24" />
        <Bone className="h-3 w-64" />
        <Bone className="h-3 w-56" />
        <Bone className="h-3 w-52" />
      </div>
    </LoadingRegion>
  );
}

export function WorkspaceSkeleton() {
  return (
    <LoadingRegion kind="workspace">
      <div className="flex flex-wrap gap-2">
        <Bone className="h-8 w-28" />
        <Bone className="h-8 w-24" />
        <Bone className="h-8 w-32" />
      </div>
      <ul className="grid grid-cols-2 gap-3">
        {Array.from({ length: 4 }, (_, index) => (
          <li key={index}>
            <CardShell>
              <Bone className="h-4 w-32" />
              <Bone className="mt-2 h-3 w-48" />
            </CardShell>
          </li>
        ))}
      </ul>
    </LoadingRegion>
  );
}

export function ProjectSkeleton() {
  return (
    <LoadingRegion kind="project">
      {Array.from({ length: 3 }, (_, index) => (
        <CardShell key={index} className="max-w-xl">
          <Bone className="h-4 w-40" />
          <Bone className="mt-2 h-3 w-56" />
          <div className="mt-3 flex gap-2">
            <Bone className="h-8 w-20" />
            <Bone className="h-8 w-20" />
            <Bone className="h-8 w-20" />
          </div>
        </CardShell>
      ))}
    </LoadingRegion>
  );
}

export function BackupSkeleton() {
  return (
    <LoadingRegion kind="backup">
      {Array.from({ length: 4 }, (_, index) => (
        <div key={index} className="flex items-center justify-between rounded-lg border border-border bg-card px-3 py-3">
          <Bone className="h-4 w-48" />
          <Bone className="h-8 w-20" />
        </div>
      ))}
    </LoadingRegion>
  );
}

export function EffectivenessSkeleton() {
  return (
    <LoadingRegion kind="effectiveness">
      <div className="grid gap-3 lg:grid-cols-[minmax(18rem,0.85fr)_minmax(0,1.15fr)]">
        <CardShell className="min-h-56">
          <Bone className="mx-auto size-28 rounded-full" />
          <Bone className="mx-auto mt-4 h-4 w-40" />
          <Bone className="mx-auto mt-2 h-3 w-52" />
        </CardShell>
        <div className="grid grid-cols-2 gap-3">
          {Array.from({ length: 4 }, (_, index) => (
            <CardShell key={index}>
              <Bone className="h-3 w-20" />
              <Bone className="mt-2 h-7 w-16" />
            </CardShell>
          ))}
        </div>
      </div>
      <ChartSkeleton />
    </LoadingRegion>
  );
}

export function SuiteSkeleton() {
  return (
    <LoadingRegion kind="suite">
      <div className="grid gap-3 sm:grid-cols-2">
        {Array.from({ length: 2 }, (_, column) => (
          <CardShell key={column}>
            <Bone className="mb-2 h-3 w-24" />
            <div className="grid grid-cols-3 gap-2">
              {Array.from({ length: 3 }, (_, cell) => (
                <Bone key={cell} className="h-16" />
              ))}
            </div>
          </CardShell>
        ))}
      </div>
    </LoadingRegion>
  );
}

export function ResultsSkeleton() {
  return (
    <LoadingRegion kind="results">
      <div className="grid gap-3 lg:grid-cols-[minmax(0,1.1fr)_minmax(16rem,0.9fr)]">
        <div className="grid grid-cols-2 gap-3">
          {Array.from({ length: 4 }, (_, index) => (
            <Bone key={index} className="h-24" />
          ))}
        </div>
        <ChartSkeleton />
      </div>
      <div className="flex gap-2">
        {Array.from({ length: 5 }, (_, index) => (
          <Bone key={index} className="h-16 flex-1" />
        ))}
      </div>
    </LoadingRegion>
  );
}

export function CompareSkeleton() {
  return (
    <LoadingRegion kind="compare" label={t("compare.loading")}>
      <div className="grid gap-3 md:grid-cols-2">
        {Array.from({ length: 2 }, (_, index) => (
          <CardShell key={index} className="min-h-64">
            <div className="flex items-center gap-2">
              <Bone className="size-8 rounded-md" />
              <Bone className="h-4 w-32" />
            </div>
            <Bone className="mt-4 h-3 w-full" />
            <Bone className="mt-2 h-3 w-11/12" />
            <Bone className="mt-2 h-3 w-4/5" />
            <Bone className="mt-2 h-3 w-2/3" />
          </CardShell>
        ))}
      </div>
    </LoadingRegion>
  );
}

export function HistorySkeleton() {
  return (
    <LoadingRegion kind="history">
      <Bone className="h-24 w-full" />
      <TableSkeleton columns={5} rows={4} />
    </LoadingRegion>
  );
}
