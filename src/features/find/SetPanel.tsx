import * as React from "react";
import { Layers, Rocket, Sparkles, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { cn, pluralize } from "@/lib/utils";
import { Button, Skeleton } from "@/components/ui";
import { BudgetMeter, ConflictList, ErrorNote, KindIcon } from "@/components/app/common";
import { DeployDialog } from "@/features/agents/DeployDialog";
import { CreateLoadoutDialog } from "@/features/loadouts/CreateLoadoutDialog";

/** The chosen set: its members, budget and conflicts, with Deploy and Save-as-loadout. */
export function SetPanel({
  title = "Suggested set",
  items,
  chars,
  budgetChars,
  tokens,
  estimated,
  conflicts,
  conflictsLoading,
  conflictsError,
  onRemove,
  targetId,
  llmNotes,
  className,
}: {
  title?: string;
  items: T.LibraryItem[];
  chars: number;
  budgetChars: number | null | undefined;
  tokens: number;
  /** True when the budget is a local estimate rather than the backend's figure. */
  estimated?: boolean;
  conflicts?: T.ConflictReport;
  conflictsLoading?: boolean;
  conflictsError?: unknown;
  onRemove?: (id: string) => void;
  targetId: string;
  llmNotes?: string | null;
  className?: string;
}) {
  const [deployOpen, setDeployOpen] = React.useState(false);
  const [saveOpen, setSaveOpen] = React.useState(false);
  const ids = items.map((i) => i.id);
  return (
    <aside className={cn("surface flex flex-col gap-3 self-start p-3 xl:sticky xl:top-0", className)} aria-label={title}>
      <div className="flex items-center justify-between">
        <span className="text-[13px] font-semibold">{title}</span>
        <span className="num text-[11px] text-subtle">{pluralize(items.length, "item")}</span>
      </div>
      {items.length === 0 ? (
        <p className="rounded-lg border border-dashed px-3 py-3 text-xs text-muted">Nothing selected. Switch items on to build a set.</p>
      ) : (
        <ul className="flex flex-col gap-1">
          {items.map((it) => (
            <li key={it.id} className="flex items-center gap-2 rounded-md bg-sunken px-2 py-1 text-xs">
              <KindIcon kind={it.kind} className="size-3.5 shrink-0 text-muted" />
              <span className="min-w-0 flex-1 truncate font-medium">{it.name}</span>
              <span className="num shrink-0 text-[11px] text-subtle">{it.descriptionTokens} tok</span>
              {onRemove ? (
                <button type="button" aria-label={`Exclude ${it.name}`} className="rounded p-0.5 text-muted hover:text-fg" onClick={() => onRemove(it.id)}>
                  <X className="size-3" />
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      )}
      <BudgetMeter chars={chars} budgetChars={budgetChars} tokens={tokens} label={estimated ? "Description budget (estimate)" : "Description budget"} />
      <div>
        <div className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle">Conflicts within the set</div>
        {items.length === 0 ? (
          <p className="text-xs text-muted">Select at least one item.</p>
        ) : conflicts ? (
          <div className={cn(conflictsLoading && "opacity-60 transition-opacity")}>
            <ConflictList report={conflicts} compact />
          </div>
        ) : conflictsError ? (
          <ErrorNote error={conflictsError} />
        ) : (
          <Skeleton className="h-6" />
        )}
      </div>
      {llmNotes ? (
        <div className="flex items-start gap-2 rounded-lg bg-info-soft px-3 py-2 text-xs text-info">
          <Sparkles className="mt-0.5 size-3.5 shrink-0" />
          <span className="selectable">{llmNotes}</span>
        </div>
      ) : null}
      <div className="flex flex-col gap-1.5 border-t pt-3">
        <Button variant="primary" disabled={items.length === 0} onClick={() => setDeployOpen(true)}>
          <Rocket /> Deploy set…
        </Button>
        <Button variant="secondary" disabled={items.length === 0} onClick={() => setSaveOpen(true)}>
          <Layers /> Save as loadout…
        </Button>
      </div>
      <DeployDialog open={deployOpen} onOpenChange={setDeployOpen} itemIds={ids} defaultTarget={targetId} />
      <CreateLoadoutDialog open={saveOpen} onOpenChange={setSaveOpen} initialItemIds={ids} title="Save as loadout" onCreated={() => setSaveOpen(false)} />
    </aside>
  );
}
