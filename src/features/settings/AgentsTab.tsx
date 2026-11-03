import * as React from "react";
import { toast } from "sonner";
import { ExternalLink, FolderInput, Monitor, Plus, RotateCcw, Trash2 } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useTargets } from "@/lib/api/hooks";
import { cn } from "@/lib/utils";
import { Badge, Button, ConfirmDialog, EmptyState, Skeleton, Tooltip, openExternal, pickDirectory } from "@/components/ui";
import { ErrorNote } from "@/components/app/common";
import { AddTargetDialog } from "@/features/agents/AddTargetDialog";

const INVALIDATE = [keys.targets, keys.installedTargets, ["targets"], keys.dashboard, keys.deployments] as const;

export function AgentsTab() {
  const { data: targets, isLoading, error } = useTargets();
  const [addOpen, setAddOpen] = React.useState(false);
  const [pendingRemove, setPendingRemove] = React.useState<T.TargetSpec | null>(null);

  const setDir = useApiMutation(({ id, dir }: { id: string; dir: string | null }) => api.setTargetSkillsDir(id, dir), INVALIDATE, (_, { dir }) => toast.success(dir ? "Skills folder changed" : "Skills folder reset to the default"));
  const remove = useApiMutation((id: string) => api.removeCustomTarget(id), INVALIDATE, () => {
    toast.success("Custom agent removed");
    setPendingRemove(null);
  });

  const change = async (t: T.TargetSpec) => {
    const dir = await pickDirectory(`Choose the skills folder for ${t.name}`);
    if (dir) setDir.mutate({ id: t.id, dir });
  };

  const th = "px-3 py-2 text-left text-[11px] font-medium uppercase tracking-wide text-subtle whitespace-nowrap";
  const td = "px-3 py-2 align-middle";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="max-w-xl text-xs text-muted">Agent tools Skill Hive can deploy to. Built-in agents are detected by their directory under your home folder; override where an agent reads skills from, or add a tool that is not listed.</p>
        <Button variant="primary" onClick={() => setAddOpen(true)}>
          <Plus /> Add custom agent
        </Button>
      </div>
      <ErrorNote error={error ?? setDir.error} />
      {isLoading ? (
        <Skeleton className="h-64 w-full" />
      ) : !targets || targets.length === 0 ? (
        <EmptyState icon={Monitor} title="No agents" description="Add a custom agent to start deploying." />
      ) : (
        <div className="surface overflow-x-auto">
          <table className="w-full text-xs">
            <thead>
              <tr className="border-b bg-sunken/60">
                <th className={th}>Agent</th>
                <th className={th}>Category</th>
                <th className={th}>User skills</th>
                <th className={th}>Project skills</th>
                <th className={th}>Detect</th>
                <th className={cn(th, "text-right")}>Budget</th>
                <th className={th}>
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {targets.map((t) => (
                <tr key={t.id} className="border-b last:border-b-0 hover:bg-sunken/40">
                  <td className={td}>
                    <div className="flex items-center gap-2 text-[13px] font-medium">
                      {t.name}
                      {t.custom ? <Badge tone="outline">Custom</Badge> : null}
                      {t.docsUrl ? (
                        <Tooltip content="Documentation">
                          <Button variant="ghost" size="icon-sm" aria-label={`${t.name} documentation`} onClick={() => void openExternal(t.docsUrl!)}>
                            <ExternalLink />
                          </Button>
                        </Tooltip>
                      ) : null}
                    </div>
                    <div className="mono text-[11px] text-subtle">{t.id}</div>
                  </td>
                  <td className={cn(td, "text-muted")}>{t.category}</td>
                  <td className={cn(td, "mono selectable")}>~/{t.userSkillsDir}</td>
                  <td className={cn(td, "mono selectable text-muted")}>{t.projectSkillsDir ? `./${t.projectSkillsDir}` : "—"}</td>
                  <td className={cn(td, "mono selectable text-muted")}>~/{t.detectDir}</td>
                  <td className={cn(td, "num text-right")}>{t.descriptionBudgetChars ? t.descriptionBudgetChars.toLocaleString() : <span className="text-subtle">unknown</span>}</td>
                  <td className={cn(td, "whitespace-nowrap text-right")}>
                    <Tooltip content="Change skills folder…">
                      <Button variant="ghost" size="icon-sm" aria-label={`Change skills folder for ${t.name}`} loading={setDir.isPending && setDir.variables?.id === t.id && !!setDir.variables.dir} onClick={() => void change(t)}>
                        <FolderInput />
                      </Button>
                    </Tooltip>
                    <Tooltip content="Reset to the default folder">
                      <Button variant="ghost" size="icon-sm" aria-label={`Reset skills folder for ${t.name}`} onClick={() => setDir.mutate({ id: t.id, dir: null })}>
                        <RotateCcw />
                      </Button>
                    </Tooltip>
                    {t.custom ? (
                      <Tooltip content="Remove custom agent">
                        <Button variant="danger-ghost" size="icon-sm" aria-label={`Remove ${t.name}`} onClick={() => setPendingRemove(t)}>
                          <Trash2 />
                        </Button>
                      </Tooltip>
                    ) : null}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <AddTargetDialog open={addOpen} onOpenChange={setAddOpen} onCreated={() => setAddOpen(false)} existingIds={(targets ?? []).map((t) => t.id)} />
      <ConfirmDialog
        open={!!pendingRemove}
        onOpenChange={(o) => !o && setPendingRemove(null)}
        title={`Remove ${pendingRemove?.name}?`}
        description="Deployments recorded for it are forgotten; files in its directory are not touched."
        confirmLabel="Remove"
        destructive
        loading={remove.isPending}
        onConfirm={() => {
          if (pendingRemove) remove.mutate(pendingRemove.id);
        }}
      />
    </div>
  );
}
