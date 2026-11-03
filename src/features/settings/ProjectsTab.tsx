import { toast } from "sonner";
import { FolderGit2, FolderPlus, X } from "lucide-react";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useProjects } from "@/lib/api/hooks";
import { displayPath, relativeTime } from "@/lib/utils";
import { Button, EmptyState, Skeleton, Tooltip, pickDirectory } from "@/components/ui";
import { ErrorNote } from "@/components/app/common";

const INVALIDATE = [keys.projects, keys.dashboard, ["targets"]] as const;

export function ProjectsTab() {
  const { data: projects, isLoading, error } = useProjects();
  const add = useApiMutation((path: string) => api.addProject(path), INVALIDATE, (p) => toast.success(`Added ${p.name}`));
  const remove = useApiMutation((path: string) => api.removeProject(path), INVALIDATE, () => toast.success("Project removed"));

  const pick = async () => {
    const dir = await pickDirectory("Choose a project folder");
    if (!dir) return;
    if (projects?.some((p) => p.path === dir)) {
      toast("Already in the list");
      return;
    }
    add.mutate(dir);
  };

  return (
    <div className="flex max-w-3xl flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="max-w-xl text-xs text-muted">Projects you deploy skills into at project scope (for example a repository's .claude/skills). Listed here so Agents and Loadouts can target them without picking a folder each time.</p>
        <Button variant="primary" loading={add.isPending} onClick={() => void pick()}>
          <FolderPlus /> Add project…
        </Button>
      </div>
      <ErrorNote error={error ?? add.error ?? remove.error} />
      {isLoading ? (
        <Skeleton className="h-32 w-full" />
      ) : !projects || projects.length === 0 ? (
        <EmptyState
          icon={FolderGit2}
          title="No projects yet"
          description="Add a repository or workspace folder to deploy skills into it at project scope."
          action={
            <Button variant="primary" onClick={() => void pick()}>
              <FolderPlus /> Add project…
            </Button>
          }
        />
      ) : (
        <ul className="surface divide-y overflow-hidden">
          {projects.map((p) => (
            <li key={p.path} className="flex items-center gap-3 px-3 py-2">
              <span className="flex size-7 shrink-0 items-center justify-center rounded-md bg-sunken text-muted">
                <FolderGit2 className="size-4" />
              </span>
              <div className="min-w-0 flex-1">
                <div className="text-[13px] font-medium">{p.name}</div>
                <div className="mono truncate text-[11px] text-subtle selectable" title={p.path}>
                  {displayPath(p.path)}
                </div>
              </div>
              <span className="text-[11px] text-subtle">added {relativeTime(p.addedAt)}</span>
              <Tooltip content="Remove from the list">
                <Button variant="ghost" size="icon-sm" aria-label={`Remove ${p.name}`} loading={remove.isPending && remove.variables === p.path} onClick={() => remove.mutate(p.path)}>
                  <X />
                </Button>
              </Tooltip>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
