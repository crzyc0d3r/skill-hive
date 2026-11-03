import { useNavigate } from "react-router-dom";
import { ArrowRight, CheckCircle2, Info, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { useUi } from "@/lib/store";
import { cn, displayPath, pluralize } from "@/lib/utils";
import { Button } from "@/components/ui";
import { KindIcon, TagChip } from "@/components/app/common";

/** What an import did: items now in the library and what was skipped, with reasons. */
export function ImportResultSummary({ result, onDismiss, className }: { result: T.ImportResult; onDismiss?: () => void; className?: string }) {
  const navigate = useNavigate();
  const openItem = useUi((s) => s.openItem);
  const nImported = result.imported.length;
  const nSkipped = result.skipped.length;
  return (
    <div className={cn("surface overflow-hidden animate-slide-up", className)} role="status">
      <div className="flex items-center gap-2 border-b bg-sunken/60 px-3 py-1.5 text-xs">
        {nImported ? <CheckCircle2 className="size-3.5 text-ok" /> : <Info className="size-3.5 text-muted" />}
        <span className="font-medium">{nImported ? `${pluralize(nImported, "item")} imported` : "Nothing imported"}</span>
        {nSkipped ? <span className="text-muted">· {pluralize(nSkipped, "item")} skipped</span> : null}
        <span className="ml-auto flex items-center gap-1">
          {nImported ? (
            <Button size="sm" variant="ghost" onClick={() => navigate("/library")}>
              Open library <ArrowRight />
            </Button>
          ) : null}
          {onDismiss ? (
            <Button size="icon-sm" variant="ghost" onClick={onDismiss} aria-label="Dismiss import result">
              <X />
            </Button>
          ) : null}
        </span>
      </div>
      {nImported ? (
        <ul className="divide-y">
          {result.imported.map((it) => (
            <li key={it.id}>
              <button type="button" onClick={() => openItem(it.id)} className="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-xs hover:bg-sunken/60">
                <KindIcon kind={it.kind} className="size-3.5 shrink-0 text-muted" />
                <span className="truncate text-[13px] font-medium">{it.name}</span>
                <span className="min-w-0 flex-1 truncate text-muted">{it.description}</span>
                <span className="flex shrink-0 gap-1">
                  {it.tags.slice(0, 3).map((t) => (
                    <TagChip key={t} tag={t} />
                  ))}
                </span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
      {nSkipped ? (
        <ul className={cn("divide-y", nImported && "border-t")}>
          {result.skipped.map((s) => (
            <li key={s.path} className="flex items-start gap-2.5 px-3 py-1.5 text-xs">
              <span className="mono shrink-0 text-subtle">{displayPath(s.path).split("/").pop()}</span>
              <span className="text-muted">{s.reason}</span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
