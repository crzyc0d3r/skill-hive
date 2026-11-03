import * as React from "react";
import { toast } from "sonner";
import { AlertTriangle, FolderOpen, Rocket } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useProjects, useSettings, useTargets } from "@/lib/api/hooks";
import { useQuery } from "@tanstack/react-query";
import { pluralize } from "@/lib/utils";
import {
  Button,
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Field,
  SimpleSelect,
  Switch,
  pickDirectory,
} from "@/components/ui";
import { ConflictList, ErrorNote } from "@/components/app/common";

export function DeployDialog({
  open,
  onOpenChange,
  itemIds,
  defaultTarget,
  defaultScope,
  defaultProject,
  onDone,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  itemIds: string[];
  defaultTarget?: string;
  /** Preselected scope (e.g. the scope currently shown on the Agents screen). */
  defaultScope?: T.Scope;
  /** Preselected project path for project scope. */
  defaultProject?: string | null;
  onDone?: () => void;
}) {
  const { data: targets } = useTargets();
  const { data: projects } = useProjects();
  const { data: settings } = useSettings();
  const [targetOverride, setTargetId] = React.useState<string | null>(null);
  const targetId = targetOverride ?? defaultTarget ?? "claude-code";
  const [scope, setScope] = React.useState<T.Scope>(defaultScope ?? "user");
  const [projectOverride, setProjectPath] = React.useState<string | null>(null);
  const projectPath = projectOverride ?? defaultProject ?? projects?.[0]?.path ?? null;
  const [mode, setMode] = React.useState<T.LinkMode | null>(null);
  const [force, setForce] = React.useState(false);

  const { data: conflicts } = useQuery({
    queryKey: ["analyze", itemIds, targetId],
    queryFn: () => api.analyzeItems(itemIds, targetId),
    enabled: open && itemIds.length > 1,
  });

  const deploy = useApiMutation(
    () => api.deploy({ itemIds, targetId, scope, projectPath: scope === "project" ? projectPath : null, mode, force }),
    [keys.deployments, keys.targets, keys.itemsAll, ["item"], keys.dashboard],
    (result) => {
      if (result.deployed.length) toast.success(`Deployed ${pluralize(result.deployed.length, "item")} to ${targets?.find((t) => t.id === targetId)?.name ?? targetId}`);
      for (const f of result.failed) toast.error(f.reason);
      if (!result.failed.length) {
        onOpenChange(false);
        onDone?.();
      }
    },
  );

  const target = targets?.find((t) => t.id === targetId);
  const effectiveMode = mode ?? settings?.defaultLinkMode ?? "link";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Deploy {pluralize(itemIds.length, "item")}</DialogTitle>
          <DialogDescription>
            Links (or copies) the library version into the agent's directory. Linked deployments always see the latest library content.
          </DialogDescription>
        </DialogHeader>
        <DialogBody className="flex flex-col gap-4">
          <Field label="Agent">
            <SimpleSelect value={targetId} onChange={setTargetId} options={(targets ?? []).map((t) => ({ value: t.id, label: t.name }))} />
          </Field>
          <div className="grid grid-cols-2 gap-4">
            <Field label="Scope">
              <SimpleSelect
                value={scope}
                onChange={(v) => setScope(v as T.Scope)}
                options={[
                  { value: "user", label: "User (all projects)" },
                  { value: "project", label: "One project", disabled: !target?.projectSkillsDir },
                ]}
              />
            </Field>
            <Field label="Mode">
              <SimpleSelect
                value={effectiveMode}
                onChange={(v) => setMode(v as T.LinkMode)}
                options={[
                  { value: "link", label: "Link (follows the library)" },
                  { value: "copy", label: "Copy (independent)" },
                ]}
              />
            </Field>
          </div>
          {scope === "project" ? (
            <Field label="Project">
              <div className="flex gap-2">
                <SimpleSelect value={projectPath} onChange={setProjectPath} placeholder="Choose a project" options={(projects ?? []).map((p) => ({ value: p.path, label: p.name }))} className="flex-1" />
                <Button
                  variant="secondary"
                  onClick={async () => {
                    const dir = await pickDirectory("Choose a project folder");
                    if (dir) {
                      const p = await api.addProject(dir);
                      setProjectPath(p.path);
                    }
                  }}
                >
                  <FolderOpen /> Browse
                </Button>
              </div>
            </Field>
          ) : null}
          <label className="flex items-center justify-between gap-3 rounded-lg bg-sunken px-3 py-2 text-xs">
            <span>
              <span className="font-medium">Replace unmanaged copies</span>
              <span className="block text-muted">If an item with the same name already exists and Skill Hive did not put it there, overwrite it.</span>
            </span>
            <Switch checked={force} onCheckedChange={setForce} />
          </label>
          {conflicts && conflicts.conflicts.length ? (
            <div>
              <div className="mb-1.5 flex items-center gap-1.5 text-xs font-medium text-warn">
                <AlertTriangle className="size-3.5" /> These items conflict with each other
              </div>
              <ConflictList report={conflicts} compact />
            </div>
          ) : null}
          <ErrorNote error={deploy.error} />
        </DialogBody>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" loading={deploy.isPending} disabled={scope === "project" && !projectPath} onClick={() => deploy.mutate(undefined)}>
            <Rocket /> Deploy
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
