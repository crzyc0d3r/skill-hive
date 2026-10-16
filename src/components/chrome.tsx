import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { InboxIcon, SearchIcon } from "lucide-react";
import { cn } from "cn";
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
          <h1 className="font-mono text-[22px] font-semibold leading-7 tracking-tight">{title}</h1>
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
          <div className="text-xs font-medium text-muted-foreground">{label}</div>
          <div className="font-mono text-[28px] font-semibold leading-9 tracking-tight tabular-nums">{value}</div>
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
        {title ? <h2 className="font-mono text-base font-medium leading-6">{title}</h2> : null}
        <div className={title ? "mt-2" : undefined}>{children}</div>
      </div>
    </Card>
  );
}

export function EmptyState({ title, action }: { title: string; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-start gap-3 rounded-lg border border-dashed border-border bg-muted/40 px-4 py-8">
      <InboxIcon className="size-5 text-muted-foreground" aria-hidden="true" />
      <p className="max-w-prose text-sm text-muted-foreground">{title}</p>
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
    <Button asChild size="sm" variant="outline">
      <Link to={to}>{children}</Link>
    </Button>
  );
}
