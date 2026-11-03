import * as React from "react";
import { toast } from "sonner";
import { api } from "@/lib/api/client";
import { keys, useApiMutation } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { Button, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input, Textarea } from "@/components/ui";
import { ErrorNote } from "@/components/app/common";

const TEMPLATE = `# Overview

Explain what this skill does in two or three sentences.

## When to use

- Situation one
- Situation two

## Steps

1. First step
2. Second step

## Notes

Keep this file under 500 lines; move detail into reference files next to it.
`;

export function CreateSkillDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const openItem = useUi((s) => s.openItem);
  const [name, setName] = React.useState("");
  const [description, setDescription] = React.useState("");
  const [tags, setTags] = React.useState("");
  const [body, setBody] = React.useState(TEMPLATE);

  const create = useApiMutation(
    () => api.createSkill(name.trim(), description.trim(), body, tags.split(",").map((t) => t.trim()).filter(Boolean)),
    [keys.itemsAll, keys.stats, keys.dashboard],
    (item) => {
      toast.success(`Created ${item.name}`);
      onOpenChange(false);
      openItem(item.id);
      setName("");
      setDescription("");
      setTags("");
      setBody(TEMPLATE);
    },
  );

  const nameOk = /^[a-z0-9]+(-[a-z0-9]+)*$/.test(name) && name.length <= 64;
  const descOk = description.trim().length >= 20 && description.length <= 1024;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>New skill</DialogTitle>
          <DialogDescription>Creates a directory with a SKILL.md in your library. You can deploy it to any agent afterwards.</DialogDescription>
        </DialogHeader>
        <DialogBody className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-4">
            <Field label="Name" hint={name && !nameOk ? "Lowercase letters, digits and single hyphens, up to 64 characters" : "Also the directory name, e.g. pdf-tools"}>
              <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="pdf-tools" autoFocus className="mono" />
            </Field>
            <Field label="Tags" hint="Comma separated">
              <Input value={tags} onChange={(e) => setTags(e.target.value)} placeholder="documents, pdf" />
            </Field>
          </div>
          <Field
            label={`Description (${description.length}/1024)`}
            hint="Agents decide whether to load the skill from this sentence alone. Say what it does and when to use it."
          >
            <Textarea value={description} onChange={(e) => setDescription(e.target.value)} rows={3} placeholder="Extract text and tables from PDF files. Use when the user shares a PDF or asks to read a document." />
          </Field>
          <Field label="Instructions (markdown)">
            <Textarea value={body} onChange={(e) => setBody(e.target.value)} rows={12} className="mono text-xs" />
          </Field>
          <ErrorNote error={create.error} />
        </DialogBody>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" disabled={!nameOk || !descOk} loading={create.isPending} onClick={() => create.mutate(undefined)}>
            Create skill
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
