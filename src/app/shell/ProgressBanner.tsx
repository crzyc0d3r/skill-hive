import * as React from "react";
import { Loader2 } from "lucide-react";
import { onHiveEvent } from "@/lib/api/client";
import { Progress } from "@/components/ui";

interface Op {
  op: string;
  message: string;
  done: number | null;
  total: number | null;
  at: number;
}

/** Thin bar under the title area that reflects long operations (scans, fetches, runs). */
export function ProgressBanner() {
  const [ops, setOps] = React.useState<Record<string, Op>>({});

  React.useEffect(() => {
    const dispose = onHiveEvent((e) => {
      if (e.type === "progress") {
        setOps((prev) => ({ ...prev, [e.op]: { op: e.op, message: e.message, done: e.done ?? null, total: e.total ?? null, at: Date.now() } }));
      }
      if (e.type === "run_updated") {
        if (e.status === "running" || e.status === "grading" || e.status === "queued") {
          setOps((prev) => ({
            ...prev,
            [`run:${e.run_id}`]: {
              op: `run:${e.run_id}`,
              message: e.progress.message ?? `Evaluation ${e.status}`,
              done: e.progress.done,
              total: e.progress.total,
              at: Date.now(),
            },
          }));
        } else {
          setOps((prev) => {
            const next = { ...prev };
            delete next[`run:${e.run_id}`];
            return next;
          });
        }
      }
    });
    const sweep = window.setInterval(() => {
      setOps((prev) => {
        const cutoff = Date.now() - 4000;
        const next: Record<string, Op> = {};
        for (const [k, v] of Object.entries(prev)) if (v.at > cutoff || k.startsWith("run:")) next[k] = v;
        return Object.keys(next).length === Object.keys(prev).length ? prev : next;
      });
    }, 1000);
    return () => {
      dispose();
      window.clearInterval(sweep);
    };
  }, []);

  const list = Object.values(ops);
  if (list.length === 0) return null;
  const op = list[list.length - 1];
  const frac = op.total ? (op.done ?? 0) / op.total : null;
  return (
    <div className="flex shrink-0 items-center gap-3 border-b bg-sunken/70 px-4 py-1.5 text-xs text-muted animate-fade-in">
      <Loader2 className="size-3.5 animate-spin text-accent" />
      <span className="truncate">{op.message}</span>
      {frac !== null ? (
        <>
          <span className="num ml-auto">
            {op.done}/{op.total}
          </span>
          <Progress value={frac} size="sm" className="w-32" />
        </>
      ) : null}
    </div>
  );
}
