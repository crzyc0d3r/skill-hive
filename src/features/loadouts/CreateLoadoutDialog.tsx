import * as React from "react";
import { toast } from "sonner";
import { Layers, Plus, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useItems } from "@/lib/api/hooks";
import { pluralize } from "@/lib/utils";
import { Button, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input, Textarea } from "@/components/ui";
import { ErrorNote, KindIcon } from "@/components/app/common";
import { ItemPickerDialog } from "@/features/agents/ItemPickerDialog";
import { ALL_ITEMS_QUERY } from "@/features/agents/queries";
import { ColorSwatches } from "./ColorSwatches";
import { LOADOUT_COLORS } from "./colors";

/** Create a loadout: name, description, colour and an optional initial set of items. */
export function CreateLoadoutDialog({
  open,
  onOpenChange,
  onCreated,
  initialItemIds = [],
  title = "New loadout",
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  onCreated?: (loadout: T.Loadout) => void;
  initialItemIds?: string[];
  title?: string;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>A named set of skills, agents and commands that you apply to an agent as one unit.</DialogDescription>
        </DialogHeader>
        <CreateLoadoutForm initialItemIds={initialItemIds} onCreated={onCreated} onCancel={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  );
}

function CreateLoadoutForm({ initialItemIds, onCreated, onCancel }: { initialItemIds: string[]; onCreated?: (l: T.Loadout) => void; onCancel: () => void }) {
  const [name, setName] = React.useState("");
  const [description, setDescription] = React.useState("");
  const [color, setColor] = React.useState<string | null>(LOADOUT_COLORS[0]);
  const [itemIds, setItemIds] = React.useState<string[]>(initialItemIds);
  const [pickerOpen, setPickerOpen] = React.useState(false);
  const { data: all } = useItems(ALL_ITEMS_QUERY);
  const items = itemIds.map((id) => all?.find((i) => i.id === id)).filter((i): i is T.LibraryItem => !!i);

  const create = useApiMutation((input: T.LoadoutInput) => api.createLoadout(input), [keys.loadouts, keys.dashboard, ["item"]], (l) => {
    toast.success(`Created loadout ${l.name}`);
    onCreated?.(l);
  });
  const valid = name.trim().length > 0;

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        if (valid) create.mutate({ name: name.trim(), description: description.trim(), itemIds, color });
      }}
      className="contents"
    >
      <DialogBody className="flex flex-col gap-4">
        <Field label="Name">
          <Input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Data analysis" />
        </Field>
        <Field label="Description">
          <Textarea value={description} onChange={(e) => setDescription(e.target.value)} rows={2} placeholder="What this set is for and when to apply it…" className="min-h-[56px] text-xs" />
        </Field>
        <Field label="Colour">
          <ColorSwatches value={color} onChange={setColor} />
        </Field>
        <div className="flex flex-col gap-1.5">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-muted">Items {itemIds.length ? `(${itemIds.length})` : ""}</span>
            <Button type="button" size="sm" variant="secondary" onClick={() => setPickerOpen(true)}>
              <Plus /> Add items…
            </Button>
          </div>
          {itemIds.length === 0 ? (
            <p className="rounded-lg border border-dashed px-3 py-3 text-xs text-muted">No items yet. You can add them now or later from the loadout page.</p>
          ) : (
            <ul className="flex flex-wrap gap-1.5">
              {itemIds.map((id) => {
                const it = items.find((i) => i.id === id);
                return (
                  <li key={id} className="inline-flex items-center gap-1 rounded-full bg-sunken py-0.5 pl-2 pr-1 text-[11px]">
                    {it ? <KindIcon kind={it.kind} className="size-3 text-muted" /> : null}
                    {it?.name ?? id}
                    <button type="button" aria-label={`Remove ${it?.name ?? id}`} className="rounded-full p-0.5 text-muted hover:text-fg" onClick={() => setItemIds(itemIds.filter((x) => x !== id))}>
                      <X className="size-3" />
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
        <ErrorNote error={create.error} />
      </DialogBody>
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" disabled={!valid} loading={create.isPending}>
          <Layers /> Create loadout{itemIds.length ? ` with ${pluralize(itemIds.length, "item")}` : ""}
        </Button>
      </DialogFooter>
      <ItemPickerDialog
        open={pickerOpen}
        onOpenChange={setPickerOpen}
        title="Add items to the loadout"
        excludeIds={itemIds}
        onConfirm={(ids) => {
          setItemIds([...itemIds, ...ids]);
          setPickerOpen(false);
        }}
      />
    </form>
  );
}
