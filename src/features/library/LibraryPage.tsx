import * as React from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { ArrowDownUp, Check, Filter, LayoutGrid, Library as LibraryIcon, Plus, RefreshCw, Rows3, Search, Star, Trash2, Upload, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useItems, useStats, useTargets } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { cn, pluralize } from "@/lib/utils";
import {
  Badge,
  Button,
  ConfirmDialog,
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  EmptyState,
  Input,
  Tooltip,
} from "@/components/ui";
import { KIND_LABEL, Page, PageBody, PageHeader, SOURCE_LABEL, TagChip } from "@/components/app/common";
import { ItemCard, ItemCardSkeleton, ItemRow } from "./ItemCard";
import { CreateSkillDialog } from "./CreateSkillDialog";
import { DeployDialog } from "@/features/agents/DeployDialog";
import { UpdatesDialog } from "./UpdatesDialog";

const SORTS: { value: T.LibrarySort; label: string }[] = [
  { value: "relevance", label: "Relevance" },
  { value: "name", label: "Name" },
  { value: "updated", label: "Recently updated" },
  { value: "effectiveness", label: "Effectiveness" },
];

export function LibraryPage() {
  const [params, setParams] = useSearchParams();
  const navigate = useNavigate();
  const [text, setText] = React.useState(params.get("q") ?? "");
  const [debounced, setDebounced] = React.useState(text);
  const [kinds, setKinds] = React.useState<T.AssetKind[]>([]);
  const [tags, setTags] = React.useState<string[]>(params.get("tag") ? [params.get("tag")!] : []);
  const [sources, setSources] = React.useState<T.SourceKind[]>([]);
  const [favorites, setFavorites] = React.useState(false);
  const [deployedTo, setDeployedTo] = React.useState<string | null>(params.get("target"));
  const [sort, setSort] = React.useState<T.LibrarySort>("relevance");
  const [view, setView] = React.useState<"grid" | "list">(() => (localStorage.getItem("hive.library.view") as "grid" | "list") || "grid");
  const [createOpen, setCreateOpen] = React.useState(false);
  const [deployOpen, setDeployOpen] = React.useState(false);
  const [confirmDelete, setConfirmDelete] = React.useState(false);
  const [updatesOpen, setUpdatesOpen] = React.useState(params.get("updates") === "1");

  const openItem = useUi((s) => s.openItem);
  const selectedIds = useUi((s) => s.selectedIds);
  const toggleSelected = useUi((s) => s.toggleSelected);
  const clearSelection = useUi((s) => s.clearSelection);
  const setSelection = useUi((s) => s.setSelection);

  React.useEffect(() => {
    const t = window.setTimeout(() => setDebounced(text), 160);
    return () => window.clearTimeout(t);
  }, [text]);
  React.useEffect(() => {
    localStorage.setItem("hive.library.view", view);
  }, [view]);
  React.useEffect(() => () => clearSelection(), [clearSelection]);

  const query: T.LibraryQuery = React.useMemo(
    () => ({ text: debounced || null, kinds, tags, sourceKinds: sources, favoritesOnly: favorites, deployedTo, sort, limit: null }),
    [debounced, kinds, tags, sources, favorites, deployedTo, sort],
  );
  const { data: items, isLoading, isError, error, refetch } = useItems(query);
  const { data: stats } = useStats();
  const { data: targets } = useTargets();

  const removeMany = useApiMutation(
    async (ids: string[]) => {
      for (const id of ids) await api.removeItem(id);
      return ids.length;
    },
    [keys.itemsAll, keys.stats, keys.dashboard],
    (n) => {
      toast.success(`Removed ${pluralize(n, "item")}`);
      clearSelection();
      setConfirmDelete(false);
    },
  );

  const toggleKind = (k: T.AssetKind) => setKinds((prev) => (prev.includes(k) ? prev.filter((x) => x !== k) : [...prev, k]));
  const toggleTag = (t: string) => setTags((prev) => (prev.includes(t) ? prev.filter((x) => x !== t) : [...prev, t]));
  const toggleSource = (s: T.SourceKind) => setSources((prev) => (prev.includes(s) ? prev.filter((x) => x !== s) : [...prev, s]));
  const activeFilters = kinds.length + tags.length + sources.length + (favorites ? 1 : 0) + (deployedTo ? 1 : 0);
  const clearFilters = () => {
    setKinds([]);
    setTags([]);
    setSources([]);
    setFavorites(false);
    setDeployedTo(null);
    setParams({});
  };

  const allSelected = !!items && items.length > 0 && items.every((i) => selectedIds.includes(i.id));

  return (
    <Page>
      <PageHeader
        title="Library"
        description={
          stats
            ? `${pluralize(stats.skills, "skill")} · ${pluralize(stats.agents, "agent")} · ${pluralize(stats.commands, "command")} · ${stats.deployed} deployed · ${stats.tested} measured`
            : "Everything you have collected, in one place."
        }
        actions={
          <>
            <Button variant="ghost" onClick={() => setUpdatesOpen(true)}>
              <RefreshCw />
              Check updates
            </Button>
            <Button variant="secondary" onClick={() => navigate("/discover")}>
              <Upload />
              Collect
            </Button>
            <Button variant="primary" onClick={() => setCreateOpen(true)}>
              <Plus />
              New skill
            </Button>
          </>
        }
      >
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative w-72">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
            <Input value={text} onChange={(e) => setText(e.target.value)} placeholder="Search name, description, body…" className="pl-8 pr-7" />
            {text ? (
              <button type="button" onClick={() => setText("")} className="absolute right-2 top-1/2 -translate-y-1/2 text-subtle hover:text-fg" aria-label="Clear">
                <X className="size-3.5" />
              </button>
            ) : null}
          </div>

          <div className="flex items-center gap-0.5 rounded-lg bg-sunken p-0.5" role="group" aria-label="Kinds">
            {(["skill", "agent", "command"] as T.AssetKind[]).map((k) => (
              <button
                key={k}
                type="button"
                onClick={() => toggleKind(k)}
                aria-pressed={kinds.includes(k)}
                className={cn(
                  "h-7 rounded-md px-2.5 text-xs font-medium transition-colors",
                  kinds.includes(k) ? "bg-elevated text-fg shadow-sm" : "text-muted hover:text-fg",
                )}
              >
                {KIND_LABEL[k]}s
              </button>
            ))}
          </div>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost">
                <Filter />
                Filters
                {activeFilters ? <Badge tone="accent">{activeFilters}</Badge> : null}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="w-64">
              <DropdownMenuCheckboxItem checked={favorites} onCheckedChange={(v) => setFavorites(!!v)}>
                <Star className="size-4" /> Favourites only
              </DropdownMenuCheckboxItem>
              <DropdownMenuSeparator />
              <DropdownMenuLabel>Source</DropdownMenuLabel>
              {(["github", "git", "local_path", "agent_dir", "authored", "url", "catalog"] as T.SourceKind[]).map((s) => (
                <DropdownMenuCheckboxItem key={s} checked={sources.includes(s)} onCheckedChange={() => toggleSource(s)}>
                  {SOURCE_LABEL[s]}
                </DropdownMenuCheckboxItem>
              ))}
              <DropdownMenuSeparator />
              <DropdownMenuLabel>Deployed to</DropdownMenuLabel>
              <DropdownMenuRadioGroup value={deployedTo ?? ""} onValueChange={(v) => setDeployedTo(v || null)}>
                <DropdownMenuRadioItem value="">Anywhere</DropdownMenuRadioItem>
                {targets?.map((t) => (
                  <DropdownMenuRadioItem key={t.id} value={t.id}>
                    {t.name}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
              {activeFilters ? (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem onSelect={clearFilters}>
                    <X className="size-4" /> Clear filters
                  </DropdownMenuItem>
                </>
              ) : null}
            </DropdownMenuContent>
          </DropdownMenu>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost">
                <ArrowDownUp />
                {SORTS.find((s) => s.value === sort)?.label}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuRadioGroup value={sort} onValueChange={(v) => setSort(v as T.LibrarySort)}>
                {SORTS.map((s) => (
                  <DropdownMenuRadioItem key={s.value} value={s.value}>
                    {s.label}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>

          <div className="ml-auto flex items-center gap-0.5 rounded-lg bg-sunken p-0.5">
            <Tooltip content="Grid">
              <button
                type="button"
                onClick={() => setView("grid")}
                aria-label="Grid view"
                aria-pressed={view === "grid"}
                className={cn("flex size-7 items-center justify-center rounded-md", view === "grid" ? "bg-elevated shadow-sm" : "text-muted hover:text-fg")}
              >
                <LayoutGrid className="size-3.5" />
              </button>
            </Tooltip>
            <Tooltip content="List">
              <button
                type="button"
                onClick={() => setView("list")}
                aria-label="List view"
                aria-pressed={view === "list"}
                className={cn("flex size-7 items-center justify-center rounded-md", view === "list" ? "bg-elevated shadow-sm" : "text-muted hover:text-fg")}
              >
                <Rows3 className="size-3.5" />
              </button>
            </Tooltip>
          </div>
        </div>

        {stats && stats.tags.length > 0 ? (
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-[11px] text-subtle">Tags</span>
            {stats.tags.slice(0, 14).map((t) => (
              <TagChip key={t.tag} tag={`${t.tag} ${t.count}`} active={tags.includes(t.tag)} onClick={() => toggleTag(t.tag)} />
            ))}
          </div>
        ) : null}
      </PageHeader>

      {selectedIds.length > 0 ? (
        <div className="mx-6 mb-3 flex items-center gap-2 rounded-xl bg-accent-soft/60 px-3 py-2 text-xs hairline animate-slide-up">
          <Check className="size-3.5 text-accent" />
          <span className="font-medium">{pluralize(selectedIds.length, "item")} selected</span>
          <Button size="sm" variant="ghost" onClick={() => (allSelected ? clearSelection() : setSelection(items?.map((i) => i.id) ?? []))}>
            {allSelected ? "Deselect all" : "Select all"}
          </Button>
          <span className="ml-auto flex items-center gap-1">
            <Button size="sm" variant="secondary" onClick={() => setDeployOpen(true)}>
              Deploy…
            </Button>
            <Button size="sm" variant="secondary" onClick={() => navigate(`/find?ids=${selectedIds.join(",")}`)}>
              Check conflicts
            </Button>
            <Button size="sm" variant="danger-ghost" onClick={() => setConfirmDelete(true)}>
              <Trash2 /> Remove
            </Button>
            <Button size="sm" variant="ghost" onClick={clearSelection} aria-label="Clear selection">
              <X />
            </Button>
          </span>
        </div>
      ) : null}

      <PageBody>
        {isLoading ? (
          <div className="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(300px,1fr))]">
            {Array.from({ length: 8 }).map((_, i) => (
              <ItemCardSkeleton key={i} />
            ))}
          </div>
        ) : isError ? (
          <EmptyState title="Could not load the library" description={(error as Error).message} action={<Button onClick={() => refetch()}>Retry</Button>} />
        ) : !items || items.length === 0 ? (
          activeFilters || debounced ? (
            <EmptyState icon={Search} title="No matches" description="Try a different search or clear the filters." action={<Button onClick={clearFilters}>Clear filters</Button>} />
          ) : (
            <EmptyState
              icon={LibraryIcon}
              title="Your library is empty"
              description="Collect skills already on this machine, pull a collection from GitHub, or write a new one."
              action={
                <>
                  <Button variant="primary" onClick={() => navigate("/discover")}>
                    <Upload /> Collect skills
                  </Button>
                  <Button onClick={() => setCreateOpen(true)}>
                    <Plus /> New skill
                  </Button>
                </>
              }
            />
          )
        ) : view === "grid" ? (
          <div className="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(300px,1fr))]">
            {items.map((item) => (
              <ItemCard
                key={item.id}
                item={item}
                onOpen={() => openItem(item.id)}
                selectable
                selected={selectedIds.includes(item.id)}
                onToggleSelect={() => toggleSelected(item.id)}
              />
            ))}
          </div>
        ) : (
          <div className="surface overflow-x-auto">
            <div className="grid min-w-[900px] grid-cols-[1rem_minmax(0,1.2fr)_minmax(0,2fr)_12.5rem_10rem_13rem] gap-3 border-b bg-sunken/60 px-3 py-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle">
              <span className="w-4" />
              <span>Name</span>
              <span>Description</span>
              <span>Status</span>
              <span>Tags</span>
              <span className="text-right">Source · Size · Updated</span>
            </div>
            {items.map((item) => (
              <ItemRow key={item.id} item={item} onOpen={() => openItem(item.id)} selected={selectedIds.includes(item.id)} onToggleSelect={() => toggleSelected(item.id)} />
            ))}
          </div>
        )}
      </PageBody>

      <CreateSkillDialog open={createOpen} onOpenChange={setCreateOpen} />
      <DeployDialog open={deployOpen} onOpenChange={setDeployOpen} itemIds={selectedIds} onDone={clearSelection} />
      <UpdatesDialog open={updatesOpen} onOpenChange={setUpdatesOpen} />
      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={`Remove ${pluralize(selectedIds.length, "item")} from the library?`}
        description="Their deployments are removed too. Files in the library are deleted; sources are untouched."
        confirmLabel="Remove"
        destructive
        loading={removeMany.isPending}
        onConfirm={() => removeMany.mutate(selectedIds)}
      />
    </Page>
  );
}
