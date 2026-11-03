import * as React from "react";
import { toast } from "sonner";
import { ArrowUpCircle, CheckCircle2, RefreshCw } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useItems } from "@/lib/api/hooks";
import { Badge, Button, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, EmptyState } from "@/components/ui";
import { ErrorNote, SourceBadge } from "@/components/app/common";
import { shortHash } from "@/lib/utils";

export function UpdatesDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const { data: items } = useItems({ text: null, kinds: [], tags: [], sourceKinds: ["github", "git", "url", "catalog"], favoritesOnly: false, deployedTo: null, sort: "name", limit: null });
  const [checks, setChecks] = React.useState<T.UpdateCheck[] | null>(null);
  const check = useApiMutation(() => api.checkUpdates(null), [], (result) => setChecks(result));
  const update = useApiMutation((id: string) => api.updateItem(id), [keys.itemsAll, ["item"], keys.deployments], (item) => {
    toast.success(`Updated ${item.name}`);
    setChecks((prev) => prev?.map((c) => (c.itemId === item.id ? { ...c, updateAvailable: false, currentRevision: c.latestRevision } : c)) ?? null);
  });

  React.useEffect(() => {
    if (open && !checks && !check.isPending) check.mutate(undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const available = checks?.filter((c) => c.updateAvailable) ?? [];
  const byId = new Map(items?.map((i) => [i.id, i]) ?? []);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>Source updates</DialogTitle>
          <DialogDescription>Compares each item's recorded revision with its source. Linked deployments follow automatically; copied ones become stale until re-synced.</DialogDescription>
        </DialogHeader>
        <DialogBody>
          {check.isPending && !checks ? (
            <div className="flex items-center gap-2 py-8 text-sm text-muted">
              <RefreshCw className="size-4 animate-spin" /> Checking {items?.length ?? ""} sources…
            </div>
          ) : checks && checks.length === 0 ? (
            <EmptyState compact title="Nothing to check" description="Only items imported from GitHub, git or a URL can be updated." />
          ) : checks ? (
            <div className="flex flex-col gap-1.5">
              {checks.map((c) => {
                const item = byId.get(c.itemId);
                return (
                  <div key={c.itemId} className="flex items-center gap-3 rounded-lg px-3 py-2 hairline">
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 text-[13px] font-medium">
                        {item?.name ?? c.itemId}
                        {item ? <SourceBadge source={item.source} /> : null}
                      </div>
                      <div className="mono text-[11px] text-subtle">
                        {c.currentRevision ? shortHash(c.currentRevision) : "?"}
                        {c.latestRevision && c.latestRevision !== c.currentRevision ? ` → ${shortHash(c.latestRevision)}` : ""}
                        {c.error ? <span className="text-danger"> · {c.error}</span> : null}
                      </div>
                    </div>
                    {c.updateAvailable ? (
                      <Button size="sm" variant="primary" loading={update.isPending && update.variables === c.itemId} onClick={() => update.mutate(c.itemId)}>
                        <ArrowUpCircle /> Update
                      </Button>
                    ) : c.error ? (
                      <Badge tone="danger">Error</Badge>
                    ) : (
                      <Badge tone="ok">
                        <CheckCircle2 /> Up to date
                      </Badge>
                    )}
                  </div>
                );
              })}
            </div>
          ) : null}
          <ErrorNote error={check.error ?? update.error} className="mt-3" />
        </DialogBody>
        <DialogFooter className="justify-between">
          <span className="text-xs text-muted">{checks ? `${available.length} update${available.length === 1 ? "" : "s"} available` : ""}</span>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => check.mutate(undefined)} loading={check.isPending}>
              <RefreshCw /> Re-check
            </Button>
            {available.length > 1 ? (
              <Button
                variant="primary"
                onClick={async () => {
                  for (const c of available) await update.mutateAsync(c.itemId);
                }}
              >
                Update all
              </Button>
            ) : null}
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
