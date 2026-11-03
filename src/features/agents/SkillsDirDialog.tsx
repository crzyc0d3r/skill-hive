import * as React from "react";
import { toast } from "sonner";
import { FolderOpen } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation } from "@/lib/api/hooks";
import { displayPath } from "@/lib/utils";
import { Button, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input, pickDirectory } from "@/components/ui";
import { ErrorNote } from "@/components/app/common";

/** Point a target at a different user-level skills folder (or reset it to the default). */
export function SkillsDirDialog({ open, onOpenChange, spec, currentPath }: { open: boolean; onOpenChange: (o: boolean) => void; spec: T.TargetSpec; currentPath: string }) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Change skills folder for {spec.name}</DialogTitle>
          <DialogDescription>Where {spec.name} reads user-level skills. Change it if you moved the folder or use a non-standard setup; existing deployments are not moved.</DialogDescription>
        </DialogHeader>
        <SkillsDirForm key={currentPath} spec={spec} currentPath={currentPath} onClose={() => onOpenChange(false)} />
      </DialogContent>
    </Dialog>
  );
}

function SkillsDirForm({ spec, currentPath, onClose }: { spec: T.TargetSpec; currentPath: string; onClose: () => void }) {
  const [dir, setDir] = React.useState(currentPath);
  const save = useApiMutation(
    (d: string | null) => api.setTargetSkillsDir(spec.id, d),
    [keys.targets, keys.deployments, keys.dashboard],
    (_r, d) => {
      toast.success(d ? "Skills folder updated" : "Skills folder reset to the default");
      onClose();
    },
  );
  const trimmed = dir.trim();
  return (
    <>
      <DialogBody className="flex flex-col gap-4">
        <Field label="Skills folder" hint={`Default: ~/${spec.userSkillsDir}`}>
          <div className="flex gap-2">
            <Input value={dir} onChange={(e) => setDir(e.target.value)} className="mono flex-1" spellCheck={false} aria-label="Skills folder path" />
            <Button
              variant="secondary"
              onClick={async () => {
                const picked = await pickDirectory(`Choose the skills folder for ${spec.name}`);
                if (picked) setDir(picked);
              }}
            >
              <FolderOpen /> Browse
            </Button>
          </div>
        </Field>
        <p className="text-xs text-muted">
          Currently <span className="mono selectable">{displayPath(currentPath)}</span>
        </p>
        <ErrorNote error={save.error} />
      </DialogBody>
      <DialogFooter>
        <Button variant="ghost" className="mr-auto" onClick={() => save.mutate(null)} loading={save.isPending && save.variables === null}>
          Reset to default
        </Button>
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" disabled={!trimmed || trimmed === currentPath} loading={save.isPending && save.variables !== null} onClick={() => save.mutate(trimmed)}>
          Save
        </Button>
      </DialogFooter>
    </>
  );
}
