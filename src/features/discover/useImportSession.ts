import * as React from "react";
import { toast } from "sonner";
import type * as T from "@/lib/api/bindings";
import { pluralize } from "@/lib/utils";
import { defaultSelection, isImportable } from "./candidates";

/**
 * Selection, tags, overwrite flag and the last result for one batch of candidates
 * (a scan report or a source preview). After an import the imported candidates are
 * marked as "in library" and dropped from the selection, so the list stays truthful.
 */
export function useImportSession(initial: T.Candidate[]) {
  const [candidates, setCandidates] = React.useState(initial);
  const [selected, setSelected] = React.useState(() => defaultSelection(initial));
  const [tags, setTags] = React.useState<string[]>([]);
  const [overwrite, setOverwrite] = React.useState(false);
  const [result, setResult] = React.useState<T.ImportResult | null>(null);

  const selectedCandidates = React.useMemo(() => candidates.filter((c) => selected.includes(c.id) && isImportable(c)), [candidates, selected]);
  const existingCount = selectedCandidates.filter((c) => c.status === "in_library" || c.status === "modified").length;

  /** Apply an import result to the local list and notify. */
  const finish = React.useCallback((res: T.ImportResult, attempted: T.Candidate[]) => {
    const skippedPaths = new Set(res.skipped.map((s) => s.path));
    const importedByName = new Map(res.imported.map((i) => [i.name, i]));
    const done = new Set(attempted.filter((c) => !skippedPaths.has(c.path)).map((c) => c.id));
    setCandidates((prev) =>
      prev.map((c) => (done.has(c.id) ? { ...c, status: "in_library", libraryItemId: importedByName.get(c.name)?.id ?? c.libraryItemId ?? null } : c)),
    );
    setSelected((prev) => prev.filter((id) => !done.has(id)));
    setResult(res);
    if (res.imported.length) {
      toast.success(`Imported ${pluralize(res.imported.length, "item")}`, {
        description: res.skipped.length ? `${pluralize(res.skipped.length, "item")} skipped` : res.imported.map((i) => i.name).join(", "),
      });
    } else if (res.skipped.length) {
      toast.warning("Nothing imported", { description: res.skipped[0].reason });
    } else {
      toast("Nothing to import");
    }
  }, []);

  return {
    candidates,
    selected,
    setSelected,
    selectedCandidates,
    existingCount,
    tags,
    setTags,
    overwrite,
    setOverwrite,
    result,
    clearResult: () => setResult(null),
    finish,
  };
}
