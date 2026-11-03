import * as React from "react";
import { ExternalLink, Eye, RefreshCw, Search, Sparkles } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useCatalog } from "@/lib/api/hooks";
import { pluralize } from "@/lib/utils";
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle, EmptyState, Input, Skeleton, openExternal } from "@/components/ui";
import { ErrorNote, TagChip } from "@/components/app/common";

/** "Catalog": curated collections, each previewable through the Online tab. */
export function CatalogTab({ onPreview }: { onPreview: (source: string) => void }) {
  const catalog = useCatalog();
  const [filter, setFilter] = React.useState("");
  const refresh = useApiMutation(() => api.catalog(true), [keys.catalog]);

  const list = React.useMemo(() => {
    const q = filter.trim().toLowerCase();
    const all = catalog.data ?? [];
    if (!q) return all;
    return all.filter((c) => `${c.name} ${c.publisher} ${c.description} ${c.source} ${c.tags.join(" ")}`.toLowerCase().includes(q));
  }, [catalog.data, filter]);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <div className="relative w-72">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-subtle" />
          <Input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Filter by name, publisher or tag…" className="pl-8" aria-label="Filter catalog" />
        </div>
        {catalog.data ? (
          <span className="text-xs text-muted">
            {filter ? `${list.length} of ` : ""}
            {pluralize(catalog.data.length, "curated collection")}
          </span>
        ) : null}
        <Button variant="ghost" size="sm" className="ml-auto" loading={refresh.isPending} onClick={() => refresh.mutate(undefined)}>
          <RefreshCw /> Refresh
        </Button>
      </div>

      {catalog.isLoading ? (
        <div className="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(300px,1fr))]">
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className="h-40" />
          ))}
        </div>
      ) : catalog.isError ? (
        <div className="flex flex-col gap-2">
          <ErrorNote error={catalog.error} />
          <div>
            <Button size="sm" onClick={() => void catalog.refetch()}>
              Retry
            </Button>
          </div>
        </div>
      ) : list.length === 0 ? (
        <EmptyState icon={Sparkles} title={filter ? "No collections match" : "The catalog is empty"} description={filter ? "Try another word, or search GitHub from the Online tab." : "Refresh to fetch the curated list."} />
      ) : (
        <div className="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(300px,1fr))]">
          {list.map((entry) => (
            <CatalogCard key={entry.id} entry={entry} onPreview={() => onPreview(entry.source)} />
          ))}
        </div>
      )}
      <ErrorNote error={refresh.error} />
    </div>
  );
}

function CatalogCard({ entry, onPreview }: { entry: T.CatalogEntry; onPreview: () => void }) {
  return (
    <Card className="flex flex-col">
      <CardHeader>
        <div className="min-w-0">
          <CardTitle className="truncate">{entry.name}</CardTitle>
          <CardDescription>
            by {entry.publisher} · <span className="mono">{entry.source}</span>
          </CardDescription>
        </div>
      </CardHeader>
      <CardContent className="flex flex-1 flex-col gap-3">
        <p className="line-clamp-3 text-xs text-muted">{entry.description}</p>
        <div className="flex flex-wrap gap-1">
          {entry.tags.map((t) => (
            <TagChip key={t} tag={t} />
          ))}
        </div>
        <div className="mt-auto flex items-center gap-2 pt-1">
          <span className="text-[11px] text-subtle">{entry.approxCount ? `~${pluralize(entry.approxCount, "skill")}` : "Size unknown"}</span>
          <span className="ml-auto flex items-center gap-1">
            {entry.homepage ? (
              <Button size="sm" variant="ghost" onClick={() => void openExternal(entry.homepage!)}>
                <ExternalLink /> Homepage
              </Button>
            ) : null}
            <Button size="sm" variant="secondary" onClick={onPreview}>
              <Eye /> Preview
            </Button>
          </span>
        </div>
      </CardContent>
    </Card>
  );
}
