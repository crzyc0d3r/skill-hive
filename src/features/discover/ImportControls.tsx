import { AlertTriangle, Download } from "lucide-react";
import { cn, pluralize } from "@/lib/utils";
import { Button, Switch } from "@/components/ui";
import { TagsInput } from "./TagsInput";

/** Tags, overwrite toggle and the import button shared by local scans and online previews. */
export function ImportControls({
  selectedCount,
  existingCount = 0,
  tags,
  onTagsChange,
  overwrite,
  onOverwriteChange,
  onImport,
  loading,
  className,
}: {
  selectedCount: number;
  /** Selected candidates that already exist in the library (in_library / modified). */
  existingCount?: number;
  tags: string[];
  onTagsChange: (tags: string[]) => void;
  overwrite: boolean;
  onOverwriteChange: (v: boolean) => void;
  onImport: () => void;
  loading?: boolean;
  className?: string;
}) {
  return (
    <div className={cn("surface sticky bottom-0 z-10 flex flex-col gap-2 p-3 shadow-md", className)}>
      <div className="flex flex-wrap items-center gap-3">
        <div className="flex min-w-[240px] flex-1 items-center gap-2">
          <span className="text-xs text-muted">Tags</span>
          <TagsInput tags={tags} onChange={onTagsChange} placeholder="Add tags to the imported items…" className="flex-1" />
        </div>
        <label className="flex items-center gap-2 text-xs">
          <Switch checked={overwrite} onCheckedChange={onOverwriteChange} aria-label="Overwrite existing items" />
          <span>
            Overwrite existing
          </span>
        </label>
        <Button variant="primary" onClick={onImport} disabled={selectedCount === 0} loading={loading}>
          <Download /> Import {selectedCount ? pluralize(selectedCount, "item") : "selected"}
        </Button>
      </div>
      {existingCount > 0 && !overwrite ? (
        <p className="flex items-center gap-1.5 text-[11px] text-warn">
          <AlertTriangle className="size-3" />
          {pluralize(existingCount, "selected item")} already {existingCount === 1 ? "exists" : "exist"} in the library and will be skipped unless Overwrite is on.
        </p>
      ) : null}
    </div>
  );
}
