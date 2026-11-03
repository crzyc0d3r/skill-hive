import { FolderPlus } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useProjects } from "@/lib/api/hooks";
import { cn } from "@/lib/utils";
import { Button, SimpleSelect, Tabs, TabsList, TabsTrigger, pickDirectory } from "@/components/ui";

/** User / Project switch with a project chooser and "Add project…" (registers the picked folder). */
export function ScopePicker({
  scope,
  onScopeChange,
  projectPath,
  onProjectChange,
  projectDisabled,
  projectDisabledReason = "This agent has no project-level skills folder",
  className,
}: {
  scope: T.Scope;
  onScopeChange: (s: T.Scope) => void;
  projectPath: string | null;
  onProjectChange: (path: string) => void;
  projectDisabled?: boolean;
  projectDisabledReason?: string;
  className?: string;
}) {
  const { data: projects } = useProjects();
  const addProject = useApiMutation((dir: string) => api.addProject(dir), [keys.projects], (p) => onProjectChange(p.path));
  return (
    <div className={cn("flex flex-wrap items-center gap-2", className)}>
      <Tabs value={scope} onValueChange={(v) => onScopeChange(v as T.Scope)}>
        <TabsList aria-label="Scope">
          <TabsTrigger value="user">User</TabsTrigger>
          <TabsTrigger value="project" disabled={projectDisabled}>
            Project
          </TabsTrigger>
        </TabsList>
      </Tabs>
      {/* Inline, not a tooltip: disabled buttons emit no pointer events in WebKit and are skipped by the roving tab index. */}
      {projectDisabled ? <span className="text-[11px] text-subtle">{projectDisabledReason}</span> : null}
      {scope === "project" ? (
        <>
          <SimpleSelect
            size="sm"
            className="w-52"
            value={projectPath}
            onChange={onProjectChange}
            placeholder="Choose a project"
            options={(projects ?? []).map((p) => ({ value: p.path, label: p.name }))}
          />
          <Button
            size="sm"
            variant="ghost"
            loading={addProject.isPending}
            onClick={async () => {
              const dir = await pickDirectory("Choose a project folder");
              if (dir) addProject.mutate(dir);
            }}
          >
            <FolderPlus /> Add project…
          </Button>
        </>
      ) : null}
    </div>
  );
}
