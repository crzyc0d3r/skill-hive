import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { AlertTriangleIcon, InboxIcon, SearchIcon, WifiOffIcon } from "lucide-react";
import { cn } from "cn";
import { t } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

export function Screen({
  testId,
  title,
  lede,
  actions,
  children,
  className,
}: {
  testId?: string;
  title: string;
  lede?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section data-testid={testId} className={cn("flex w-full min-w-0 flex-col gap-4", className)}>
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div className="min-w-0">
          <h1 className="font-sans text-display font-semibold tracking-tight">{title}</h1>
          {lede ? <p className="mt-1 max-w-prose text-sm leading-5 text-muted-foreground">{lede}</p> : null}
        </div>
        {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
      </header>
      {children}
    </section>
  );
}

export function Stat({ label, value, hint, icon }: { label: string; value: ReactNode; hint?: ReactNode; icon?: ReactNode }) {
  return (
    <Card className="gap-1 rounded-lg border-border bg-card py-3 shadow-none">
      <div className="flex items-start justify-between gap-3 px-3">
        <div className="min-w-0">
          <div className="text-overline font-semibold text-muted-foreground uppercase">{label}</div>
          <div className="font-sans text-metric font-semibold tracking-tight tabular-nums">{value}</div>
          {hint ? <div className="text-xs text-muted-foreground">{hint}</div> : null}
        </div>
        {icon ? <div className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">{icon}</div> : null}
      </div>
    </Card>
  );
}

export function Panel({
  title,
  children,
  className,
  testId,
}: {
  title?: string;
  children: ReactNode;
  className?: string;
  testId?: string;
}) {
  return (
    <Card data-testid={testId} className={cn("gap-2 rounded-lg border-border bg-card py-3 shadow-none", className)}>
      <div className="px-3">
        {title ? <h2 className="font-sans text-title font-semibold">{title}</h2> : null}
        <div className={title ? "mt-2" : undefined}>{children}</div>
      </div>
    </Card>
  );
}

export function EmptyState({ title, body, action }: { title: string; body?: ReactNode; action?: ReactNode }) {
  return (
    <div data-testid="empty-state" className="flex flex-col items-center gap-3 rounded-lg border border-dashed border-border bg-muted/40 px-6 py-10 text-center">
      <InboxIcon className="size-10 text-muted-foreground" aria-hidden="true" />
      <h2 className="font-sans text-title font-semibold">{title}</h2>
      {body ? <p className="max-w-prose text-sm text-muted-foreground">{body}</p> : null}
      {action}
    </div>
  );
}

export function ErrorState({ title, detail, onRetry, retryLabel }: { title: string; detail?: string; onRetry?: () => void; retryLabel?: string }) {
  return (
    <div role="alert" data-testid="error-state" className="rounded-lg border border-destructive/40 bg-card px-4 py-4 text-card-foreground">
      <h2 className="font-sans text-base font-semibold">{title}</h2>
      <p className="mt-1 text-sm text-muted-foreground">{t("shell.renderError")}</p>
      {onRetry ? (
        <Button type="button" size="sm" className="mt-3" onClick={onRetry}>
          {retryLabel ?? t("banner.retry")}
        </Button>
      ) : null}
      <details className="mt-3">
        <summary className="cursor-pointer text-sm">{t("shell.details")}</summary>
        <pre className="mt-2 overflow-auto whitespace-pre-wrap font-mono text-xs">{detail || title}</pre>
      </details>
    </div>
  );
}

export function StatePanel({
  tone,
  title,
  action,
  testId,
}: {
  tone: "loading" | "empty" | "error" | "offline";
  title: string;
  action?: ReactNode;
  testId?: string;
}) {
  const icon =
    tone === "loading" ? null : tone === "error" ? (
      <AlertTriangleIcon className="size-5 text-destructive" aria-hidden="true" />
    ) : tone === "offline" ? (
      <WifiOffIcon className="size-5 text-warning-text" aria-hidden="true" />
    ) : (
      <InboxIcon className="size-5 text-muted-foreground" aria-hidden="true" />
    );
  const surface =
    tone === "error"
      ? "border-destructive/40 bg-destructive/10"
      : tone === "offline"
        ? "border-warning/40 bg-warning-subtle"
        : "border-dashed border-border bg-muted/40";
  return (
    <div
      role={tone === "error" ? "alert" : tone === "loading" ? "progressbar" : "status"}
      aria-busy={tone === "loading" ? true : undefined}
      data-testid={testId ?? (tone === "loading" ? "loading-state" : undefined)}
      className={cn("flex w-fit max-w-xl flex-col items-start gap-3 self-start rounded-lg border px-4 py-4", surface)}
    >
      {icon}
      <p className="max-w-prose text-sm text-foreground">{title || (tone === "offline" ? t("shell.offline") : t("shell.empty"))}</p>
      {action}
    </div>
  );
}

export function SearchField(props: React.ComponentProps<typeof Input>) {
  return (
    <div className="relative w-56 max-w-full">
      <SearchIcon className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" aria-hidden="true" />
      <Input {...props} className={cn("h-8 bg-background pl-8 text-[13px]", props.className)} />
    </div>
  );
}

export function TextField(props: React.ComponentProps<typeof Input>) {
  return <Input {...props} className={cn("h-8 bg-background text-[13px]", props.className)} />;
}

export function PrimaryLink({ to, children }: { to: string; children: ReactNode }) {
  return (
    <Button asChild size="sm">
      <Link to={to}>{children}</Link>
    </Button>
  );
}

export function QuietLink({ to, children }: { to: string; children: ReactNode }) {
  return (
    <Link to={to} className="text-sm font-medium underline underline-offset-2">
      {children}
    </Link>
  );
}
