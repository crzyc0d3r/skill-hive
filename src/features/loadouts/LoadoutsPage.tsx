import * as React from "react";
import { Navigate, useNavigate, useParams, useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { Layers, Plus, Save, Trash2, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useAnalyze, useApiMutation, useItems, useLoadout, useLoadouts, useTargets } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { cn, formatTokens, pluralize, relativeTime } from "@/lib/utils";
import { Badge, Button, ConfirmDialog, EmptyState, Input, Skeleton, Textarea, Tooltip } from "@/components/ui";
import { BudgetMeter, ConflictList, ErrorNote, KindIcon, Page, PageHeader } from "@/components/app/common";
import { ItemPickerDialog } from "@/features/agents/ItemPickerDialog";
import { ALL_ITEMS_QUERY, estimateDescriptionChars } from "@/features/agents/queries";
import { ApplyLoadoutCard } from "./ApplyLoadoutCard";
import { ColorSwatches, LoadoutDot } from "./ColorSwatches";
import { CreateLoadoutDialog } from "./CreateLoadoutDialog";

/** Loadouts: curated sets, edited here and applied to an agent exclusively or additively. */
export function LoadoutsPage() {
  const { loadoutId } = useParams<{ loadoutId: string }>();
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const loadouts = useLoadouts();
  const [createOpen, setCreateOpen] = React.useState(false);
  const targetParam = params.get("target");
  const suffix = targetParam ? `?target=${encodeURIComponent(targetParam)}` : "";

  if (!loadoutId && loadouts.data?.length) return <Navigate to={`/loadouts/${loadouts.data[0].id}${suffix}`} replace />;

  return (
    <Page>
      <PageHeader
        title="Loadouts"
        description="Curated sets of skills applied to an agent as one unit: exclusive to replace what is installed, additive to layer on top."
        actions={
          <Button variant="primary" onClick={() => setCreateOpen(true)}>
            <Plus /> New loadout
          </Button>
        }
      />
      <div className="flex min-h-0 flex-1 border-t">
        <aside className="flex w-64 shrink-0 flex-col border-r" aria-label="Loadouts">
          <div className="scroll-y flex-1 p-2">
            {loadouts.isLoading ? (
              <div className="flex flex-col gap-1.5 px-1">
                {Array.from({ length: 3 }).map((_, i) => (
                  <Skeleton key={i} className="h-14" />
                ))}
              </div>
            ) : loadouts.isError ? (
              <ErrorNote error={loadouts.error} />
            ) : loadouts.data?.length === 0 ? (
              <p className="px-2.5 py-2 text-xs text-muted">No loadouts yet.</p>
            ) : (
              <div className="flex flex-col gap-0.5">
                {loadouts.data?.map((l) => {
                  const active = l.id === loadoutId;
                  return (
                    <button
                      key={l.id}
                      type="button"
                      onClick={() => navigate(`/loadouts/${l.id}${suffix}`)}
                      aria-current={active ? "page" : undefined}
                      className={cn("flex w-full items-start gap-2.5 rounded-lg px-2.5 py-2 text-left transition-colors hover:bg-sunken", active && "bg-elevated shadow-sm hairline hover:bg-elevated")}
                    >
                      <LoadoutDot color={l.color} className="mt-1.5" />
                      <span className="min-w-0 flex-1">
                        <span className="flex items-center gap-2">
                          <span className="truncate text-[13px] font-medium">{l.name}</span>
                          <span className="num ml-auto text-[11px] text-subtle">{l.itemIds.length}</span>
                        </span>
                        <span className="line-clamp-1 text-[11px] text-muted">{l.description || "No description"}</span>
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </aside>
        <div className="min-w-0 flex-1 scroll-y px-6 py-4">
          {loadoutId ? (
            <LoadoutDetail key={loadoutId} id={loadoutId} initialTarget={targetParam} />
          ) : loadouts.isLoading ? (
            <div className="flex flex-col gap-3">
              <Skeleton className="h-28" />
              <Skeleton className="h-48" />
            </div>
          ) : loadouts.isError ? null : (
            <EmptyState
              icon={Layers}
              title="No loadouts yet"
              description="Group the skills you use together, such as everything for document work or an engineering baseline, and apply the set to any agent in one step."
              action={
                <Button variant="primary" onClick={() => setCreateOpen(true)}>
                  <Plus /> New loadout
                </Button>
              }
            />
          )}
        </div>
      </div>
      <CreateLoadoutDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        onCreated={(l) => {
          setCreateOpen(false);
          navigate(`/loadouts/${l.id}${suffix}`);
        }}
      />
    </Page>
  );
}

function LoadoutDetail({ id, initialTarget }: { id: string; initialTarget: string | null }) {
  const navigate = useNavigate();
  const openItem = useUi((s) => s.openItem);
  const { data: loadout, isLoading, error, refetch } = useLoadout(id);
  const { data: all } = useItems(ALL_ITEMS_QUERY);
  const { data: targets } = useTargets();
  const [targetId, setTargetId] = React.useState(initialTarget ?? "claude-code");
  const [pickerOpen, setPickerOpen] = React.useState(false);
  const [confirmDelete, setConfirmDelete] = React.useState(false);

  const update = useApiMutation((input: T.LoadoutInput) => api.updateLoadout(id, input), [keys.loadouts, keys.dashboard, ["item"]], () => toast.success("Loadout saved"));
  const remove = useApiMutation(() => api.deleteLoadout(id), [keys.loadouts, keys.dashboard, ["item"]], () => {
    toast.success(`Deleted ${loadout?.name ?? "loadout"}`);
    navigate("/loadouts", { replace: true });
  });

  const itemIds = React.useMemo(() => loadout?.itemIds ?? [], [loadout]);
  const analysis = useAnalyze(itemIds, targetId);

  if (isLoading || !loadout) {
    return (
      <div className="flex flex-col gap-3" aria-busy={isLoading}>
        {error ? (
          <>
            <ErrorNote error={error} />
            <div>
              <Button size="sm" onClick={() => void refetch()}>
                Retry
              </Button>
            </div>
          </>
        ) : (
          <>
            <Skeleton className="h-28" />
            <Skeleton className="h-48" />
          </>
        )}
      </div>
    );
  }

  const items = itemIds.map((iid) => all?.find((i) => i.id === iid)).filter((i): i is T.LibraryItem => !!i);
  const missing = all ? itemIds.length - items.length : 0;
  const target = targets?.find((t) => t.id === targetId);
  const chars = estimateDescriptionChars(items);
  const tokens = items.reduce((n, i) => n + i.descriptionTokens, 0);
  const base = { name: loadout.name, description: loadout.description, color: loadout.color ?? null };

  return (
    <div className="flex flex-col gap-4">
      <LoadoutHeader
        key={`${loadout.name}|${loadout.description}|${loadout.color ?? ""}`}
        loadout={loadout}
        saving={update.isPending}
        onSave={(patch) => update.mutate({ ...base, ...patch, itemIds })}
        onDelete={() => setConfirmDelete(true)}
      />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.9fr)]">
        <div className="flex min-w-0 flex-col gap-4">
          <div className="surface overflow-hidden">
            <div className="flex items-center gap-2 border-b bg-sunken/60 px-3 py-1.5 text-xs">
              <span className="font-medium">Items</span>
              <span className="num text-subtle">{itemIds.length}</span>
              {missing ? <Badge tone="warn">{pluralize(missing, "item")} missing from the library</Badge> : null}
              <Button size="sm" variant="secondary" className="ml-auto" onClick={() => setPickerOpen(true)}>
                <Plus /> Add items…
              </Button>
            </div>
            {itemIds.length === 0 ? (
              <EmptyState
                compact
                className="m-3"
                title="Empty loadout"
                description="Add skills, agents or commands that belong together."
                action={
                  <Button size="sm" onClick={() => setPickerOpen(true)}>
                    <Plus /> Add items…
                  </Button>
                }
              />
            ) : !all ? (
              <div className="flex flex-col gap-2 p-3">
                {itemIds.map((iid) => (
                  <Skeleton key={iid} className="h-9" />
                ))}
              </div>
            ) : (
              <ul className="divide-y">
                {itemIds.map((iid) => {
                  const it = all.find((i) => i.id === iid);
                  return (
                    <li key={iid} className="flex items-center gap-3 px-3 py-2 text-xs">
                      {it ? <KindIcon kind={it.kind} className="size-4 shrink-0 text-muted" /> : <span className="size-4 shrink-0" />}
                      <div className="min-w-0 flex-1">
                        {it ? (
                          <button type="button" onClick={() => openItem(it.id)} className="truncate text-[13px] font-medium hover:underline">
                            {it.name}
                          </button>
                        ) : (
                          <span className="mono text-[13px] text-subtle">{iid} (missing)</span>
                        )}
                        {it ? <p className="line-clamp-1 text-muted">{it.description}</p> : null}
                      </div>
                      {it ? (
                        <Tooltip content="Description tokens · instruction tokens">
                          <span className="num shrink-0 text-[11px] text-subtle">
                            {it.descriptionTokens} desc · {formatTokens(it.tokenEstimate)} tok
                          </span>
                        </Tooltip>
                      ) : null}
                      <Tooltip content="Remove from loadout">
                        <Button
                          size="icon-sm"
                          variant="ghost"
                          aria-label={`Remove ${it?.name ?? iid} from loadout`}
                          disabled={update.isPending}
                          onClick={() => update.mutate({ ...base, itemIds: itemIds.filter((x) => x !== iid) })}
                        >
                          <X />
                        </Button>
                      </Tooltip>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>

          <div className="grid gap-3 md:grid-cols-2">
            <div className="surface p-3">
              <BudgetMeter chars={chars} budgetChars={target?.descriptionBudgetChars} tokens={tokens} />
              <p className="mt-2 text-[11px] text-subtle">
                The loadout's descriptions alone, against {target?.name ?? targetId}'s budget. The plan on the right accounts for whatever else is installed.
              </p>
            </div>
            <div className="surface p-3">
              <div className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle">Conflicts within the loadout</div>
              {itemIds.length === 0 ? (
                <p className="text-xs text-muted">Add items to check.</p>
              ) : analysis.data ? (
                <ConflictList report={analysis.data} compact />
              ) : analysis.isError ? (
                <ErrorNote error={analysis.error} />
              ) : (
                <Skeleton className="h-6" />
              )}
            </div>
          </div>
          <ErrorNote error={update.error} />
        </div>

        <ApplyLoadoutCard loadout={loadout} targetId={targetId} onTargetChange={setTargetId} />
      </div>

      <ItemPickerDialog
        open={pickerOpen}
        onOpenChange={setPickerOpen}
        title={`Add items to ${loadout.name}`}
        excludeIds={itemIds}
        loading={update.isPending}
        onConfirm={(ids) => {
          update.mutate({ ...base, itemIds: [...itemIds, ...ids] });
          setPickerOpen(false);
        }}
      />
      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={`Delete ${loadout.name}?`}
        description="The loadout is removed. Items stay in the library and anything already deployed stays deployed."
        confirmLabel="Delete"
        destructive
        loading={remove.isPending}
        onConfirm={() => remove.mutate(undefined)}
      />
    </div>
  );
}

/** Keyed by the saved name/description/colour so saves re-seed the fields without touching item edits. */
function LoadoutHeader({
  loadout,
  saving,
  onSave,
  onDelete,
}: {
  loadout: T.Loadout;
  saving: boolean;
  onSave: (patch: { name: string; description: string; color: string | null }) => void;
  onDelete: () => void;
}) {
  const [name, setName] = React.useState(loadout.name);
  const [description, setDescription] = React.useState(loadout.description);
  const [color, setColor] = React.useState<string | null>(loadout.color ?? null);
  const dirty = name !== loadout.name || description !== loadout.description || (color ?? null) !== (loadout.color ?? null);
  const reset = () => {
    setName(loadout.name);
    setDescription(loadout.description);
    setColor(loadout.color ?? null);
  };
  return (
    <div className="surface flex items-start gap-3 p-4">
      <LoadoutDot color={color} className="mt-3 size-3" />
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <Input value={name} onChange={(e) => setName(e.target.value)} aria-label="Loadout name" className="h-9 text-base font-semibold tracking-[-0.01em]" />
        <Textarea value={description} onChange={(e) => setDescription(e.target.value)} rows={2} aria-label="Loadout description" placeholder="What this set is for and when to apply it…" className="min-h-[44px] text-xs" />
        <div className="flex flex-wrap items-center gap-3">
          <ColorSwatches value={color} onChange={setColor} />
          <span className="ml-auto text-[11px] text-subtle">
            {pluralize(loadout.itemIds.length, "item")} · updated {relativeTime(loadout.updatedAt)}
          </span>
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        {dirty ? (
          <>
            <Button size="sm" variant="ghost" onClick={reset}>
              Discard
            </Button>
            <Button size="sm" variant="primary" loading={saving} disabled={!name.trim()} onClick={() => onSave({ name: name.trim(), description: description.trim(), color })}>
              <Save /> Save
            </Button>
          </>
        ) : null}
        <Tooltip content="Delete loadout">
          <Button size="icon-sm" variant="danger-ghost" onClick={onDelete} aria-label="Delete loadout">
            <Trash2 />
          </Button>
        </Tooltip>
      </div>
    </div>
  );
}
