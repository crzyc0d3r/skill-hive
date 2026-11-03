import * as React from "react";
import { toast } from "sonner";
import { Activity, Cpu, MoreHorizontal, Pencil, Plus, ScanSearch, Star, Trash2 } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useProviders, useSettings } from "@/lib/api/hooks";
import { cn, relativeTime } from "@/lib/utils";
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  ConfirmDialog,
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Dot,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
  EmptyState,
  Skeleton,
  Tooltip,
} from "@/components/ui";
import { ErrorNote, InlineCode } from "@/components/app/common";
import { ProviderDialog, type ProviderPreset } from "./ProviderDialog";
import { PROVIDER_KIND_META, ROLE_META, useSettingsPatch } from "./shared";

export function ProvidersTab() {
  const { data: providers, isLoading, error } = useProviders();
  const { data: settings } = useSettings();
  const [dialog, setDialog] = React.useState<{ provider?: T.ProviderConfig; preset?: ProviderPreset } | null>(null);
  const [detected, setDetected] = React.useState<T.DetectedCli[] | null>(null);
  const [pendingDelete, setPendingDelete] = React.useState<T.ProviderConfig | null>(null);
  const setDefault = useSettingsPatch();

  const probe = useApiMutation((id: string) => api.probeProvider(id), [keys.providers, keys.dashboard], (res, id) => {
    const name = providers?.find((p) => p.id === id)?.name ?? "Provider";
    if (res.ok) toast.success(`${name} is reachable`, { description: `${res.latencyMs} ms${res.version ? ` · ${res.version}` : ""}` });
    else toast.error(`${name} is not reachable`, { description: res.message });
  });
  const remove = useApiMutation((id: string) => api.deleteProvider(id), [keys.providers, keys.dashboard], () => {
    toast.success("Provider removed");
    setPendingDelete(null);
  });
  const detect = useApiMutation(() => api.detectClis(), [], (list) => setDetected(list));

  const list = providers ?? [];
  const isAdded = (c: T.DetectedCli) => list.some((p) => p.kind === c.kind && (p.command ?? PROVIDER_KIND_META[p.kind].defaultCommand) === c.command);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="max-w-xl text-xs text-muted">
          Evaluations need an <span className="font-medium text-fg">executor</span> (an agent CLI that performs tasks) and benefit from a <span className="font-medium text-fg">grader</span> (an API model that judges expectations with evidence). Recommendations use a <span className="font-medium text-fg">recommender</span> to re-rank results.
        </p>
        <div className="flex items-center gap-2">
          <Button variant="secondary" loading={detect.isPending} onClick={() => detect.mutate(undefined)}>
            <ScanSearch /> Detect CLIs
          </Button>
          <Button variant="primary" onClick={() => setDialog({})}>
            <Plus /> Add provider
          </Button>
        </div>
      </div>
      <ErrorNote error={error ?? detect.error} />

      {isLoading ? (
        <div className="grid gap-3 xl:grid-cols-2">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-36" />
          ))}
        </div>
      ) : list.length === 0 ? (
        <EmptyState
          icon={Cpu}
          title="No providers yet"
          description="Detect the agent CLIs installed on this machine, or add an API endpoint for grading."
          action={
            <>
              <Button variant="primary" loading={detect.isPending} onClick={() => detect.mutate(undefined)}>
                <ScanSearch /> Detect CLIs
              </Button>
              <Button onClick={() => setDialog({})}>
                <Plus /> Add provider
              </Button>
            </>
          }
        />
      ) : (
        <div className="grid gap-3 xl:grid-cols-2">
          {list.map((p) => {
            const meta = PROVIDER_KIND_META[p.kind];
            const isExec = settings?.defaultExecutorId === p.id;
            const isGrader = settings?.defaultGraderId === p.id;
            const probing = probe.isPending && probe.variables === p.id;
            return (
              <Card key={p.id}>
                <CardHeader>
                  <div className="min-w-0">
                    <CardTitle className="flex items-center gap-2">
                      <span className="truncate">{p.name}</span>
                      {isExec ? (
                        <Tooltip content="Used by default to execute eval tasks">
                          <Badge tone="accent">
                            <Star /> Executor
                          </Badge>
                        </Tooltip>
                      ) : null}
                      {isGrader ? (
                        <Tooltip content="Used by default to grade results">
                          <Badge tone="accent">
                            <Star /> Grader
                          </Badge>
                        </Tooltip>
                      ) : null}
                    </CardTitle>
                    <p className="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-xs text-muted">
                      <span>{meta.label}</span>
                      {p.model ? (
                        <>
                          <span>·</span>
                          <span className="mono">{p.model}</span>
                        </>
                      ) : null}
                      {meta.family === "http" && p.baseUrl ? (
                        <>
                          <span>·</span>
                          <span className="mono truncate">{p.baseUrl.replace(/^https?:\/\//, "")}</span>
                        </>
                      ) : null}
                      {meta.family !== "http" && p.command ? (
                        <>
                          <span>·</span>
                          <InlineCode>{[p.command, ...p.args].join(" ")}</InlineCode>
                        </>
                      ) : null}
                    </p>
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm" aria-label={`Actions for ${p.name}`}>
                        <MoreHorizontal />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onSelect={() => setDialog({ provider: p })}>
                        <Pencil /> Edit
                      </DropdownMenuItem>
                      <DropdownMenuItem disabled={!p.roles.includes("executor") || isExec} onSelect={() => setDefault.mutate({ patch: { defaultExecutorId: p.id }, message: `${p.name} is the default executor` })}>
                        <Star /> Make default executor
                      </DropdownMenuItem>
                      <DropdownMenuItem disabled={!p.roles.includes("grader") || isGrader} onSelect={() => setDefault.mutate({ patch: { defaultGraderId: p.id }, message: `${p.name} is the default grader` })}>
                        <Star /> Make default grader
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem destructive onSelect={() => setPendingDelete(p)}>
                        <Trash2 /> Delete
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </CardHeader>
                <CardContent className="flex flex-col gap-3">
                  <div className="flex flex-wrap items-center gap-1.5">
                    {p.roles.map((r) => (
                      <Tooltip key={r} content={ROLE_META[r].description}>
                        <Badge tone="outline">{ROLE_META[r].label}</Badge>
                      </Tooltip>
                    ))}
                    <span className="ml-auto num text-[11px] text-subtle">
                      {p.timeoutSecs} s · ×{p.maxConcurrency}
                      {meta.family === "http" ? (p.apiKeyEnv ? ` · key from $${p.apiKeyEnv}` : p.hasSecret ? " · key in keychain" : " · no key") : ""}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <ProbeStatus probe={p.lastProbe ?? null} />
                    <span className="ml-auto flex items-center gap-1.5">
                      <Button size="sm" variant="secondary" loading={probing} onClick={() => probe.mutate(p.id)}>
                        <Activity /> Test
                      </Button>
                      <Button size="sm" variant="ghost" onClick={() => setDialog({ provider: p })}>
                        <Pencil /> Edit
                      </Button>
                    </span>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <ProviderDialog open={!!dialog} onOpenChange={(o) => !o && setDialog(null)} provider={dialog?.provider} preset={dialog?.preset} />

      <Dialog open={detected !== null} onOpenChange={(o) => !o && setDetected(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Detected CLIs</DialogTitle>
            <DialogDescription>Agent command-line tools found on this machine's PATH.</DialogDescription>
          </DialogHeader>
          <DialogBody className="pb-4">
            {detected && detected.length === 0 ? (
              <EmptyState compact title="No supported CLIs found" description="Install Claude Code or Codex, or add an API provider instead." />
            ) : (
              <div className="surface divide-y overflow-hidden">
                {(detected ?? []).map((c) => {
                  const added = isAdded(c);
                  return (
                    <div key={`${c.kind}-${c.path}`} className="flex items-center gap-3 px-3 py-2 text-xs">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2 text-[13px] font-medium">
                          {PROVIDER_KIND_META[c.kind].label}
                          {c.version ? <Badge tone="neutral">{c.version}</Badge> : null}
                        </div>
                        <div className="mono truncate text-subtle">{c.path}</div>
                      </div>
                      {added ? (
                        <Badge tone="ok">Added</Badge>
                      ) : (
                        <Button
                          size="sm"
                          variant="primary"
                          onClick={() => {
                            setDetected(null);
                            setDialog({ preset: { kind: c.kind, command: c.command, name: PROVIDER_KIND_META[c.kind].label } });
                          }}
                        >
                          <Plus /> Add
                        </Button>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </DialogBody>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setDetected(null)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmDialog
        open={!!pendingDelete}
        onOpenChange={(o) => !o && setPendingDelete(null)}
        title={`Delete ${pendingDelete?.name}?`}
        description="Its stored API key is removed from the keychain. Past runs keep their results."
        confirmLabel="Delete"
        destructive
        loading={remove.isPending}
        onConfirm={() => {
          if (pendingDelete) remove.mutate(pendingDelete.id);
        }}
      />
    </div>
  );
}

function ProbeStatus({ probe }: { probe: T.ProviderProbe | null }) {
  if (!probe) {
    return (
      <span className="flex items-center gap-1.5 text-xs text-subtle">
        <Dot tone="neutral" /> Not tested yet
      </span>
    );
  }
  return (
    <Tooltip content={`${probe.ok ? "Reachable" : "Unreachable"} · ${probe.latencyMs} ms${probe.version ? ` · ${probe.version}` : ""}${probe.message ? ` · ${probe.message}` : ""}`}>
      <span className={cn("flex min-w-0 items-center gap-1.5 text-xs", probe.ok ? "text-muted" : "text-danger")}>
        <Dot tone={probe.ok ? "ok" : "danger"} />
        <span className="truncate">
          {probe.ok ? "Reachable" : "Unreachable"}
          <span className="num"> · {probe.latencyMs} ms</span>
          {probe.version ? ` · ${probe.version}` : ""}
          {!probe.ok && probe.message ? ` · ${probe.message}` : ""}
          <span className="text-subtle"> · {relativeTime(probe.checkedAt)}</span>
        </span>
      </span>
    </Tooltip>
  );
}
