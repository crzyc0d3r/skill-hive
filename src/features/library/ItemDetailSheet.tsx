import * as React from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import {
  AlertTriangle,
  Check,
  Copy,
  ExternalLink,
  File,
  FlaskConical,
  Folder,
  FolderOpen,
  Info,
  Layers,
  MoreHorizontal,
  Pencil,
  RefreshCw,
  Rocket,
  Save,
  Star,
  Trash2,
  Unlink,
  X,
  XCircle,
} from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useItem } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { cn, displayPath, formatBytes, formatDate, formatTokens, pluralize } from "@/lib/utils";
import {
  Badge,
  Button,
  ConfirmDialog,
  Dialog,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  Input,
  LineTabsList,
  LineTabsTrigger,
  Markdown,
  SheetContent,
  Skeleton,
  Tabs,
  TabsContent,
  Textarea,
  Tooltip,
  copyText,
  revealPath,
} from "@/components/ui";
import { EffectivenessBadge, ErrorNote, InlineCode, InstallStateBadge, KindBadge, SourceBadge, VerdictBadge, sourceDisplay } from "@/components/app/common";
import { DeployDialog } from "@/features/agents/DeployDialog";
import { HistorySparkline } from "@/features/evals/HistorySparkline";

export function ItemDetailSheet() {
  const id = useUi((s) => s.detailItemId);
  const openItem = useUi((s) => s.openItem);
  return (
    <Dialog open={!!id} onOpenChange={(o) => !o && openItem(null)}>
      <SheetContent width="min(640px, 94vw)" aria-describedby={undefined}>
        {id ? <ItemDetail id={id} onClose={() => openItem(null)} /> : null}
      </SheetContent>
    </Dialog>
  );
}

function ItemDetail({ id, onClose }: { id: string; onClose: () => void }) {
  const { data, isLoading, error } = useItem(id);
  const navigate = useNavigate();
  const [tab, setTab] = React.useState("overview");
  const [deployOpen, setDeployOpen] = React.useState(false);
  const [confirmRemove, setConfirmRemove] = React.useState(false);
  const [editing, setEditing] = React.useState(false);
  const [draft, setDraft] = React.useState("");

  const invalidate = [keys.item(id), keys.itemsAll, keys.stats, keys.dashboard];
  const favorite = useApiMutation((v: boolean) => api.setFavorite(id, v), invalidate);
  const setTags = useApiMutation((tags: string[]) => api.setTags(id, tags), invalidate);
  const setNotes = useApiMutation((notes: string | null) => api.setNotes(id, notes), invalidate);
  const remove = useApiMutation(() => api.removeItem(id), [keys.itemsAll, keys.stats, keys.dashboard, keys.deployments], (item) => {
    toast.success(`Removed ${item.name}`);
    onClose();
  });
  const undeploy = useApiMutation((depId: string) => api.undeploy(depId), [...invalidate, keys.deployments, keys.targets]);
  const resync = useApiMutation((depId: string) => api.resync(depId), [...invalidate, keys.deployments, keys.targets]);
  const update = useApiMutation(() => api.updateItem(id), [...invalidate, keys.deployments], () => toast.success("Updated from source"));
  const write = useApiMutation((text: string) => api.writeDocument(id, text), invalidate, () => {
    toast.success("Saved");
    setEditing(false);
  });
  const readDoc = useApiMutation((path: string) => api.readItemFile(id, path), [], (text) => {
    setDraft(text);
    setEditing(true);
  });

  if (isLoading || !data) {
    return (
      <div className="flex flex-col gap-4 p-6">
        <Skeleton className="h-6 w-1/2" />
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-4/5" />
        <Skeleton className="h-40 w-full" />
        {error ? <ErrorNote error={error} /> : null}
      </div>
    );
  }
  const { item, body, files, validation, deployments, loadouts, history } = data;
  const mainFile = item.kind === "skill" ? "SKILL.md" : `${item.name}.md`;
  const errors = validation.findings.filter((f) => f.severity === "error");
  const warnings = validation.findings.filter((f) => f.severity !== "error");
  const canUpdate = item.source.kind === "github" || item.source.kind === "git" || item.source.kind === "url";

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-start gap-3 border-b px-5 pb-3 pt-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <KindBadge kind={item.kind} />
            <SourceBadge source={item.source} />
            {item.deploymentCount > 0 ? (
              <Badge tone="ok">
                <Rocket /> {pluralize(item.deploymentCount, "deployment")}
              </Badge>
            ) : null}
          </div>
          <h2 className="mt-1.5 truncate text-lg font-semibold tracking-[-0.015em] selectable">{item.name}</h2>
          <p className="mt-0.5 text-xs text-muted selectable">{item.description}</p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <Tooltip content={item.favorite ? "Unfavourite" : "Favourite"}>
            <Button variant="ghost" size="icon-sm" onClick={() => favorite.mutate(!item.favorite)}>
              <Star className={cn(item.favorite && "fill-accent text-accent")} />
            </Button>
          </Tooltip>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm" aria-label="More">
                <MoreHorizontal />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={() => readDoc.mutate(mainFile)}>
                <Pencil /> Edit {mainFile}
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => void revealPath(item.path)}>
                <FolderOpen /> Reveal in file manager
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => void copyText(item.path).then(() => toast("Path copied"))}>
                <Copy /> Copy path
              </DropdownMenuItem>
              {canUpdate ? (
                <DropdownMenuItem onSelect={() => update.mutate(undefined)}>
                  <RefreshCw /> Update from source
                </DropdownMenuItem>
              ) : null}
              <DropdownMenuSeparator />
              <DropdownMenuItem destructive onSelect={() => setConfirmRemove(true)}>
                <Trash2 /> Remove from library
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          <Button variant="ghost" size="icon-sm" onClick={onClose} aria-label="Close">
            <X />
          </Button>
        </div>
      </div>

      <div className="flex items-center gap-2 border-b px-5 py-2.5">
        <Button variant="primary" size="sm" onClick={() => setDeployOpen(true)}>
          <Rocket /> Deploy
        </Button>
        {item.kind === "skill" ? (
          <Button size="sm" onClick={() => navigate(`/evals/${item.id}`)}>
            <FlaskConical /> Evaluate
          </Button>
        ) : null}
        <Button size="sm" variant="ghost" onClick={() => readDoc.mutate(mainFile)} loading={readDoc.isPending}>
          <Pencil /> Edit
        </Button>
        <div className="ml-auto flex items-center gap-1.5">
          <EffectivenessBadge e={item.effectiveness} />
        </div>
      </div>

      {editing ? (
        <div className="flex min-h-0 flex-1 flex-col gap-3 p-5">
          <div className="flex items-center justify-between text-xs text-muted">
            <span>
              Editing <InlineCode>{mainFile}</InlineCode>. Frontmatter is validated on save.
            </span>
            <span className="flex gap-1.5">
              <Button size="sm" variant="ghost" onClick={() => setEditing(false)}>
                Cancel
              </Button>
              <Button size="sm" variant="primary" loading={write.isPending} onClick={() => write.mutate(draft)}>
                <Save /> Save
              </Button>
            </span>
          </div>
          <Textarea value={draft} onChange={(e) => setDraft(e.target.value)} className="mono min-h-0 flex-1 text-xs leading-5" spellCheck={false} />
          <ErrorNote error={write.error} />
        </div>
      ) : (
        <Tabs value={tab} onValueChange={setTab} className="flex min-h-0 flex-1 flex-col">
          <LineTabsList className="px-5">
            <LineTabsTrigger value="overview">Overview</LineTabsTrigger>
            <LineTabsTrigger value="instructions">Instructions</LineTabsTrigger>
            <LineTabsTrigger value="files">Files ({files.filter((f) => !f.isDir).length})</LineTabsTrigger>
            <LineTabsTrigger value="deployments">Deployments ({deployments.length})</LineTabsTrigger>
            {validation.findings.length ? (
              <LineTabsTrigger value="validation">
                {errors.length ? <XCircle className="text-danger" /> : <AlertTriangle className="text-warn" />}
                Validation ({validation.findings.length})
              </LineTabsTrigger>
            ) : null}
          </LineTabsList>
          <div className="min-h-0 flex-1 scroll-y px-5 py-4">
            <TabsContent value="overview" className="mt-0 flex flex-col gap-5">
              <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-xs">
                <dt className="text-subtle">Source</dt>
                <dd className="selectable">{sourceDisplay(item.source)}</dd>
                {item.source.revision ? (
                  <>
                    <dt className="text-subtle">Revision</dt>
                    <dd className="mono selectable">{item.source.revision.slice(0, 12)}</dd>
                  </>
                ) : null}
                <dt className="text-subtle">Location</dt>
                <dd className="mono selectable break-all">{displayPath(item.path)}</dd>
                <dt className="text-subtle">Size</dt>
                <dd className="num">
                  {formatBytes(item.sizeBytes)} · ~{formatTokens(item.tokenEstimate)} tokens of instructions · {item.descriptionTokens} tokens of description
                </dd>
                <dt className="text-subtle">Updated</dt>
                <dd>{formatDate(item.updatedAt)}</dd>
                <dt className="text-subtle">Content hash</dt>
                <dd className="mono selectable">{item.contentHash.slice(0, 16)}</dd>
              </dl>

              <section>
                <div className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle">Tags</div>
                <TagEditor tags={item.tags} onChange={(tags) => setTags.mutate(tags)} />
              </section>

              {item.kind === "skill" ? (
                <section>
                  <div className="mb-1.5 flex items-center justify-between text-[11px] font-medium uppercase tracking-wide text-subtle">
                    <span>Measured effectiveness</span>
                    <Button variant="link" size="sm" onClick={() => navigate(`/evals/${item.id}`)}>
                      Open evals
                    </Button>
                  </div>
                  {history.length ? (
                    <div className="surface p-3">
                      <HistorySparkline points={history} />
                      <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted">
                        <VerdictBadge verdict={history[history.length - 1].verdict} delta={history[history.length - 1].delta} />
                        <span>
                          over {pluralize(history.length, "run")} · latest with skill {Math.round(history[history.length - 1].withSkillPassRate * 100)}% vs {Math.round(history[history.length - 1].baselinePassRate * 100)}% baseline
                        </span>
                      </div>
                    </div>
                  ) : (
                    <div className="rounded-lg border border-dashed px-3 py-3 text-xs text-muted">
                      Not measured yet. Run a with-skill vs without-skill evaluation to see whether these instructions actually help.
                    </div>
                  )}
                </section>
              ) : null}

              {loadouts.length ? (
                <section>
                  <div className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle">In loadouts</div>
                  <div className="flex flex-wrap gap-1.5">
                    {loadouts.map((l) => (
                      <Button key={l.id} size="sm" variant="subtle" onClick={() => navigate(`/loadouts/${l.id}`)}>
                        <Layers /> {l.name}
                      </Button>
                    ))}
                  </div>
                </section>
              ) : null}

              <section>
                <div className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle">Notes</div>
                <NotesEditor key={item.notes ?? ""} notes={item.notes ?? ""} onSave={(n) => setNotes.mutate(n || null)} />
              </section>

              {Object.keys(item.frontmatter).filter((k) => !["name", "description"].includes(k)).length ? (
                <section>
                  <div className="mb-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle">Frontmatter</div>
                  <pre className="mono selectable overflow-x-auto rounded-lg bg-sunken p-3 text-[11.5px] leading-5">
                    {Object.entries(item.frontmatter)
                      .filter(([k]) => !["name", "description"].includes(k))
                      .map(([k, v]) => `${k}: ${typeof v === "string" ? v : JSON.stringify(v)}`)
                      .join("\n")}
                  </pre>
                </section>
              ) : null}
            </TabsContent>

            <TabsContent value="instructions" className="mt-0">
              {body.trim() ? <Markdown>{body}</Markdown> : <p className="text-sm text-muted">No instructions after the frontmatter.</p>}
            </TabsContent>

            <TabsContent value="files" className="mt-0">
              <FileBrowser itemId={item.id} files={files} />
            </TabsContent>

            <TabsContent value="deployments" className="mt-0 flex flex-col gap-2">
              {deployments.length === 0 ? (
                <div className="rounded-lg border border-dashed px-3 py-6 text-center text-xs text-muted">
                  Not deployed anywhere. Deploy it to an agent so it can use these instructions.
                </div>
              ) : (
                deployments.map((d) => (
                  <div key={d.id} className="flex items-center gap-3 rounded-lg px-3 py-2 hairline">
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 text-[13px] font-medium">
                        {d.targetId}
                        <Badge tone="outline">{d.scope}</Badge>
                        <Badge tone="outline">{d.mode}</Badge>
                        <InstallStateBadge state={d.state} />
                      </div>
                      <div className="mono truncate text-[11px] text-subtle">{displayPath(d.path)}</div>
                    </div>
                    {d.state === "stale" || d.state === "modified" ? (
                      <Tooltip content="Overwrite with the library version">
                        <Button size="sm" variant="secondary" loading={resync.isPending} onClick={() => resync.mutate(d.id)}>
                          <RefreshCw /> Re-sync
                        </Button>
                      </Tooltip>
                    ) : null}
                    <Tooltip content="Remove from this agent">
                      <Button size="icon-sm" variant="ghost" onClick={() => undeploy.mutate(d.id)}>
                        <Unlink />
                      </Button>
                    </Tooltip>
                  </div>
                ))
              )}
            </TabsContent>

            <TabsContent value="validation" className="mt-0 flex flex-col gap-1.5">
              {[...errors, ...warnings].map((f, i) => (
                <div key={i} className={cn("flex items-start gap-2 rounded-lg px-3 py-2 text-xs", f.severity === "error" ? "bg-danger-soft text-danger" : f.severity === "warning" ? "bg-warn-soft text-warn" : "bg-info-soft text-info")}>
                  {f.severity === "error" ? <XCircle className="mt-0.5 size-3.5 shrink-0" /> : f.severity === "warning" ? <AlertTriangle className="mt-0.5 size-3.5 shrink-0" /> : <Info className="mt-0.5 size-3.5 shrink-0" />}
                  <div>
                    <div className="font-medium text-fg">
                      {f.message}
                      {f.field ? <InlineCode className="ml-1.5">{f.field}</InlineCode> : null}
                    </div>
                    <div className="mono text-[10.5px] opacity-70">{f.code}</div>
                  </div>
                </div>
              ))}
            </TabsContent>
          </div>
        </Tabs>
      )}

      <DeployDialog open={deployOpen} onOpenChange={setDeployOpen} itemIds={[item.id]} />
      <ConfirmDialog
        open={confirmRemove}
        onOpenChange={setConfirmRemove}
        title={`Remove ${item.name} from the library?`}
        description={`${pluralize(deployments.length, "deployment")} will be removed as well. The source is not touched.`}
        confirmLabel="Remove"
        destructive
        loading={remove.isPending}
        onConfirm={() => remove.mutate(undefined)}
      />
    </div>
  );
}

function TagEditor({ tags, onChange }: { tags: string[]; onChange: (tags: string[]) => void }) {
  const [value, setValue] = React.useState("");
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {tags.map((t) => (
        <span key={t} className="inline-flex items-center gap-1 rounded-full bg-sunken pl-2 pr-1 text-[11px] text-muted">
          {t}
          <button type="button" className="rounded-full p-0.5 hover:text-fg" onClick={() => onChange(tags.filter((x) => x !== t))} aria-label={`Remove tag ${t}`}>
            <X className="size-3" />
          </button>
        </span>
      ))}
      <Input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if ((e.key === "Enter" || e.key === ",") && value.trim()) {
            e.preventDefault();
            onChange([...tags, value.trim().toLowerCase()]);
            setValue("");
          }
        }}
        placeholder="Add tag…"
        className="h-6 w-28 text-[11px]"
      />
    </div>
  );
}

function NotesEditor({ notes, onSave }: { notes: string; onSave: (n: string) => void }) {
  const [value, setValue] = React.useState(notes);
  const dirty = value !== notes;
  return (
    <div className="flex flex-col gap-1.5">
      <Textarea value={value} onChange={(e) => setValue(e.target.value)} rows={2} placeholder="Why you keep this skill, caveats, who maintains it…" className="text-xs" />
      {dirty ? (
        <div className="flex justify-end gap-1.5">
          <Button size="sm" variant="ghost" onClick={() => setValue(notes)}>
            Discard
          </Button>
          <Button size="sm" variant="secondary" onClick={() => onSave(value.trim())}>
            <Check /> Save note
          </Button>
        </div>
      ) : null}
    </div>
  );
}

function FileBrowser({ itemId, files }: { itemId: string; files: T.FileEntry[] }) {
  const [open, setOpen] = React.useState<string | null>(null);
  const [content, setContent] = React.useState<string>("");
  const read = useApiMutation((path: string) => api.readItemFile(itemId, path), [], (text, path) => {
    setContent(text);
    setOpen(path);
  });
  const isText = (p: string) => /\.(md|txt|py|js|ts|json|yaml|yml|toml|sh|csv|html|css|rs|go|java|rb)$/i.test(p) || !p.includes(".");
  return (
    <div className="flex flex-col gap-3">
      <ul className="surface divide-y overflow-hidden">
        {files.map((f) => (
          <li key={f.path}>
            <button
              type="button"
              disabled={f.isDir || !isText(f.path)}
              onClick={() => read.mutate(f.path)}
              className={cn("flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs hover:bg-sunken disabled:opacity-60 disabled:hover:bg-transparent", open === f.path && "bg-sunken")}
              style={{ paddingLeft: 12 + f.path.split("/").length * 10 - 10 }}
            >
              {f.isDir ? <Folder className="size-3.5 text-muted" /> : <File className="size-3.5 text-muted" />}
              <span className="mono truncate">{f.path.split("/").pop()}</span>
              {!f.isDir ? <span className="ml-auto num text-subtle">{formatBytes(f.sizeBytes)}</span> : null}
            </button>
          </li>
        ))}
      </ul>
      {open ? (
        <div className="surface overflow-hidden">
          <div className="flex items-center justify-between border-b bg-sunken/60 px-3 py-1.5 text-[11px]">
            <span className="mono">{open}</span>
            <span className="flex items-center gap-1">
              <Button size="icon-sm" variant="ghost" onClick={() => void copyText(content)}>
                <Copy />
              </Button>
              <Button size="icon-sm" variant="ghost" onClick={() => setOpen(null)}>
                <X />
              </Button>
            </span>
          </div>
          {open.endsWith(".md") ? <div className="p-3"><Markdown>{content}</Markdown></div> : <pre className="mono selectable max-h-[420px] overflow-auto p-3 text-[11.5px] leading-5">{content}</pre>}
        </div>
      ) : null}
      <ErrorNote error={read.error} />
      <p className="flex items-center gap-1 text-[11px] text-subtle">
        <ExternalLink className="size-3" /> Open the folder in your file manager from the ⋯ menu to edit bundled files.
      </p>
    </div>
  );
}
