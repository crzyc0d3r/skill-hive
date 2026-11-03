import * as React from "react";
import { Search } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { useItems } from "@/lib/api/hooks";
import { cn, formatTokens, pluralize } from "@/lib/utils";
import { Button, Checkbox, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input, Skeleton } from "@/components/ui";
import { ErrorNote, KIND_LABEL, KindIcon, ValidationBadge } from "@/components/app/common";
import { ALL_ITEMS_QUERY } from "./queries";

/** Multi-select over the library, used to pick items to deploy or to add to a loadout. */
export function ItemPickerDialog({
  open,
  onOpenChange,
  title = "Add items",
  description = "Search the library and tick what to include.",
  confirmLabel = "Add",
  excludeIds = [],
  onConfirm,
  loading,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  title?: React.ReactNode;
  description?: React.ReactNode;
  confirmLabel?: string;
  /** Items already present wherever the picked items go; hidden from the list. */
  excludeIds?: string[];
  onConfirm: (ids: string[]) => void;
  loading?: boolean;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg" className="h-[min(640px,85vh)]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <PickerBody excludeIds={excludeIds} confirmLabel={confirmLabel} loading={loading} onConfirm={onConfirm} onCancel={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  );
}

function PickerBody({
  excludeIds,
  confirmLabel,
  loading,
  onConfirm,
  onCancel,
}: {
  excludeIds: string[];
  confirmLabel: string;
  loading?: boolean;
  onConfirm: (ids: string[]) => void;
  onCancel: () => void;
}) {
  const [text, setText] = React.useState("");
  const [debounced, setDebounced] = React.useState("");
  const [kinds, setKinds] = React.useState<T.AssetKind[]>([]);
  const [selected, setSelected] = React.useState<string[]>([]);
  React.useEffect(() => {
    const t = window.setTimeout(() => setDebounced(text.trim()), 150);
    return () => window.clearTimeout(t);
  }, [text]);
  const query = React.useMemo<T.LibraryQuery>(() => ({ ...ALL_ITEMS_QUERY, text: debounced || null, kinds, sort: debounced ? "relevance" : "name" }), [debounced, kinds]);
  const { data, isLoading, isError, error } = useItems(query);

  const excluded = React.useMemo(() => new Set(excludeIds), [excludeIds]);
  const visible = React.useMemo(() => (data ?? []).filter((i) => !excluded.has(i.id)), [data, excluded]);
  const hidden = (data?.length ?? 0) - visible.length;
  const toggle = (id: string) => setSelected((s) => (s.includes(id) ? s.filter((x) => x !== id) : [...s, id]));
  const toggleKind = (k: T.AssetKind) => setKinds((prev) => (prev.includes(k) ? prev.filter((x) => x !== k) : [...prev, k]));

  return (
    <>
      <div className="flex flex-wrap items-center gap-2 px-5 pb-2">
        <div className="relative min-w-[200px] flex-1">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
          <Input autoFocus value={text} onChange={(e) => setText(e.target.value)} placeholder="Search name, description, tags…" className="pl-8" aria-label="Search library" />
        </div>
        <div className="flex items-center gap-0.5 rounded-lg bg-sunken p-0.5">
          {(["skill", "agent", "command"] as T.AssetKind[]).map((k) => (
            <button
              key={k}
              type="button"
              onClick={() => toggleKind(k)}
              aria-pressed={kinds.includes(k)}
              className={cn("h-7 rounded-md px-2.5 text-xs font-medium transition-colors", kinds.includes(k) ? "bg-elevated text-fg shadow-sm" : "text-muted hover:text-fg")}
            >
              {KIND_LABEL[k]}s
            </button>
          ))}
        </div>
      </div>
      <DialogBody className="flex min-h-0 flex-col pb-0">
        {isLoading ? (
          <div className="flex flex-col gap-2">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-10" />
            ))}
          </div>
        ) : isError ? (
          <ErrorNote error={error} />
        ) : visible.length === 0 ? (
          <div className="rounded-lg border border-dashed px-3 py-8 text-center text-xs text-muted">
            {hidden ? `Everything matching is already included (${hidden} hidden).` : "No library items match."}
          </div>
        ) : (
          <div className="surface divide-y overflow-hidden">
            {visible.map((item) => {
              const sel = selected.includes(item.id);
              return (
                <div
                  key={item.id}
                  onClick={() => toggle(item.id)}
                  className={cn("flex items-center gap-3 px-3 py-2 text-xs transition-colors hover:bg-sunken/60", sel && "bg-accent-soft/35")}
                >
                  <span onClick={(e) => e.stopPropagation()} className="flex items-center">
                    <Checkbox checked={sel} onCheckedChange={() => toggle(item.id)} aria-label={`Select ${item.name}`} />
                  </span>
                  <KindIcon kind={item.kind} className="size-3.5 shrink-0 text-muted" />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <span className="truncate text-[13px] font-medium">{item.name}</span>
                      <ValidationBadge status={item.validation} />
                    </div>
                    <p className="line-clamp-1 text-muted">{item.description}</p>
                  </div>
                  <span className="num shrink-0 text-[11px] text-subtle">
                    {item.descriptionTokens} desc · {formatTokens(item.tokenEstimate)} tok
                  </span>
                </div>
              );
            })}
          </div>
        )}
        {hidden > 0 && visible.length > 0 ? <p className="py-2 text-[11px] text-subtle">{pluralize(hidden, "item")} already included and hidden.</p> : null}
      </DialogBody>
      <DialogFooter>
        <span className="mr-auto text-xs text-muted">{pluralize(selected.length, "item")} selected</span>
        <Button variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button variant="primary" disabled={selected.length === 0} loading={loading} onClick={() => onConfirm(selected)}>
          {confirmLabel}
          {selected.length ? ` (${selected.length})` : ""}
        </Button>
      </DialogFooter>
    </>
  );
}
