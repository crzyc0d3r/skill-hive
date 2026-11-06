import * as React from "react";
import { useBlocker, useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { FilePlus2, FolderOpen, History, Monitor, RotateCcw, Save, Trash2 } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useInstruction, useInstructionBackups, useInstructionFiles, useProjects } from "@/lib/api/hooks";
import { cn, formatBytes, formatDate, formatTokens, modKey, pluralize } from "@/lib/utils";
import {
  Badge,
  Button,
  ConfirmDialog,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
  EmptyState,
  Input,
  Markdown,
  Skeleton,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  Textarea,
  revealPath,
} from "@/components/ui";
import { ErrorNote, InlineCode, Page, PageHeader } from "@/components/app/common";
import { ScopePicker } from "@/features/agents/ScopePicker";

/** Instructions: the CLAUDE.md, AGENTS.md, GEMINI.md and rules files each agent loads, per scope, with an editor and backups. */
export function InstructionsPage() {
  const navigate = useNavigate();
  const { data: projects } = useProjects();
  const [scope, setScope] = React.useState<T.Scope>("user");
  const [projectOverride, setProjectOverride] = React.useState<string | null>(null);
  const projectPath = projectOverride ?? projects?.[0]?.path ?? null;
  const effectiveProject = scope === "project" ? projectPath : null;
  const files = useInstructionFiles(scope, effectiveProject);
  const [selectedPath, setSelectedPath] = React.useState<string | null>(null);
  /** A rules-folder member chosen through "New rule…" that is not on disk yet. */
  const [pending, setPending] = React.useState<T.InstructionFile | null>(null);
  const dirtyRef = React.useRef(false);
  const [isDirty, setIsDirty] = React.useState(false);
  const onDirtyChange = React.useCallback((dirty: boolean) => {
    dirtyRef.current = dirty;
    setIsDirty(dirty);
  }, []);
  /** Action waiting on the "Discard changes?" dialog. */
  const [discard, setDiscard] = React.useState<(() => void) | null>(null);
  // Leaving the page (sidebar, shortcuts, command palette) asks the same question.
  const blocker = useBlocker(() => dirtyRef.current);
  const leaving = blocker.state === "blocked";

  const guard = (fn: () => void) => {
    if (dirtyRef.current) setDiscard(() => fn);
    else fn();
  };
  const select = (path: string | null, draft: T.InstructionFile | null = null) => {
    if (path === selectedPath && !draft) return;
    guard(() => {
      setSelectedPath(path);
      setPending(draft);
      dirtyRef.current = false;
    });
  };
  const changeScope = (next: T.Scope, project?: string) =>
    guard(() => {
      setScope(next);
      if (project !== undefined) setProjectOverride(project);
      setSelectedPath(null);
      setPending(null);
      dirtyRef.current = false;
    });

  const entries = files.data ?? [];
  // A pending rule that has been written is listed like any other member; once deleted it is gone.
  // A member removed behind the editor's back (by the agent, or the CLI) stays open while the
  // draft is unsaved, as a missing file the Create button writes back.
  const listed = entries.find((f) => f.path === selectedPath);
  const [lastListed, setLastListed] = React.useState<T.InstructionFile | null>(null);
  if (listed && listed !== lastListed) setLastListed(listed);
  const vanished = lastListed?.path === selectedPath && isDirty ? { ...lastListed, exists: false } : null;
  const selected = listed ?? (pending && pending.path === selectedPath ? pending : null) ?? vanished;
  const onDeleted = React.useCallback((path: string) => setPending((p) => (p?.path === path ? null : p)), []);
  const needsProject = scope === "project" && !projectPath;

  return (
    <Page>
      <PageHeader
        title="Instructions"
        description="The files each agent loads into every session: CLAUDE.md, AGENTS.md, GEMINI.md and rules folders. Edit them here; a backup of every version you overwrite is kept."
      >
        <ScopePicker scope={scope} onScopeChange={(s) => changeScope(s)} projectPath={projectPath} onProjectChange={(p) => changeScope("project", p)} projectDisabled={false} />
      </PageHeader>
      <div className="flex min-h-0 flex-1 border-t">
        {needsProject ? (
          <div className="flex-1 px-6 py-6">
            <EmptyState compact icon={FolderOpen} title="No project selected" description="Choose a project to see its files." />
          </div>
        ) : files.isError ? (
          <div className="flex flex-1 flex-col gap-3 px-6 py-4">
            <ErrorNote error={files.error} />
            <div>
              <Button size="sm" onClick={() => void files.refetch()}>
                Retry
              </Button>
            </div>
          </div>
        ) : files.data && files.data.length === 0 ? (
          <div className="flex-1 px-6 py-6">
            <EmptyState
              icon={Monitor}
              title="No agent tools detected"
              description="Instruction files are listed for the agents installed on this machine. Install one, or describe a custom agent under Agents."
              action={<Button onClick={() => navigate("/agents")}>Open Agents</Button>}
            />
          </div>
        ) : (
          <>
            <FileRail entries={entries} pending={pending} loading={files.isLoading} selectedPath={selectedPath} onSelect={(p) => select(p)} onNewRule={(dir, name) => select(`${dir.path}/${name}`, newMember(dir, name))} />
            <div className="flex min-w-0 flex-1 flex-col">
              {selected ? (
                selected.kind === "directory" ? (
                  <FolderPanel key={selected.path} entry={selected} members={entries.filter((e) => e.parent === selected.path)} onNewRule={(name) => select(`${selected.path}/${name}`, newMember(selected, name))} />
                ) : (
                  <InstructionEditor key={selected.path} entry={selected} project={effectiveProject} onDirtyChange={onDirtyChange} onDeleted={onDeleted} />
                )
              ) : files.isLoading ? (
                <div className="flex flex-col gap-3 px-5 py-4" aria-busy="true">
                  <Skeleton className="h-6 w-1/3" />
                  <Skeleton className="h-48" />
                </div>
              ) : (
                <p className="flex flex-1 items-center justify-center px-6 text-center text-xs text-muted">Pick a file on the left to read or edit it. Missing files can be created in place.</p>
              )}
            </div>
          </>
        )}
      </div>
      <ConfirmDialog
        open={!!discard || leaving}
        onOpenChange={(o) => {
          if (o) return;
          setDiscard(null);
          if (leaving) blocker.reset();
        }}
        title="Discard changes?"
        description="The edits to this file have not been saved."
        confirmLabel="Discard"
        destructive
        onConfirm={() => {
          if (leaving) {
            dirtyRef.current = false;
            blocker.proceed();
            return;
          }
          discard?.();
          setDiscard(null);
        }}
      />
    </Page>
  );
}

/** A not-yet-existing member of a rules folder, described like the listing would once it is written. */
function newMember(dir: T.InstructionFile, name: string): T.InstructionFile {
  return { ...dir, path: `${dir.path}/${name}`, display: `${dir.display}/${name}`, kind: "file", pattern: null, parent: dir.path, exists: false, sizeBytes: 0, modifiedAt: null, tokens: 0, lines: 0, note: null };
}

/** What a missing file starts with, by what it is to the agent. */
function starterFor(entry: T.InstructionFile): string {
  const name = entry.display.split("/").pop() ?? "";
  if (name.endsWith(".mdc")) return "---\ndescription: \nalwaysApply: false\n---\n\n";
  if (name.endsWith(".instructions.md")) return '---\napplyTo: "**"\n---\n\n';
  if (entry.parent) return `# ${name.replace(/\.[^.]+$/, "")}\n\n`;
  if (entry.scope === "user") return "# Preferences\n\n";
  return "# Project notes\n\n## Commands\n\n## Conventions\n";
}

const isMarkdown = (display: string) => /\.(md|mdc)$/i.test(display);

function FileRail({
  entries,
  pending,
  loading,
  selectedPath,
  onSelect,
  onNewRule,
}: {
  entries: T.InstructionFile[];
  pending: T.InstructionFile | null;
  loading: boolean;
  selectedPath: string | null;
  onSelect: (path: string) => void;
  onNewRule: (dir: T.InstructionFile, name: string) => void;
}) {
  const [newRuleFor, setNewRuleFor] = React.useState<string | null>(null);
  // A pending new rule sits under its folder, after the members on disk.
  const list = entries.slice();
  if (pending && !list.some((e) => e.path === pending.path)) {
    let at = -1;
    list.forEach((e, i) => {
      if (e.path === pending.parent || e.parent === pending.parent) at = i;
    });
    if (at >= 0) list.splice(at + 1, 0, pending);
  }
  // Grouped by the agents that read them, in order of first appearance.
  const groups: { key: string; entries: T.InstructionFile[] }[] = [];
  for (const e of list) {
    const key = e.targetNames.join(" · ");
    const g = groups.find((x) => x.key === key);
    if (g) g.entries.push(e);
    else groups.push({ key, entries: [e] });
  }

  return (
    <aside className="flex w-72 shrink-0 flex-col border-r" aria-label="Instruction files">
      <div className="scroll-y flex-1 p-2">
        {loading ? (
          <div className="flex flex-col gap-1.5 px-1" aria-busy="true">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} className="h-12" />
            ))}
          </div>
        ) : (
          groups.map((g) => (
            <div key={g.key} className="mb-3">
              <div className="truncate px-2 pb-1 pt-1 text-[11px] font-medium uppercase tracking-wide text-subtle" title={g.key}>
                {g.key}
              </div>
              <div className="flex flex-col gap-0.5">
                {g.entries.map((e) => (
                  <RailEntry
                    key={e.path}
                    entry={e}
                    active={e.path === selectedPath}
                    onClick={() => onSelect(e.path)}
                    newRuleOpen={newRuleFor === e.path}
                    onToggleNewRule={(open) => setNewRuleFor(open ? e.path : null)}
                    onNewRule={(name) => {
                      setNewRuleFor(null);
                      onNewRule(e, name);
                    }}
                  />
                ))}
              </div>
            </div>
          ))
        )}
      </div>
    </aside>
  );
}

function RailEntry({
  entry,
  active,
  onClick,
  newRuleOpen,
  onToggleNewRule,
  onNewRule,
}: {
  entry: T.InstructionFile;
  active: boolean;
  onClick: () => void;
  newRuleOpen: boolean;
  onToggleNewRule: (open: boolean) => void;
  onNewRule: (name: string) => void;
}) {
  const isDir = entry.kind === "directory";
  return (
    <div className={cn(entry.parent && "ml-4")}>
      <button
        type="button"
        onClick={onClick}
        aria-current={active ? "page" : undefined}
        className={cn("flex w-full flex-col gap-0.5 rounded-lg px-2.5 py-1.5 text-left transition-colors hover:bg-sunken", active && "bg-elevated shadow-sm hairline hover:bg-elevated")}
      >
        <span className="mono w-full truncate text-xs" title={entry.display}>
          {entry.display}
        </span>
        <span className="flex w-full items-center gap-2 text-[11px] text-muted">
          <span className="min-w-0 flex-1 truncate">{entry.label}</span>
          <RailStatus entry={entry} />
        </span>
        {entry.note ? <span className="text-[11px] text-subtle">{entry.note}</span> : null}
      </button>
      {isDir ? (
        <div className="px-2.5 pb-1 pt-0.5">
          {newRuleOpen ? (
            <NewRuleInput dir={entry} onSubmit={onNewRule} onCancel={() => onToggleNewRule(false)} />
          ) : (
            <Button size="sm" variant="ghost" aria-label={`New rule in ${entry.display}`} onClick={() => onToggleNewRule(true)}>
              <FilePlus2 /> New rule…
            </Button>
          )}
        </div>
      ) : null}
    </div>
  );
}

function RailStatus({ entry }: { entry: T.InstructionFile }) {
  if (!entry.exists) return <Badge>Missing</Badge>;
  if (entry.kind === "directory") return <span className="shrink-0 text-subtle">Folder</span>;
  return (
    <span className="num shrink-0 text-subtle">
      {pluralize(entry.lines, "line")} · ~{formatTokens(entry.tokens)} tokens
    </span>
  );
}

/** Asks for the name of a new rules-folder member; the folder's suffix is required. */
function NewRuleInput({ dir, onSubmit, onCancel }: { dir: T.InstructionFile; onSubmit: (name: string) => void; onCancel: () => void }) {
  const pattern = dir.pattern ?? ".md";
  const [name, setName] = React.useState("");
  const trimmed = name.trim();
  const error = !trimmed
    ? null
    : /[/\\]/.test(trimmed)
      ? "No slashes: the file goes directly in the folder"
      : trimmed.startsWith(".")
        ? "The name cannot start with a dot"
        : !trimmed.endsWith(pattern) || trimmed.length <= pattern.length
          ? `The name must end with ${pattern}`
          : null;
  const ok = !!trimmed && !error;
  return (
    <form
      className="flex flex-col gap-1"
      onSubmit={(e) => {
        e.preventDefault();
        if (ok) onSubmit(trimmed);
      }}
    >
      <Input
        autoFocus
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder={`name${pattern}`}
        aria-label={`New rule file name in ${dir.display}`}
        aria-invalid={!!error}
        spellCheck={false}
        className="mono h-7 text-xs"
        onKeyDown={(e) => {
          if (e.key === "Escape") onCancel();
        }}
      />
      <div className="flex items-center justify-between gap-2 text-[11px]">
        <span className={cn("min-w-0 break-words", error ? "text-danger" : "text-subtle")}>{error ?? `Ends with ${pattern}`}</span>
        <span className="flex shrink-0 gap-1">
          <Button size="sm" variant="ghost" type="button" onClick={onCancel}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" type="submit" disabled={!ok}>
            Add
          </Button>
        </span>
      </div>
    </form>
  );
}

/** Path, readers, size and modification time of an entry, with Reveal when it is on disk. */
function EntryHeader({ file }: { file: T.InstructionFile }) {
  return (
    <div className="flex flex-wrap items-start justify-between gap-3 border-b px-5 py-3">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <h2 className="mono selectable text-sm font-semibold" title={file.path}>
            {file.display}
          </h2>
          {file.targetNames.map((n) => (
            <Badge key={n} tone="outline">
              {n}
            </Badge>
          ))}
          {!file.exists ? <Badge>Missing</Badge> : null}
        </div>
        <div className="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-[11px] text-muted">
          <span>{file.label}</span>
          {file.exists && file.kind === "file" ? (
            <span className="num">
              · {formatBytes(file.sizeBytes)} · {pluralize(file.lines, "line")} · ~{formatTokens(file.tokens)} tokens
            </span>
          ) : null}
          {file.modifiedAt ? <span>· Modified {formatDate(file.modifiedAt)}</span> : null}
        </div>
        {file.note ? <p className="mt-1 text-[11px] text-subtle">{file.note}</p> : null}
      </div>
      {file.exists ? (
        <Button variant="ghost" size="sm" className="shrink-0" onClick={() => void revealPath(file.path)}>
          <FolderOpen /> Reveal
        </Button>
      ) : null}
    </div>
  );
}

function FolderPanel({ entry, members, onNewRule }: { entry: T.InstructionFile; members: T.InstructionFile[]; onNewRule: (name: string) => void }) {
  const [adding, setAdding] = React.useState(false);
  const pattern = entry.pattern ?? ".md";
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <EntryHeader file={entry} />
      <div className="flex flex-col gap-3 px-5 py-4">
        <p className="text-xs text-muted">
          A rules folder: every <InlineCode>{pattern}</InlineCode> file directly inside it is loaded.{" "}
          {entry.exists ? (members.length ? `${pluralize(members.length, "rule")} on disk; pick one on the left to edit it.` : "No rules in it yet.") : "The folder is created with its first rule."}
        </p>
        {adding ? (
          <div className="max-w-sm">
            <NewRuleInput dir={entry} onSubmit={onNewRule} onCancel={() => setAdding(false)} />
          </div>
        ) : (
          <div>
            <Button size="sm" variant="primary" onClick={() => setAdding(true)}>
              <FilePlus2 /> New rule…
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}

/** Keyed by path by the page, so the draft resets when another file is picked. */
function InstructionEditor({
  entry,
  project,
  onDirtyChange,
  onDeleted,
}: {
  entry: T.InstructionFile;
  project: string | null;
  onDirtyChange: (dirty: boolean) => void;
  onDeleted: (path: string) => void;
}) {
  const qc = useQueryClient();
  const doc = useInstruction(entry.path, project);
  // The read answers for the file as it is now; the listing can lag behind a save or delete.
  const file = doc.data?.file ?? entry;
  const exists = file.exists;
  const base = exists ? doc.data?.text : starterFor(entry);
  const [draft, setDraft] = React.useState<string | null>(null);
  const [tab, setTab] = React.useState("edit");
  const [confirmDelete, setConfirmDelete] = React.useState(false);
  const [backupsOpen, setBackupsOpen] = React.useState(false);
  const value = draft ?? base ?? "";
  const dirty = draft !== null && draft !== base;
  const canSave = exists ? dirty : base !== undefined;

  React.useEffect(() => {
    onDirtyChange(dirty);
    return () => onDirtyChange(false);
  }, [dirty, onDirtyChange]);

  const write = useApiMutation((text: string) => api.writeInstruction(entry.path, text, project), [["instructions"]], (saved) => {
    qc.setQueryData(keys.instruction(entry.path), saved);
    setDraft(null);
    toast.success(exists ? `Saved ${entry.display}` : `Created ${entry.display}`);
  });
  const remove = useApiMutation(() => api.deleteInstruction(entry.path, project), [["instructions"]], (deleted) => {
    qc.setQueryData(keys.instruction(entry.path), { file: deleted, text: "" } satisfies T.InstructionDocument);
    setDraft(null);
    setConfirmDelete(false);
    toast.success(`Deleted ${entry.display}`);
    onDeleted(entry.path);
  });
  const backups = useInstructionBackups(entry.path, backupsOpen);
  const loadBackup = useApiMutation((b: T.InstructionBackup) => api.readInstruction(b.path), [], (d, b) => {
    setDraft(d.text);
    setTab("edit");
    toast(`Loaded backup from ${formatDate(b.savedAt)}; save to restore`);
  });

  const save = () => {
    if (canSave && !write.isPending) write.mutate(value);
  };
  // Ctrl/Cmd+S saves wherever focus sits while a file is open (the preview, the footer, the
  // page body), and never reaches the WebView's own save dialog.
  const saveRef = React.useRef(save);
  React.useEffect(() => {
    saveRef.current = save;
  });
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === "s") {
        e.preventDefault();
        saveRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (doc.isError && !doc.data) {
    return (
      <div className="flex min-h-0 flex-1 flex-col">
        <EntryHeader file={entry} />
        <div className="flex flex-col gap-3 px-5 py-4">
          <ErrorNote error={doc.error} />
          <div>
            <Button size="sm" onClick={() => void doc.refetch()}>
              Retry
            </Button>
          </div>
        </div>
      </div>
    );
  }
  if (exists && doc.isLoading) {
    return (
      <div className="flex min-h-0 flex-1 flex-col" aria-busy="true">
        <EntryHeader file={entry} />
        <div className="flex flex-col gap-3 px-5 py-4">
          <Skeleton className="h-8 w-40" />
          <Skeleton className="h-64" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <EntryHeader file={file} />
      <Tabs value={tab} onValueChange={setTab} className="flex min-h-0 flex-1 flex-col">
        <div className="flex items-center gap-3 px-5 pt-3">
          <TabsList aria-label="View">
            <TabsTrigger value="edit">Edit</TabsTrigger>
            <TabsTrigger value="preview">Preview</TabsTrigger>
          </TabsList>
          {!exists ? <span className="text-[11px] text-subtle">Not on disk yet; Create writes this draft.</span> : null}
        </div>
        <TabsContent value="edit" className="mt-2 flex min-h-0 flex-1 flex-col px-5 pb-3">
          <Textarea aria-label={`Contents of ${entry.display}`} value={value} onChange={(e) => setDraft(e.target.value)} spellCheck={false} className="mono min-h-0 flex-1 resize-none text-xs leading-5" />
        </TabsContent>
        <TabsContent value="preview" className="mt-2 min-h-0 flex-1 scroll-y px-5 pb-3">
          {value.trim() ? (
            isMarkdown(entry.display) ? (
              <Markdown>{value}</Markdown>
            ) : (
              <pre className="mono selectable whitespace-pre-wrap text-[11.5px] leading-5">{value}</pre>
            )
          ) : (
            <p className="text-xs text-muted">Nothing to preview.</p>
          )}
        </TabsContent>
      </Tabs>
      <ErrorNote error={write.error ?? remove.error ?? loadBackup.error} className="mx-5 mb-2" />
      <div className="flex flex-wrap items-center gap-2 border-t px-5 py-3">
        <Button variant="primary" size="sm" loading={write.isPending} disabled={!canSave} onClick={save}>
          <Save /> {exists ? "Save" : "Create"}
        </Button>
        {dirty ? (
          <Button size="sm" variant="ghost" onClick={() => setDraft(null)}>
            <RotateCcw /> Revert
          </Button>
        ) : null}
        <span className="text-[11px] text-subtle">{modKey()}+S saves</span>
        <div className="ml-auto flex items-center gap-1.5">
          <DropdownMenu open={backupsOpen} onOpenChange={setBackupsOpen}>
            <DropdownMenuTrigger asChild>
              <Button size="sm" variant="ghost">
                <History /> Backups
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="max-h-72 overflow-y-auto">
              <DropdownMenuLabel>Previous versions</DropdownMenuLabel>
              {backups.isLoading ? (
                <DropdownMenuItem disabled>Loading…</DropdownMenuItem>
              ) : backups.isError ? (
                <DropdownMenuItem disabled>{backups.error instanceof Error ? backups.error.message : "Could not list backups"}</DropdownMenuItem>
              ) : !backups.data?.length ? (
                <DropdownMenuItem disabled>No backups yet</DropdownMenuItem>
              ) : (
                backups.data
                  .slice()
                  .reverse()
                  .map((b) => (
                    <DropdownMenuItem key={b.path} onSelect={() => loadBackup.mutate(b)}>
                      <span className="num">{formatDate(b.savedAt)}</span>
                      <span className="num ml-auto text-[11px] text-subtle">{formatBytes(b.sizeBytes)}</span>
                    </DropdownMenuItem>
                  ))
              )}
            </DropdownMenuContent>
          </DropdownMenu>
          {exists ? (
            <Button size="sm" variant="danger-ghost" onClick={() => setConfirmDelete(true)}>
              <Trash2 /> Delete
            </Button>
          ) : null}
        </div>
      </div>
      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={`Delete ${entry.display}?`}
        description="A backup is kept."
        confirmLabel="Delete"
        destructive
        loading={remove.isPending}
        onConfirm={() => remove.mutate(undefined)}
      />
    </div>
  );
}
