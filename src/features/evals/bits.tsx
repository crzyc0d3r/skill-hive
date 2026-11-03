// Small presentational pieces shared by the eval screens.

import * as React from "react";
import { cn } from "@/lib/utils";

export function ArmSwatch({ color, className }: { color: string; className?: string }) {
  return <span aria-hidden className={cn("inline-block size-2.5 shrink-0 rounded-[3px]", className)} style={{ background: color }} />;
}

export function MiniStat({ label, value, hint, tone, className }: { label: React.ReactNode; value: React.ReactNode; hint?: React.ReactNode; tone?: "ok" | "warn" | "danger"; className?: string }) {
  const toneClass = tone === "ok" ? "text-ok" : tone === "warn" ? "text-warn" : tone === "danger" ? "text-danger" : "";
  return (
    <div className={cn("rounded-lg bg-sunken px-3 py-2", className)}>
      <div className="text-[11px] font-medium uppercase tracking-wide text-subtle">{label}</div>
      <div className={cn("mt-0.5 text-lg font-semibold tracking-tight", toneClass)}>{value}</div>
      {hint ? <div className="text-[11px] text-muted">{hint}</div> : null}
    </div>
  );
}
