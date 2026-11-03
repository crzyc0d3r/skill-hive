import * as React from "react";
import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, ChevronDown, ChevronRight, FolderOpen, ScanSearch } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useSettings, useTargets } from "@/lib/api/hooks";
import { cn, displayPath, formatDuration, groupBy, pluralize } from "@/lib/utils";
import { Badge, Button, EmptyState, Skeleton, pickDirectory } from "@/components/ui";
import { ErrorNote, InlineCode } from "@/components/app/common";
import { CandidateList } from "./CandidateList";
import type { CandidateGroup } from "./candidates";
import { ImportControls } from "./ImportControls";
import { ImportResultSummary } from "./ImportResultSummary";
import { useImportSession } from "./useImportSession";

interface ScanRequest {
  seq: number;
  /** `null` scans the whole machine; otherwise the folders the user picked. */
  paths: string[] | null;
}

/** "On this machine": scan the detected agents' folders (or any folder) and import what is new. */
export function LocalTab({ autoScan }: { autoScan: boolean }) {
  const [request, setRequest] = React.useState<ScanRequest>(() => ({ seq: autoScan ? 1 : 0, paths: null }));
  const scan = useQuery({
    queryKey: ["scan", request.seq, request.paths],
    queryFn: () => (request.paths ? api.scanPaths(request.paths) : api.scanLocal([])),
    enabled: request.seq > 0,
    staleTime: Infinity,
    gcTime: 0,
    retry: false,
  });
  const { data: settings } = useSettings();
  const [picking, setPicking] = React.useState(false);

  const scanMachine = () => setRequest((r) => ({ seq: r.seq + 1, paths: null }));
  const pickFolder = async () => {
    setPicking(true);
    try {
      const dir = await pickDirectory("Choose a folder to scan for skills");
      if (dir) setRequest((r) => ({ seq: r.seq + 1, paths: [dir] }));
    } finally {
      setPicking(false);
    }
  };

  const scanning = scan.isFetching;
  const report = scan.data;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="primary" onClick={scanMachine} loading={scanning && !request.paths}>
          <ScanSearch /> Scan this machine
        </Button>
        <Button variant="secondary" onClick={pickFolder} loading={picking || (scanning && !!request.paths)}>
          <FolderOpen /> Pick a folder…
        </Button>
        <p className="text-xs text-muted">
          Looks in every detected agent's skills, agents and commands folders
          {settings?.extraScanDirs.length ? ` plus ${pluralize(settings.extraScanDirs.length, "extra folder")} from Settings` : ""}. Nothing is copied until you import.
        </p>
      </div>

      {scan.isError ? (
        <ErrorNote error={scan.error} />
      ) : scanning && !report ? (
        <ScanSkeleton />
      ) : report ? (
        <>
          <ReportSummary report={report} />
          {report.candidates.length === 0 ? (
            <EmptyState
              icon={ScanSearch}
              title="No skills found"
              description="None of the scanned folders contain a SKILL.md directory, a subagent or a command. Pick another folder or add a source from GitHub."
              action={
                <Button onClick={pickFolder}>
                  <FolderOpen /> Pick a folder…
                </Button>
              }
            />
          ) : (
            <ScanCandidates key={request.seq} report={report} />
          )}
        </>
      ) : (
        <EmptyState
          icon={ScanSearch}
          title="Collect what is already on this machine"
          description="Skill Hive finds skills, subagents and slash commands installed for Claude Code, Codex, Cursor and other agents, and lets you bring them into one library."
          action={
            <>
              <Button variant="primary" onClick={scanMachine}>
                <ScanSearch /> Scan this machine
              </Button>
              <Button onClick={pickFolder}>
                <FolderOpen /> Pick a folder…
              </Button>
            </>
          }
        />
      )}
    </div>
  );
}

function ScanSkeleton() {
  return (
    <div className="flex flex-col gap-3" aria-busy="true" aria-label="Scanning">
      <Skeleton className="h-10 w-full" />
      <div className="surface flex flex-col gap-2 p-3">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-10 w-full" />
        ))}
      </div>
    </div>
  );
}

function ReportSummary({ report }: { report: T.ScanReport }) {
  const [open, setOpen] = React.useState(report.errors.length > 0);
  return (
    <div className="surface px-3 py-2 text-xs">
      <div className="flex flex-wrap items-center gap-2">
        <button type="button" onClick={() => setOpen((o) => !o)} className="flex items-center gap-1 font-medium hover:text-fg" aria-expanded={open}>
          {open ? <ChevronDown className="size-3.5 text-muted" /> : <ChevronRight className="size-3.5 text-muted" />}
          {pluralize(report.candidates.length, "candidate")}
        </button>
        <span className="text-muted">
          in {pluralize(report.rootsScanned.length, "folder")} · {formatDuration(report.durationMs)}
        </span>
        {report.errors.length ? (
          <Badge tone="warn">
            <AlertTriangle /> {pluralize(report.errors.length, "error")}
          </Badge>
        ) : null}
      </div>
      {open ? (
        <div className="mt-2 flex flex-col gap-2 border-t pt-2">
          <div className="flex flex-wrap items-center gap-1">
            <span className="mr-1 text-subtle">Scanned</span>
            {report.rootsScanned.map((r) => (
              <InlineCode key={r}>{displayPath(r)}</InlineCode>
            ))}
          </div>
          {report.errors.length ? (
            <ul className="flex flex-col gap-1">
              {report.errors.map((e, i) => (
                <li key={i} className="flex items-start gap-1.5 text-warn">
                  <AlertTriangle className="mt-0.5 size-3 shrink-0" />
                  <span className="mono selectable break-all text-[11px]">{e}</span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

/** Keyed per scan so the selection resets with every new report. */
function ScanCandidates({ report }: { report: T.ScanReport }) {
  const session = useImportSession(report.candidates);
  const { data: targets } = useTargets();

  const groupOf = React.useCallback(
    (c: T.Candidate): CandidateGroup => {
      if (c.source.targetId) {
        const t = targets?.find((x) => x.id === c.source.targetId);
        return { key: `target:${c.source.targetId}`, label: t?.name ?? c.source.targetId };
      }
      const root = report.rootsScanned.find((r) => c.path.startsWith(r)) ?? c.path.slice(0, c.path.lastIndexOf("/"));
      return { key: `root:${root}`, label: displayPath(root) };
    },
    [targets, report.rootsScanned],
  );

  const importMut = useApiMutation(
    async (chosen: T.Candidate[]) => {
      // One request per source kind + agent target; the first candidate's source stands for the group.
      const groups = groupBy(chosen, (c) => `${c.source.kind}|${c.source.targetId ?? ""}`);
      const merged: T.ImportResult = { imported: [], skipped: [] };
      for (const group of Object.values(groups)) {
        const res = await api.importItems({ paths: group.map((c) => c.path), source: group[0].source, tags: session.tags, overwrite: session.overwrite });
        merged.imported.push(...res.imported);
        merged.skipped.push(...res.skipped);
      }
      return merged;
    },
    [keys.itemsAll, keys.stats, keys.dashboard, keys.targets, keys.deployments],
    (res, chosen) => session.finish(res, chosen),
  );

  return (
    <div className={cn("flex flex-col gap-3")}>
      <CandidateList candidates={session.candidates} selected={session.selected} onSelectedChange={session.setSelected} groupOf={groupOf} />
      <ImportControls
        selectedCount={session.selectedCandidates.length}
        existingCount={session.existingCount}
        tags={session.tags}
        onTagsChange={session.setTags}
        overwrite={session.overwrite}
        onOverwriteChange={session.setOverwrite}
        onImport={() => importMut.mutate(session.selectedCandidates)}
        loading={importMut.isPending}
      />
      <ErrorNote error={importMut.error} />
      {session.result ? <ImportResultSummary result={session.result} onDismiss={session.clearResult} /> : null}
    </div>
  );
}
