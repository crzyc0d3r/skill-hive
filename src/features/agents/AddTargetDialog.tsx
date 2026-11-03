import * as React from "react";
import { toast } from "sonner";
import { Plus } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation } from "@/lib/api/hooks";
import { Button, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input, SimpleSelect } from "@/components/ui";
import { ErrorNote } from "@/components/app/common";

const ID_RE = /^[a-z0-9]+(-[a-z0-9]+)*$/;
const slugify = (s: string) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

/** Describe an agent tool Skill Hive does not know about, so it can deploy there and track its budget. */
export function AddTargetDialog({
  open,
  onOpenChange,
  onCreated,
  existingIds = [],
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  onCreated?: (spec: T.TargetSpec) => void;
  /** Ids already in use; a colliding id is refused before the backend sees it. */
  existingIds?: string[];
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add a custom agent</DialogTitle>
          <DialogDescription>Tell Skill Hive where another agent tool reads its skills. Paths are relative to your home folder or to a project root.</DialogDescription>
        </DialogHeader>
        <AddTargetForm onCreated={onCreated} onCancel={() => onOpenChange(false)} existingIds={existingIds} />
      </DialogContent>
    </Dialog>
  );
}

function AddTargetForm({ onCreated, onCancel, existingIds }: { onCreated?: (spec: T.TargetSpec) => void; onCancel: () => void; existingIds: string[] }) {
  const [name, setName] = React.useState("");
  const [idOverride, setIdOverride] = React.useState<string | null>(null);
  const id = idOverride ?? slugify(name);
  const [category, setCategory] = React.useState<T.TargetCategory>("coding");
  const [userDirOverride, setUserDirOverride] = React.useState<string | null>(null);
  const userSkillsDir = userDirOverride ?? (id ? `.${id}/skills` : "");
  const [projectDirOverride, setProjectDirOverride] = React.useState<string | null>(null);
  const projectSkillsDir = projectDirOverride ?? (id ? `.${id}/skills` : "");
  const [detectOverride, setDetectOverride] = React.useState<string | null>(null);
  const detectDir = detectOverride ?? (id ? `.${id}` : "");
  const [budget, setBudget] = React.useState("15000");
  const [docsUrl, setDocsUrl] = React.useState("");

  const create = useApiMutation((spec: T.TargetSpec) => api.addCustomTarget(spec), [keys.targets, keys.installedTargets, keys.dashboard], (spec) => {
    toast.success(`Added ${spec.name}`);
    onCreated?.(spec);
  });

  const idTaken = existingIds.includes(id);
  const idOk = ID_RE.test(id) && !idTaken;
  const budgetNum = budget.trim() ? Number(budget) : null;
  const budgetOk = budgetNum === null || (Number.isFinite(budgetNum) && budgetNum > 0);
  const valid = !!name.trim() && idOk && !!userSkillsDir.trim() && !!detectDir.trim() && budgetOk;

  const submit = () =>
    create.mutate({
      id,
      name: name.trim(),
      category,
      userSkillsDir: userSkillsDir.trim(),
      projectSkillsDir: projectSkillsDir.trim() || null,
      userAgentsDir: null,
      projectAgentsDir: null,
      userCommandsDir: null,
      projectCommandsDir: null,
      detectDir: detectDir.trim(),
      descriptionBudgetChars: budgetNum,
      docsUrl: docsUrl.trim() || null,
      custom: true,
    });

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        if (valid) submit();
      }}
      className="contents"
    >
      <DialogBody className="flex flex-col gap-4">
        <Field label="Name">
          <Input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Aider" />
        </Field>
        <div className="grid grid-cols-2 gap-4">
          <Field label="Id" hint={id && idTaken ? "That id is already used." : id && !idOk ? "Lowercase letters, digits and single hyphens" : "Used in paths and commands"}>
            <Input value={id} onChange={(e) => setIdOverride(e.target.value)} placeholder="aider" className="mono" spellCheck={false} />
          </Field>
          <Field label="Category">
            <SimpleSelect
              value={category}
              onChange={(v) => setCategory(v as T.TargetCategory)}
              options={[
                { value: "coding", label: "Coding agent" },
                { value: "assistant", label: "Assistant" },
              ]}
            />
          </Field>
        </div>
        <Field label="User skills folder" hint="Relative to your home folder. Skills deployed at user scope go here.">
          <Input value={userSkillsDir} onChange={(e) => setUserDirOverride(e.target.value)} className="mono" spellCheck={false} />
        </Field>
        <Field label="Project skills folder" hint="Relative to a project root. Leave empty if the agent has no project-level skills.">
          <Input value={projectSkillsDir} onChange={(e) => setProjectDirOverride(e.target.value)} className="mono" spellCheck={false} />
        </Field>
        <div className="grid grid-cols-2 gap-4">
          <Field label="Detect folder" hint="Relative to home; its presence means the tool is installed.">
            <Input value={detectDir} onChange={(e) => setDetectOverride(e.target.value)} className="mono" spellCheck={false} />
          </Field>
          <Field label="Description budget" hint="Characters of skill descriptions the agent loads before truncating. Claude Code: ~15 000.">
            <Input type="number" min={1000} step={500} value={budget} onChange={(e) => setBudget(e.target.value)} className="num" />
          </Field>
        </div>
        <Field label="Docs URL (optional)">
          <Input value={docsUrl} onChange={(e) => setDocsUrl(e.target.value)} placeholder="https://…" spellCheck={false} />
        </Field>
        <ErrorNote error={create.error} />
      </DialogBody>
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" disabled={!valid} loading={create.isPending}>
          <Plus /> Add agent
        </Button>
      </DialogFooter>
    </form>
  );
}
