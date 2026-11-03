import * as React from "react";
import { AlertTriangle, BookOpen } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { useUi } from "@/lib/store";
import { cn, displayPath, formatTokens, pluralize } from "@/lib/utils";
import { Badge, Button, Checkbox, Tooltip } from "@/components/ui";
import { KIND_LABEL, KindIcon } from "@/components/app/common";
import { CANDIDATE_STATUS_META, defaultSelection, isImportable, toggleId, type CandidateGroup } from "./candidates";

export function CandidateStatusBadge({ candidate }: { candidate: T.Candidate }) {
  const meta = CANDIDATE_STATUS_META[candidate.status];
  const firstError = candidate.validation.findings.find((f) => f.severity === "error") ?? candidate.validation.findings[0];
  const tip = candidate.status === "invalid" && firstError ? `${firstError.message}${firstError.field ? ` (${firstError.field})` : ""}` : meta.hint;
  return (
    <Tooltip content={tip}>
      <span>
        <Badge tone={meta.tone}>
          {candidate.status === "invalid" ? <AlertTriangle /> : null}
          {meta.label}
        </Badge>
      </span>
    </Tooltip>
  );
}

/**
 * Selectable list of import candidates, shared by local scans and online previews.
 * Selection is controlled; invalid candidates cannot be selected.
 */
export function CandidateList({
  candidates,
  selected,
  onSelectedChange,
  groupOf,
  className,
}: {
  candidates: T.Candidate[];
  selected: string[];
  onSelectedChange: (ids: string[]) => void;
  /** Optional grouping (e.g. by agent target or scan root). */
  groupOf?: (c: T.Candidate) => CandidateGroup;
  className?: string;
}) {
  const importable = React.useMemo(() => candidates.filter(isImportable), [candidates]);
  const importableIds = React.useMemo(() => new Set(importable.map((c) => c.id)), [importable]);
  const selectedSet = React.useMemo(() => new Set(selected.filter((id) => importableIds.has(id))), [selected, importableIds]);
  const counts = React.useMemo(() => {
    const out: Record<T.CandidateStatus, number> = { new: 0, in_library: 0, modified: 0, managed: 0, invalid: 0 };
    for (const c of candidates) out[c.status] += 1;
    return out;
  }, [candidates]);

  const groups = React.useMemo(() => {
    if (!groupOf) return [{ key: "all", label: "", hint: undefined as string | undefined, candidates }];
    const map = new Map<string, CandidateGroup & { candidates: T.Candidate[] }>();
    for (const c of candidates) {
      const g = groupOf(c);
      const entry = map.get(g.key) ?? { ...g, candidates: [] };
      entry.candidates.push(c);
      map.set(g.key, entry);
    }
    return [...map.values()];
  }, [candidates, groupOf]);

  const toggle = (id: string) => {
    if (!importableIds.has(id)) return;
    onSelectedChange(toggleId([...selectedSet], id));
  };
  const setGroup = (ids: string[], on: boolean) => {
    const next = new Set(selectedSet);
    for (const id of ids) {
      if (!importableIds.has(id)) continue;
      if (on) next.add(id);
      else next.delete(id);
    }
    onSelectedChange([...next]);
  };

  return (
    <div className={cn("surface overflow-hidden", className)}>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 border-b bg-sunken/60 px-3 py-1.5 text-[11px]">
        <span className="font-medium">
          {selectedSet.size} of {importable.length} selected
        </span>
        <span className="text-subtle">
          {(["new", "modified", "in_library", "managed", "invalid"] as T.CandidateStatus[])
            .filter((s) => counts[s] > 0)
            .map((s) => `${counts[s]} ${CANDIDATE_STATUS_META[s].label.toLowerCase()}`)
            .join(" · ")}
        </span>
        <span className="ml-auto flex items-center gap-0.5">
          <Button size="sm" variant="ghost" onClick={() => onSelectedChange(defaultSelection(candidates))} disabled={counts.new === 0}>
            New only
          </Button>
          <Button size="sm" variant="ghost" onClick={() => onSelectedChange(importable.map((c) => c.id))} disabled={importable.length === 0}>
            All
          </Button>
          <Button size="sm" variant="ghost" onClick={() => onSelectedChange([])} disabled={selectedSet.size === 0}>
            None
          </Button>
        </span>
      </div>

      {candidates.length === 0 ? (
        <div className="px-3 py-8 text-center text-xs text-muted">No skills, agents or commands found.</div>
      ) : (
        groups.map((g) => {
          const groupImportable = g.candidates.filter(isImportable).map((c) => c.id);
          const groupSelected = groupImportable.filter((id) => selectedSet.has(id)).length;
          const state: boolean | "indeterminate" = groupImportable.length > 0 && groupSelected === groupImportable.length ? true : groupSelected > 0 ? "indeterminate" : false;
          return (
            <section key={g.key} aria-label={g.label || undefined}>
              {groupOf ? (
                <div className="flex items-center gap-2.5 border-b bg-sunken/30 px-3 py-1.5 text-xs">
                  <Checkbox
                    checked={state}
                    disabled={groupImportable.length === 0}
                    onCheckedChange={(v) => setGroup(groupImportable, v === true)}
                    aria-label={`Select all in ${g.label}`}
                  />
                  <span className="font-medium">{g.label}</span>
                  {g.hint ? <span className="mono truncate text-[11px] text-subtle">{g.hint}</span> : null}
                  <span className="ml-auto text-[11px] text-subtle">{pluralize(g.candidates.length, "candidate")}</span>
                </div>
              ) : null}
              {g.candidates.map((c) => (
                <CandidateRow key={c.id} candidate={c} selected={selectedSet.has(c.id)} onToggle={() => toggle(c.id)} />
              ))}
            </section>
          );
        })
      )}
    </div>
  );
}

function CandidateRow({ candidate: c, selected, onToggle }: { candidate: T.Candidate; selected: boolean; onToggle: () => void }) {
  const openItem = useUi((s) => s.openItem);
  const importable = isImportable(c);
  return (
    <div
      onClick={() => importable && onToggle()}
      className={cn(
        "flex items-center gap-3 border-b px-3 py-2 text-xs transition-colors last:border-b-0",
        importable ? "hover:bg-sunken/60" : "opacity-70",
        selected && "bg-accent-soft/35",
      )}
    >
      <span onClick={(e) => e.stopPropagation()} className="flex items-center">
        <Checkbox checked={selected} disabled={!importable} onCheckedChange={onToggle} aria-label={`Select ${c.name}`} />
      </span>
      <Tooltip content={KIND_LABEL[c.kind]}>
        <span className="flex size-6 shrink-0 items-center justify-center rounded-md bg-sunken text-muted">
          <KindIcon kind={c.kind} className="size-3.5" />
        </span>
      </Tooltip>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="truncate text-[13px] font-medium">{c.name || "(unnamed)"}</span>
          <CandidateStatusBadge candidate={c} />
          {c.libraryItemId ? (
            <Tooltip content="Open the library item">
              <button
                type="button"
                className="inline-flex items-center gap-1 text-[11px] text-info hover:underline"
                onClick={(e) => {
                  e.stopPropagation();
                  openItem(c.libraryItemId!);
                }}
              >
                <BookOpen className="size-3" /> in library
              </button>
            </Tooltip>
          ) : null}
        </div>
        <p className={cn("line-clamp-1 text-muted", !c.description && "italic text-subtle")}>{c.description || "No description"}</p>
        <p className="mono truncate text-[10.5px] text-subtle">{displayPath(c.path)}</p>
      </div>
      <Tooltip content="Estimated tokens of the instructions">
        <span className="num shrink-0 text-[11px] text-subtle">{formatTokens(c.tokenEstimate)} tok</span>
      </Tooltip>
    </div>
  );
}
