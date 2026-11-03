// Layout pieces for settings screens: a titled section of rows.

import * as React from "react";
import { cn } from "@/lib/utils";

export function SettingsSection({ title, description, action, children, className }: { title: React.ReactNode; description?: React.ReactNode; action?: React.ReactNode; children: React.ReactNode; className?: string }) {
  return (
    <section className={cn("surface overflow-hidden", className)}>
      <div className="flex items-start justify-between gap-4 border-b px-4 py-3">
        <div>
          <h2 className="text-sm font-semibold tracking-[-0.01em]">{title}</h2>
          {description ? <p className="mt-0.5 text-xs text-muted">{description}</p> : null}
        </div>
        {action ? <div className="flex shrink-0 items-center gap-2">{action}</div> : null}
      </div>
      <div className="divide-y">{children}</div>
    </section>
  );
}

export function SettingsRow({ label, description, children, align = "center", className }: { label: React.ReactNode; description?: React.ReactNode; children?: React.ReactNode; align?: "center" | "start"; className?: string }) {
  return (
    <div className={cn("flex justify-between gap-6 px-4 py-3", align === "center" ? "items-center" : "items-start", className)}>
      <div className="min-w-0 max-w-md">
        <div className="text-[13px] font-medium">{label}</div>
        {description ? <div className="mt-0.5 text-xs text-muted">{description}</div> : null}
      </div>
      {children ? <div className="flex shrink-0 items-center gap-2">{children}</div> : null}
    </div>
  );
}

/** Segmented control over a small set of string options. */
export function Segmented<V extends string>({ value, onChange, options, ariaLabel, className }: { value: V; onChange: (v: V) => void; options: { value: V; label: React.ReactNode; title?: string }[]; ariaLabel: string; className?: string }) {
  return (
    <div className={cn("flex items-center gap-0.5 rounded-lg bg-sunken p-0.5", className)} role="radiogroup" aria-label={ariaLabel}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={value === o.value}
          title={o.title}
          onClick={() => onChange(o.value)}
          className={cn("inline-flex h-7 items-center gap-1.5 rounded-md px-2.5 text-xs font-medium transition-colors [&_svg]:size-3.5", value === o.value ? "bg-elevated text-fg shadow-sm" : "text-muted hover:text-fg")}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
