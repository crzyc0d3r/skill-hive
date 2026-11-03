import * as React from "react";
import { Checkbox as CheckboxPrimitive, Progress as ProgressPrimitive, ScrollArea as ScrollAreaPrimitive, Separator as SeparatorPrimitive, Switch as SwitchPrimitive } from "radix-ui";
import { Check, Minus } from "lucide-react";
import { cn } from "@/lib/utils";

export const Switch = React.forwardRef<HTMLButtonElement, React.ComponentPropsWithoutRef<typeof SwitchPrimitive.Root>>(
  ({ className, ...props }, ref) => (
    <SwitchPrimitive.Root
      ref={ref}
      className={cn(
        "peer inline-flex h-5 w-9 shrink-0 cursor-default items-center rounded-full transition-colors",
        "data-[state=checked]:bg-accent data-[state=unchecked]:bg-line-strong disabled:opacity-50",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent-ring)]",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb className="block size-4 rounded-full bg-white shadow-sm transition-transform data-[state=checked]:translate-x-[18px] data-[state=unchecked]:translate-x-0.5" />
    </SwitchPrimitive.Root>
  ),
);
Switch.displayName = "Switch";

export const Checkbox = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof CheckboxPrimitive.Root>
>(({ className, ...props }, ref) => (
  <CheckboxPrimitive.Root
    ref={ref}
    className={cn(
      "peer size-4 shrink-0 rounded-[4px] hairline bg-elevated transition-colors",
      "data-[state=checked]:bg-accent data-[state=checked]:text-accent-fg data-[state=indeterminate]:bg-accent data-[state=indeterminate]:text-accent-fg",
      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent-ring)] disabled:opacity-50",
      className,
    )}
    {...props}
  >
    <CheckboxPrimitive.Indicator className="flex items-center justify-center">
      {props.checked === "indeterminate" ? <Minus className="size-3" /> : <Check className="size-3" strokeWidth={3} />}
    </CheckboxPrimitive.Indicator>
  </CheckboxPrimitive.Root>
));
Checkbox.displayName = "Checkbox";

export function Progress({
  value,
  className,
  tone = "accent",
  size = "md",
}: {
  value: number; // 0..1
  className?: string;
  tone?: "accent" | "ok" | "warn" | "danger" | "info";
  size?: "sm" | "md";
}) {
  const pct = Math.max(0, Math.min(100, value * 100));
  const bar = { accent: "bg-accent", ok: "bg-ok", warn: "bg-warn", danger: "bg-danger", info: "bg-info" }[tone];
  return (
    <ProgressPrimitive.Root
      value={pct}
      className={cn("relative w-full overflow-hidden rounded-full bg-sunken", size === "sm" ? "h-1.5" : "h-2", className)}
    >
      <ProgressPrimitive.Indicator
        className={cn("h-full rounded-full transition-[width] duration-300", bar)}
        style={{ width: `${pct}%` }}
      />
    </ProgressPrimitive.Root>
  );
}

export const ScrollArea = React.forwardRef<HTMLDivElement, React.ComponentPropsWithoutRef<typeof ScrollAreaPrimitive.Root>>(
  ({ className, children, ...props }, ref) => (
    <ScrollAreaPrimitive.Root ref={ref} className={cn("relative overflow-hidden", className)} {...props}>
      <ScrollAreaPrimitive.Viewport className="size-full rounded-[inherit]">{children}</ScrollAreaPrimitive.Viewport>
      <ScrollAreaPrimitive.Scrollbar orientation="vertical" className="flex w-2.5 touch-none select-none p-0.5">
        <ScrollAreaPrimitive.Thumb className="relative flex-1 rounded-full bg-line-strong" />
      </ScrollAreaPrimitive.Scrollbar>
      <ScrollAreaPrimitive.Corner />
    </ScrollAreaPrimitive.Root>
  ),
);
ScrollArea.displayName = "ScrollArea";

export function Separator({ className, orientation = "horizontal", ...props }: React.ComponentPropsWithoutRef<typeof SeparatorPrimitive.Root>) {
  return (
    <SeparatorPrimitive.Root
      orientation={orientation}
      decorative
      className={cn("shrink-0 bg-line", orientation === "horizontal" ? "h-px w-full" : "h-full w-px", className)}
      {...props}
    />
  );
}

export function Skeleton({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("animate-pulse-soft rounded-md bg-sunken", className)} {...props} />;
}

export function Kbd({ className, children }: { className?: string; children: React.ReactNode }) {
  return (
    <kbd
      className={cn(
        "inline-flex h-5 min-w-5 items-center justify-center rounded-[5px] bg-sunken px-1 font-sans text-[10.5px] font-medium text-muted hairline",
        className,
      )}
    >
      {children}
    </kbd>
  );
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
  compact,
}: {
  icon?: React.ComponentType<{ className?: string }>;
  title: React.ReactNode;
  description?: React.ReactNode;
  action?: React.ReactNode;
  className?: string;
  compact?: boolean;
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center rounded-xl border border-dashed text-center",
        compact ? "gap-2 px-4 py-8" : "gap-3 px-6 py-14",
        className,
      )}
    >
      {Icon ? (
        <div className="flex size-10 items-center justify-center rounded-xl bg-sunken text-muted">
          <Icon className="size-5" />
        </div>
      ) : null}
      <div>
        <div className="text-sm font-medium">{title}</div>
        {description ? <div className="mt-1 max-w-sm text-xs text-muted text-balance">{description}</div> : null}
      </div>
      {action ? <div className="mt-1 flex items-center gap-2">{action}</div> : null}
    </div>
  );
}

/** Inline status dot. */
export function Dot({ tone = "neutral", className }: { tone?: "neutral" | "ok" | "warn" | "danger" | "info" | "accent"; className?: string }) {
  const color = { neutral: "bg-subtle", ok: "bg-ok", warn: "bg-warn", danger: "bg-danger", info: "bg-info", accent: "bg-accent" }[tone];
  return <span className={cn("inline-block size-1.5 rounded-full", color, className)} />;
}
