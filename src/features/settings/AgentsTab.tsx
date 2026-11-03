import * as React from "react";
import { toast } from "sonner";
import { ExternalLink, FolderInput, Monitor, Plus, RotateCcw, Trash2 } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useTargets } from "@/lib/api/hooks";
import { cn } from "@/lib/utils";
import { Badge, Button, ConfirmDialog, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, EmptyState, Field, Input, SimpleSelect, Skeleton, Tooltip, openExternal, pickDirectory } from "@/components/ui";
import { ErrorNote } from "@/components/app/common";

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
                      {t.custom ? <Badge tone="accent">custom</Badge> : null}
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
                        <Button variant="ghost" size="icon-sm" aria-label={`Remove ${t.name}`} onClick={() => setPendingRemove(t)}>
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
      <AddAgentDialog open={addOpen} onOpenChange={setAddOpen} existingIds={(targets ?? []).map((t) => t.id)} />
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

function AddAgentDialog({ open, onOpenChange, existingIds }: { open: boolean; onOpenChange: (o: boolean) => void; existingIds: string[] }) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>Add custom agent</DialogTitle>
          <DialogDescription>Describe where the tool reads skills from. Paths are relative to your home folder or to a project root.</DialogDescription>
        </DialogHeader>
        {open ? <AddAgentForm existingIds={existingIds} onClose={() => onOpenChange(false)} /> : null}
      </DialogContent>
    </Dialog>
  );
}

function AddAgentForm({ existingIds, onClose }: { existingIds: string[]; onClose: () => void }) {
  const [id, setId] = React.useState("");
  const [name, setName] = React.useState("");
  const [category, setCategory] = React.useState<T.TargetCategory>("coding");
  const [userDir, setUserDir] = React.useState("");
  const [projectDir, setProjectDir] = React.useState("");
  const [detectDir, setDetectDir] = React.useState("");
  const [budget, setBudget] = React.useState("15000");

  const add = useApiMutation((spec: T.TargetSpec) => api.addCustomTarget(spec), INVALIDATE, (spec) => {
    toast.success(`Added ${spec.name}`);
    onClose();
  });

  const idOk = /^[a-z0-9]+(-[a-z0-9]+)*$/.test(id) && !existingIds.includes(id);
  const problems: string[] = [];
  if (!idOk) problems.push(existingIds.includes(id) ? "That id is already used." : "Id must be a slug, e.g. my-agent.");
  if (!name.trim()) problems.push("Name is required.");
  if (!userDir.trim()) problems.push("User skills directory is required.");
  const budgetN = Number(budget);
  if (budget.trim() && (!Number.isFinite(budgetN) || budgetN < 100)) problems.push("Budget must be a number of characters (or blank if unknown).");

  const clean = (s: string) => s.trim().replace(/^~\//, "").replace(/^\.\//, "").replace(/\/+$/, "");
  const submit = () =>
    add.mutate({
      id,
      name: name.trim(),
      category,
      userSkillsDir: clean(userDir),
      projectSkillsDir: clean(projectDir) || null,
      userAgentsDir: null,
      projectAgentsDir: null,
      userCommandsDir: null,
      projectCommandsDir: null,
      detectDir: clean(detectDir) || clean(userDir).split("/")[0],
      descriptionBudgetChars: budget.trim() ? Math.round(budgetN) : null,
      docsUrl: null,
      custom: true,
    });

  return (
    <>
      <DialogBody className="flex flex-col gap-4 pb-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Id" hint="Stable identifier used in deployments and the CLI.">
            <Input value={id} onChange={(e) => setId(e.target.value.toLowerCase())} placeholder="my-agent" aria-label="Agent id" className="mono" autoFocus />
          </Field>
          <Field label="Name">
            <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="My Agent" aria-label="Agent name" />
          </Field>
          <Field label="Category">
            <SimpleSelect value={category} onChange={(v) => setCategory(v as T.TargetCategory)} options={[{ value: "coding", label: "Coding agent" }, { value: "assistant", label: "Assistant" }]} />
          </Field>
          <Field label="Description budget (chars)" hint="How much skill-description text the agent loads before truncating. Blank if unknown.">
            <Input type="number" min={100} step={500} value={budget} onChange={(e) => setBudget(e.target.value)} aria-label="Description budget" />
          </Field>
          <Field label="User skills directory" hint="Relative to your home folder.">
            <Input value={userDir} onChange={(e) => setUserDir(e.target.value)} placeholder=".my-agent/skills" aria-label="User skills directory" className="mono" spellCheck={false} />
          </Field>
          <Field label="Project skills directory" hint="Relative to a project root; blank if the agent has no project scope.">
            <Input value={projectDir} onChange={(e) => setProjectDir(e.target.value)} placeholder=".my-agent/skills" aria-label="Project skills directory" className="mono" spellCheck={false} />
          </Field>
          <Field label="Detect directory" hint="Its presence under home means the tool is installed. Defaults to the first segment of the skills directory.">
            <Input value={detectDir} onChange={(e) => setDetectDir(e.target.value)} placeholder=".my-agent" aria-label="Detect directory" className="mono" spellCheck={false} />
          </Field>
        </div>
        {id || name ? problems.length ? <ul className="list-disc pl-4 text-xs text-warn">{problems.map((p) => <li key={p}>{p}</li>)}</ul> : null : null}
        <ErrorNote error={add.error} />
      </DialogBody>
      <DialogFooter>
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" disabled={problems.length > 0} loading={add.isPending} onClick={submit}>
          Add agent
        </Button>
      </DialogFooter>
    </>
  );
}
