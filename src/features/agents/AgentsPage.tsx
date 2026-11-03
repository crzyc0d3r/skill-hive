import * as React from "react";
import { Navigate, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import {
  BookOpen,
  ChevronDown,
  ChevronRight,
  Copy,
  ExternalLink,
  FolderCog,
  FolderOpen,
  Layers,
  Monitor,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Rocket,
  Trash2,
} from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useInstalledTargets, useProjects, useTargetStatus, useTargets } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { cn, displayPath, pluralize } from "@/lib/utils";
import {
  Badge,
  Button,
  ConfirmDialog,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  EmptyState,
  Progress,
  Skeleton,
  Tooltip,
  copyText,
  openExternal,
  revealPath,
} from "@/components/ui";
import { BudgetMeter, ConflictList, ErrorNote, InstallStateBadge, KindIcon, Page, PageHeader } from "@/components/app/common";
import { DeployDialog } from "./DeployDialog";
import { ItemPickerDialog } from "./ItemPickerDialog";
import { ScopePicker } from "./ScopePicker";
import { AddTargetDialog } from "./AddTargetDialog";
import { SkillsDirDialog } from "./SkillsDirDialog";

/** Agents: what each agent tool has installed, its context budget, conflicts and per-asset actions. */
export function AgentsPage() {
  const { targetId } = useParams<{ targetId: string }>();
  const navigate = useNavigate();
  const installed = useInstalledTargets();
  const all = useTargets();
  const [addOpen, setAddOpen] = React.useState(false);

  const firstInstalled = installed.data?.[0]?.spec.id;
  if (!targetId && firstInstalled) return <Navigate to={`/agents/${firstInstalled}`} replace />;

  const notDetected = (all.data ?? []).filter((t) => !installed.data?.some((s) => s.spec.id === t.id));

  return (
    <Page>
      <PageHeader
        title="Agents"
        description="What each agent tool loads into its prompt, how much of its context budget that costs, and where installed skills collide."
        actions={
          <Button variant="secondary" onClick={() => setAddOpen(true)}>
            <Plus /> Add custom agent
          </Button>
        }
      />
      <div className="flex min-h-0 flex-1 border-t">
        <TargetRail installed={installed} notDetected={notDetected} selectedId={targetId ?? null} onSelect={(id) => navigate(`/agents/${id}`)} />
        <div className="min-w-0 flex-1 scroll-y px-6 py-4">
          {targetId ? (
            <TargetPanel key={targetId} targetId={targetId} />
          ) : installed.isError ? (
            <ErrorNote error={installed.error} />
          ) : installed.isLoading ? (
            <div className="flex flex-col gap-3">
              <Skeleton className="h-8 w-1/3" />
              <Skeleton className="h-24" />
              <Skeleton className="h-48" />
            </div>
          ) : (
            <EmptyState
              icon={Monitor}
              title="No agent tools detected"
              description="Install Claude Code, Codex, Cursor or another supported agent, pick one from the list to prepare it anyway, or describe a custom agent."
              action={
                <Button variant="primary" onClick={() => setAddOpen(true)}>
                  <Plus /> Add custom agent
                </Button>
              }
            />
          )}
        </div>
      </div>
      <AddTargetDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        onCreated={(spec) => {
          setAddOpen(false);
          navigate(`/agents/${spec.id}`);
        }}
      />
    </Page>
  );
}

function TargetRail({
  installed,
  notDetected,
  selectedId,
  onSelect,
}: {
  installed: ReturnType<typeof useInstalledTargets>;
  notDetected: T.TargetSpec[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  const [othersOverride, setOthersOverride] = React.useState<boolean | null>(null);
  const forcedOpen = notDetected.some((t) => t.id === selectedId);
  const othersOpen = othersOverride ?? forcedOpen;

  return (
    <aside className="flex w-60 shrink-0 flex-col border-r" aria-label="Agents">
      <div className="scroll-y flex-1 p-2">
        <div className="px-2 pb-1 pt-1 text-[11px] font-medium uppercase tracking-wide text-subtle">Detected</div>
        {installed.isLoading ? (
          <div className="flex flex-col gap-1.5 px-1">
            {Array.from({ length: 3 }).map((_, i) => (
              <Skeleton key={i} className="h-16" />
            ))}
          </div>
        ) : installed.data && installed.data.length === 0 ? (
          <p className="px-2.5 py-2 text-xs text-muted">None detected on this machine.</p>
        ) : (
          <div className="flex flex-col gap-0.5">
            {installed.data?.map((t) => (
              <InstalledRow key={t.spec.id} status={t} active={t.spec.id === selectedId} onClick={() => onSelect(t.spec.id)} />
            ))}
          </div>
        )}
        {notDetected.length ? (
          <>
            <button
              type="button"
              onClick={() => setOthersOverride(!othersOpen)}
              aria-expanded={othersOpen}
              className="mt-3 flex w-full items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium uppercase tracking-wide text-subtle hover:text-fg"
            >
              {othersOpen ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
              Not detected
              <span className="num ml-auto">{notDetected.length}</span>
            </button>
            {othersOpen ? (
              <div className="flex flex-col gap-0.5">
                {notDetected.map((spec) => {
                  const active = spec.id === selectedId;
                  return (
                    <button
                      key={spec.id}
                      type="button"
                      onClick={() => onSelect(spec.id)}
                      aria-current={active ? "page" : undefined}
                      className={cn(
                        "flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-xs text-muted transition-colors hover:bg-sunken hover:text-fg",
                        active && "bg-elevated text-fg shadow-sm hairline",
                      )}
                    >
                      <span className="min-w-0 flex-1 truncate">{spec.name}</span>
                      {spec.custom ? <Badge tone="outline">custom</Badge> : null}
                    </button>
                  );
                })}
              </div>
            ) : null}
          </>
        ) : null}
      </div>
    </aside>
  );
}

function InstalledRow({ status: t, active, onClick }: { status: T.TargetStatus; active: boolean; onClick: () => void }) {
  const n = t.conflicts.conflicts.length;
  const high = t.conflicts.conflicts.some((c) => c.severity === "high");
  const frac = t.budgetUsed ?? 0;
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      className={cn("flex w-full flex-col gap-1.5 rounded-lg px-2.5 py-2 text-left transition-colors hover:bg-sunken", active && "bg-elevated shadow-sm hairline hover:bg-elevated")}
    >
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate text-[13px] font-medium">{t.spec.name}</span>
        {n ? <Badge tone={high ? "danger" : "warn"}>{pluralize(n, "conflict")}</Badge> : <Badge tone="ok">clean</Badge>}
      </div>
      <div className="flex items-center gap-2 text-[11px] text-subtle">
        <span>{pluralize(t.assets.length, "asset")}</span>
        <span className="num ml-auto">{t.budgetUsed != null ? `${Math.round(t.budgetUsed * 100)}% of budget` : "budget unknown"}</span>
      </div>
      <Progress value={Math.min(1, frac)} size="sm" tone={frac > 1 ? "danger" : frac > 0.8 ? "warn" : "ok"} />
    </button>
  );
}

/** Keyed by target id so scope and dialogs reset when switching agents. */
function TargetPanel({ targetId }: { targetId: string }) {
  const navigate = useNavigate();
  const openItem = useUi((s) => s.openItem);
  const { data: targets } = useTargets();
  const { data: projects } = useProjects();
  const [scope, setScope] = React.useState<T.Scope>("user");
  const [projectOverride, setProjectOverride] = React.useState<string | null>(null);
  const projectPath = projectOverride ?? projects?.[0]?.path ?? null;
  const effectiveProject = scope === "project" ? projectPath : null;
  const status = useTargetStatus(targetId, scope, effectiveProject);
  const spec = status.data?.spec ?? targets?.find((t) => t.id === targetId);

  const [pickOpen, setPickOpen] = React.useState(false);
  const [deployIds, setDeployIds] = React.useState<string[]>([]);
  const [deployOpen, setDeployOpen] = React.useState(false);
  const [confirm, setConfirm] = React.useState<T.InstalledAsset | null>(null);
  const [skillsDirOpen, setSkillsDirOpen] = React.useState(false);
  const [confirmRemoveTarget, setConfirmRemoveTarget] = React.useState(false);

  const invalidate = [keys.targets, keys.deployments, keys.itemsAll, ["item"], keys.dashboard];
  const resync = useApiMutation((id: string) => api.resync(id), invalidate, (d) => toast.success(`Re-synced ${d.itemName}`));
  const undeploy = useApiMutation((id: string) => api.undeploy(id), invalidate, (d) => {
    toast.success(`Removed ${d.itemName} from ${spec?.name ?? targetId}`);
    setConfirm(null);
  });
  const removePath = useApiMutation((path: string) => api.removeInstalledPath(path), invalidate, (_r, path) => {
    toast.success(`Deleted ${displayPath(path)}`);
    setConfirm(null);
  });
  const addToLibrary = useApiMutation(
    (a: T.InstalledAsset) =>
      api.importItems({
        paths: [a.path],
        source: { kind: "agent_dir", locator: a.path, targetId, fetchedAt: Date.now(), subpath: null, reference: null, revision: null },
        tags: [],
        overwrite: false,
      }),
    [...invalidate, keys.stats],
    (res, a) => {
      if (res.imported.length) toast.success(`Added ${a.name} to the library`);
      for (const s of res.skipped) toast.warning(`Skipped ${a.name}`, { description: s.reason });
    },
  );
  const removeTarget = useApiMutation(() => api.removeCustomTarget(targetId), [keys.targets, keys.installedTargets, keys.dashboard], () => {
    toast.success(`Removed ${spec?.name ?? targetId}`);
    navigate("/agents", { replace: true });
  });

  if (status.isError && !status.data) {
    return (
      <div className="flex flex-col gap-3">
        <ErrorNote error={status.error} />
        <div>
          <Button size="sm" onClick={() => void status.refetch()}>
            Retry
          </Button>
        </div>
      </div>
    );
  }
  if (!spec) {
    return (
      <div className="flex flex-col gap-3" aria-busy="true">
        <Skeleton className="h-8 w-1/3" />
        <Skeleton className="h-24" />
        <Skeleton className="h-48" />
      </div>
    );
  }

  const s = status.data;
  const assets = s?.assets ?? [];
  const managedCount = assets.filter((a) => a.deploymentId).length;
  const installedLibraryIds = assets.map((a) => a.libraryItemId).filter((x): x is string => !!x);
  const needsProject = scope === "project" && !projectPath;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h2 className="text-base font-semibold tracking-[-0.01em]">{spec.name}</h2>
            {s ? <Badge tone={s.installed ? "ok" : "neutral"}>{s.installed ? "Installed" : "Not detected"}</Badge> : null}
            {spec.custom ? <Badge tone="outline">Custom</Badge> : null}
            {spec.docsUrl ? (
              <Button variant="link" size="sm" onClick={() => void openExternal(spec.docsUrl!)}>
                Docs <ExternalLink />
              </Button>
            ) : null}
          </div>
          {s ? (
            <div className="mt-0.5 flex items-center gap-0.5 text-xs text-muted">
              <span className="mono selectable truncate" title={s.skillsPath}>
                {displayPath(s.skillsPath)}
              </span>
              <Tooltip content="Copy path">
                <Button size="icon-sm" variant="ghost" aria-label="Copy skills path" onClick={() => void copyText(s.skillsPath).then(() => toast("Path copied"))}>
                  <Copy />
                </Button>
              </Tooltip>
              <Tooltip content="Reveal in file manager">
                <Button size="icon-sm" variant="ghost" aria-label="Reveal skills folder" onClick={() => void revealPath(s.skillsPath)}>
                  <FolderOpen />
                </Button>
              </Tooltip>
            </div>
          ) : null}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button variant="primary" onClick={() => setPickOpen(true)}>
            <Rocket /> Deploy…
          </Button>
          <Button variant="secondary" onClick={() => navigate(`/loadouts?target=${targetId}`)}>
            <Layers /> Apply loadout…
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" aria-label="More actions">
                <MoreHorizontal />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={() => setSkillsDirOpen(true)}>
                <FolderCog /> Change skills folder…
              </DropdownMenuItem>
              {s ? (
                <DropdownMenuItem onSelect={() => void revealPath(s.skillsPath)}>
                  <FolderOpen /> Reveal skills folder
                </DropdownMenuItem>
              ) : null}
              {spec.docsUrl ? (
                <DropdownMenuItem onSelect={() => void openExternal(spec.docsUrl!)}>
                  <ExternalLink /> Open documentation
                </DropdownMenuItem>
              ) : null}
              {spec.custom ? (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem destructive onSelect={() => setConfirmRemoveTarget(true)}>
                    <Trash2 /> Remove custom agent
                  </DropdownMenuItem>
                </>
              ) : null}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <ScopePicker scope={scope} onScopeChange={setScope} projectPath={projectPath} onProjectChange={setProjectOverride} projectDisabled={!spec.projectSkillsDir} />
        {s ? (
          <span className="ml-auto text-xs text-muted">
            {pluralize(assets.length, "asset")} · {managedCount} managed · {assets.length - managedCount} unmanaged
          </span>
        ) : null}
      </div>

      {needsProject ? (
        <EmptyState compact icon={FolderOpen} title="No project selected" description="Add a project folder to see what this agent loads there and deploy project-scoped skills." />
      ) : status.isLoading && !s ? (
        <div className="flex flex-col gap-3" aria-busy="true">
          <Skeleton className="h-24" />
          <Skeleton className="h-48" />
        </div>
      ) : s ? (
        <>
          <div className="grid gap-3 lg:grid-cols-[1.1fr_1fr]">
            <div className="surface p-3">
              <BudgetMeter chars={s.contextChars} budgetChars={spec.descriptionBudgetChars} tokens={s.contextTokens} />
              <p className="mt-2 text-[11px] text-subtle">
                Every installed skill's name and description is loaded into the prompt up front. Past the budget the list is truncated and skills silently stop triggering.
              </p>
            </div>
            <div className="surface p-3">
              <div className="mb-1.5 flex items-center justify-between text-[11px] font-medium uppercase tracking-wide text-subtle">
                <span>Conflicts</span>
                {s.conflicts.conflicts.length ? (
                  <Badge tone={s.conflicts.conflicts.some((c) => c.severity === "high") ? "danger" : "warn"}>{s.conflicts.conflicts.length}</Badge>
                ) : null}
              </div>
              <ConflictList report={s.conflicts} compact />
            </div>
          </div>

          <AssetTable
            assets={assets}
            onOpen={(id) => openItem(id)}
            onResync={(id) => resync.mutate(id)}
            onRemove={(a) => setConfirm(a)}
            onAddToLibrary={(a) => addToLibrary.mutate(a)}
            busyPath={addToLibrary.isPending ? addToLibrary.variables?.path : resync.isPending ? assets.find((a) => a.deploymentId === resync.variables)?.path : undefined}
            onDeploy={() => setPickOpen(true)}
          />
          <ErrorNote error={resync.error ?? addToLibrary.error} />
        </>
      ) : null}

      <ItemPickerDialog
        open={pickOpen}
        onOpenChange={setPickOpen}
        title={`Deploy to ${spec.name}`}
        description="Choose what to install. Items already installed here are hidden."
        confirmLabel="Next: Deploy"
        excludeIds={installedLibraryIds}
        onConfirm={(ids) => {
          setDeployIds(ids);
          setPickOpen(false);
          setDeployOpen(true);
        }}
      />
      <DeployDialog open={deployOpen} onOpenChange={setDeployOpen} itemIds={deployIds} defaultTarget={targetId} defaultScope={scope} defaultProject={projectPath} />
      {s ? <SkillsDirDialog open={skillsDirOpen} onOpenChange={setSkillsDirOpen} spec={spec} currentPath={s.skillsPath} /> : null}
      <ConfirmDialog
        open={!!confirm}
        onOpenChange={(o) => !o && setConfirm(null)}
        title={confirm?.deploymentId ? `Remove ${confirm.name} from ${spec.name}?` : `Delete ${confirm?.name ?? "this file"} from disk?`}
        description={
          confirm?.deploymentId
            ? "The link or copy is removed from the agent's folder. The library keeps the item."
            : "Skill Hive did not put this file here. It is deleted from the agent's folder and cannot be recovered; add it to the library first if you want to keep it."
        }
        confirmLabel={confirm?.deploymentId ? "Remove" : "Delete"}
        destructive
        loading={undeploy.isPending || removePath.isPending}
        onConfirm={() => {
          if (!confirm) return;
          if (confirm.deploymentId) undeploy.mutate(confirm.deploymentId);
          else removePath.mutate(confirm.path);
        }}
      />
      <ConfirmDialog
        open={confirmRemoveTarget}
        onOpenChange={setConfirmRemoveTarget}
        title={`Remove ${spec.name}?`}
        description="The custom agent definition is deleted. Files in its folders are not touched."
        confirmLabel="Remove"
        destructive
        loading={removeTarget.isPending}
        onConfirm={() => removeTarget.mutate(undefined)}
      />
    </div>
  );
}

export function AssetTable({
  assets,
  onOpen,
  onResync,
  onRemove,
  onAddToLibrary,
  onDeploy,
  busyPath,
}: {
  assets: T.InstalledAsset[];
  onOpen: (libraryItemId: string) => void;
  onResync: (deploymentId: string) => void;
  onRemove: (asset: T.InstalledAsset) => void;
  onAddToLibrary: (asset: T.InstalledAsset) => void;
  onDeploy?: () => void;
  busyPath?: string;
}) {
  const cols = "grid-cols-[auto_minmax(0,1.1fr)_minmax(0,1.6fr)_auto_auto_auto]";
  return (
    <div className="surface overflow-hidden" role="table" aria-label="Installed assets">
      <div role="row" className={cn("grid items-center gap-3 border-b bg-sunken/60 px-3 py-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle", cols)}>
        <span className="w-4" />
        <span role="columnheader">Name</span>
        <span role="columnheader">Description</span>
        <span role="columnheader">State</span>
        <span role="columnheader">Managed</span>
        <span role="columnheader" className="text-right">
          Actions
        </span>
      </div>
      {assets.length === 0 ? (
        <div className="flex flex-col items-center gap-2 px-3 py-8 text-center text-xs text-muted">
          Nothing installed here yet.
          {onDeploy ? (
            <Button size="sm" onClick={onDeploy}>
              <Rocket /> Deploy…
            </Button>
          ) : null}
        </div>
      ) : (
        assets.map((a) => {
          const managed = !!a.deploymentId;
          const canResync = managed && (a.state === "stale" || a.state === "modified" || a.state === "broken");
          const busy = busyPath === a.path;
          return (
            <div key={a.path} role="row" className={cn("grid items-center gap-3 border-b px-3 py-2 text-xs last:border-b-0", cols)}>
              <Tooltip content={a.kind}>
                <span className="flex w-4 justify-center">
                  <KindIcon kind={a.kind} className="size-4 text-muted" />
                </span>
              </Tooltip>
              <span className="flex min-w-0 flex-col">
                <span className="truncate text-[13px] font-medium">{a.name}</span>
                <span className="mono truncate text-[10.5px] text-subtle" title={a.path}>
                  {displayPath(a.path)}
                </span>
              </span>
              <span className={cn("truncate text-muted", !a.description && "italic text-subtle")} title={a.description}>
                {a.description || "No description"}
              </span>
              <span>{a.state === "unmanaged" ? <span className="text-subtle">—</span> : <InstallStateBadge state={a.state} />}</span>
              <span>{managed ? <Badge tone="accent">Skill Hive</Badge> : <Badge tone="outline">Unmanaged</Badge>}</span>
              <span className="flex items-center justify-end gap-0.5">
                {a.libraryItemId ? (
                  <Tooltip content="Open in library">
                    <Button size="icon-sm" variant="ghost" aria-label={`Open ${a.name} in library`} onClick={() => onOpen(a.libraryItemId!)}>
                      <BookOpen />
                    </Button>
                  </Tooltip>
                ) : null}
                {canResync ? (
                  <Tooltip content="Overwrite with the library version">
                    <Button size="icon-sm" variant="ghost" aria-label={`Re-sync ${a.name}`} loading={busy} onClick={() => onResync(a.deploymentId!)}>
                      <RefreshCw />
                    </Button>
                  </Tooltip>
                ) : null}
                {!managed && !a.libraryItemId ? (
                  <Button size="sm" variant="subtle" loading={busy} onClick={() => onAddToLibrary(a)}>
                    <Plus /> Add to library
                  </Button>
                ) : null}
                <Tooltip content={managed ? "Remove from this agent" : "Delete this unmanaged file"}>
                  <Button size="icon-sm" variant="ghost" aria-label={`Remove ${a.name}`} onClick={() => onRemove(a)}>
                    <Trash2 className="text-danger" />
                  </Button>
                </Tooltip>
              </span>
            </div>
          );
        })
      )}
    </div>
  );
}
