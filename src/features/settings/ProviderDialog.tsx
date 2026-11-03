// Add / edit a provider. The kind decides which fields are shown.

import * as React from "react";
import { toast } from "sonner";
import { KeyRound } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation } from "@/lib/api/hooks";
import { cn } from "@/lib/utils";
import { Badge, Button, Checkbox, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input } from "@/components/ui";
import { ErrorNote } from "@/components/app/common";
import { Segmented } from "./bits";
import { BASE_URL_PRESETS, PROVIDER_KINDS, PROVIDER_KIND_META, ROLES, ROLE_META, joinArgs, splitArgs } from "./shared";

export interface ProviderPreset {
  kind?: T.ProviderKind;
  name?: string;
  command?: string;
}

interface FormState {
  kind: T.ProviderKind;
  name: string;
  model: string;
  baseUrl: string;
  command: string;
  args: string;
  keySource: "keychain" | "env";
  apiKey: string;
  apiKeyEnv: string;
  roles: T.ProviderRole[];
  timeout: string;
  concurrency: string;
}

function initialState(provider: T.ProviderConfig | null | undefined, preset: ProviderPreset | null | undefined): FormState {
  if (provider) {
    return {
      kind: provider.kind,
      name: provider.name,
      model: provider.model ?? "",
      baseUrl: provider.baseUrl ?? "",
      command: provider.command ?? "",
      args: joinArgs(provider.args),
      keySource: provider.apiKeyEnv ? "env" : "keychain",
      apiKey: "",
      apiKeyEnv: provider.apiKeyEnv ?? "",
      roles: provider.roles,
      timeout: String(provider.timeoutSecs),
      concurrency: String(provider.maxConcurrency),
    };
  }
  const kind = preset?.kind ?? "claude_code";
  const meta = PROVIDER_KIND_META[kind];
  return {
    kind,
    name: preset?.name ?? meta.label,
    model: meta.defaultModel ?? "",
    baseUrl: meta.defaultBaseUrl ?? "",
    command: preset?.command ?? meta.defaultCommand ?? "",
    args: "",
    keySource: "keychain",
    apiKey: "",
    apiKeyEnv: "",
    roles: meta.defaultRoles,
    timeout: meta.family === "http" ? "120" : "600",
    concurrency: meta.family === "http" ? "4" : "2",
  };
}

export function ProviderDialog({ open, onOpenChange, provider, preset }: { open: boolean; onOpenChange: (o: boolean) => void; provider?: T.ProviderConfig | null; preset?: ProviderPreset | null }) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{provider ? `Edit ${provider.name}` : "Add provider"}</DialogTitle>
          <DialogDescription>Providers execute eval tasks, grade results and re-rank recommendations. API keys are stored in the OS keychain, never in the database.</DialogDescription>
        </DialogHeader>
        {open ? <ProviderForm provider={provider ?? null} preset={preset ?? null} onClose={() => onOpenChange(false)} /> : null}
      </DialogContent>
    </Dialog>
  );
}

function ProviderForm({ provider, preset, onClose }: { provider: T.ProviderConfig | null; preset: ProviderPreset | null; onClose: () => void }) {
  const [form, setForm] = React.useState<FormState>(() => initialState(provider, preset));
  const meta = PROVIDER_KIND_META[form.kind];
  const set = (patch: Partial<FormState>) => setForm((f) => ({ ...f, ...patch }));

  const changeKind = (kind: T.ProviderKind) => {
    if (kind === form.kind) return;
    const next = PROVIDER_KIND_META[kind];
    const prev = PROVIDER_KIND_META[form.kind];
    setForm((f) => ({
      ...f,
      kind,
      name: !f.name.trim() || f.name === prev.label ? next.label : f.name,
      command: next.family === "script" ? "" : (next.defaultCommand ?? ""),
      baseUrl: next.defaultBaseUrl ?? "",
      model: f.model && f.model !== (prev.defaultModel ?? "") ? f.model : (next.defaultModel ?? ""),
      roles: next.defaultRoles,
      timeout: next.family === "http" ? "120" : "600",
      concurrency: next.family === "http" ? "4" : "2",
    }));
  };

  const save = useApiMutation(
    (input: T.ProviderInput) => api.saveProvider(provider?.id ?? null, input),
    [keys.providers, keys.dashboard],
    (saved) => {
      toast.success(provider ? `Saved ${saved.name}` : `Added ${saved.name}`, { description: provider ? undefined : "Run Test to check it is reachable." });
      onClose();
    },
  );

  const timeout = Number(form.timeout);
  const concurrency = Number(form.concurrency);
  const problems: string[] = [];
  if (!form.name.trim()) problems.push("Name is required.");
  if (form.roles.length === 0) problems.push("Pick at least one role.");
  if (meta.family === "http" && !/^https?:\/\//.test(form.baseUrl.trim())) problems.push("Base URL must start with http:// or https://.");
  if (meta.family === "http" && form.kind === "openai_compatible" && !form.model.trim()) problems.push("Model is required for an OpenAI-compatible endpoint.");
  if (meta.family !== "http" && !form.command.trim()) problems.push(meta.family === "script" ? "Script path is required." : "Command is required.");
  if (form.keySource === "env" && !form.apiKeyEnv.trim()) problems.push("Name the environment variable that holds the key.");
  if (!Number.isFinite(timeout) || timeout < 10) problems.push("Timeout must be at least 10 seconds.");
  if (!Number.isFinite(concurrency) || concurrency < 1) problems.push("Concurrency must be at least 1.");

  const submit = () => {
    const input: T.ProviderInput = {
      name: form.name.trim(),
      kind: form.kind,
      model: form.model.trim() || null,
      baseUrl: meta.family === "http" ? form.baseUrl.trim() : null,
      command: meta.family === "http" ? null : form.command.trim() || null,
      args: meta.family === "http" ? [] : splitArgs(form.args),
      env: provider?.env ?? {},
      apiKeyEnv: meta.family === "http" && form.keySource === "env" ? form.apiKeyEnv.trim() : null,
      apiKey: meta.family === "http" && form.keySource === "keychain" && form.apiKey ? form.apiKey : null,
      roles: ROLES.filter((r) => form.roles.includes(r)),
      timeoutSecs: Math.round(timeout),
      maxConcurrency: Math.round(concurrency),
    };
    save.mutate(input);
  };

  return (
    <>
      <DialogBody className="flex flex-col gap-4 pb-4">
        <Field label="Kind">
          {provider ? (
            <div className="flex items-center gap-2 text-xs text-muted">
              <Badge tone="outline">{meta.label}</Badge>
              {meta.description}
            </div>
          ) : (
            <div className="grid gap-1.5 sm:grid-cols-2" role="radiogroup" aria-label="Provider kind">
              {PROVIDER_KINDS.map((k) => {
                const m = PROVIDER_KIND_META[k];
                const active = form.kind === k;
                return (
                  <button
                    key={k}
                    type="button"
                    role="radio"
                    aria-checked={active}
                    onClick={() => changeKind(k)}
                    className={cn("flex flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-left transition-colors", active ? "bg-accent-soft/70 hairline" : "bg-sunken hover:bg-line/60")}
                  >
                    <span className="text-[13px] font-medium">{m.label}</span>
                    <span className="text-[11px] leading-4 text-muted">{m.description}</span>
                  </button>
                );
              })}
            </div>
          )}
        </Field>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Name">
            <Input value={form.name} onChange={(e) => set({ name: e.target.value })} placeholder={meta.label} aria-label="Provider name" autoFocus />
          </Field>
          <Field label={meta.family === "cli" ? "Model (optional)" : "Model"} hint={meta.family === "cli" ? "Passed to the CLI as --model; blank uses its default." : meta.family === "script" ? "Informational; shown in reports." : "Model id sent with every request."}>
            <Input value={form.model} onChange={(e) => set({ model: e.target.value })} placeholder={form.kind === "openai_compatible" ? "gpt-4.1-mini, llama3.1, …" : form.kind === "anthropic" ? "claude-sonnet-4-5" : "default"} aria-label="Model" className="mono" />
          </Field>
        </div>

        {meta.family === "http" ? (
          <>
            <Field
              label="Base URL"
              hint={
                form.kind === "openai_compatible" ? (
                  <span className="flex flex-wrap items-center gap-1">
                    Presets:
                    {BASE_URL_PRESETS.map((p) => (
                      <Button key={p.label} variant="link" size="sm" onClick={() => set({ baseUrl: p.url })}>
                        {p.label}
                      </Button>
                    ))}
                  </span>
                ) : (
                  "Change only for a proxy or gateway."
                )
              }
            >
              <Input value={form.baseUrl} onChange={(e) => set({ baseUrl: e.target.value })} placeholder={meta.defaultBaseUrl ?? "https://"} aria-label="Base URL" className="mono" spellCheck={false} />
            </Field>
            <div className="flex flex-col gap-2 rounded-lg bg-sunken p-3">
              <div className="flex items-center justify-between gap-3">
                <span className="flex items-center gap-1.5 text-xs font-medium">
                  <KeyRound className="size-3.5 text-muted" /> API key
                </span>
                <Segmented<"keychain" | "env">
                  ariaLabel="API key source"
                  value={form.keySource}
                  onChange={(v) => set({ keySource: v, apiKeyEnv: v === "env" && !form.apiKeyEnv ? (meta.envVar ?? "") : form.apiKeyEnv })}
                  options={[
                    { value: "keychain", label: "Store in keychain" },
                    { value: "env", label: "Read from env var" },
                  ]}
                />
              </div>
              {form.keySource === "keychain" ? (
                <Field label="Key" hint={provider?.hasSecret ? "A key is stored in the OS keychain. Leave blank to keep it." : "Stored in the OS keychain (macOS Keychain, Windows Credential Manager, Secret Service)."}>
                  <Input type="password" value={form.apiKey} onChange={(e) => set({ apiKey: e.target.value })} placeholder={provider?.hasSecret ? "•••••••••••• (unchanged)" : "sk-…"} aria-label="API key" autoComplete="off" className="mono" />
                </Field>
              ) : (
                <Field label="Environment variable" hint="Read from the app's environment at call time; nothing is stored.">
                  <Input value={form.apiKeyEnv} onChange={(e) => set({ apiKeyEnv: e.target.value })} placeholder={meta.envVar ?? "API_KEY"} aria-label="API key environment variable" className="mono" spellCheck={false} />
                </Field>
              )}
            </div>
          </>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label={meta.family === "script" ? "Script path" : "Command"} hint={meta.family === "script" ? "Executable run inside each sandbox; task on stdin, answer on stdout." : "Executable name or absolute path."}>
              <Input value={form.command} onChange={(e) => set({ command: e.target.value })} placeholder={meta.family === "script" ? "/Users/you/bin/my-agent" : (meta.defaultCommand ?? "")} aria-label={meta.family === "script" ? "Script path" : "Command"} className="mono" spellCheck={false} />
            </Field>
            <Field label="Extra arguments" hint="Space separated; quote arguments with spaces.">
              <Input value={form.args} onChange={(e) => set({ args: e.target.value })} placeholder={form.kind === "claude_code" ? "--permission-mode acceptEdits" : ""} aria-label="Arguments" className="mono" spellCheck={false} />
            </Field>
          </div>
        )}

        <Field label="Roles" hint="What this provider may be used for.">
          <div className="grid gap-1.5 sm:grid-cols-3">
            {ROLES.map((r) => {
              const checked = form.roles.includes(r);
              return (
                <label key={r} className={cn("flex cursor-default items-start gap-2 rounded-lg px-2.5 py-2 text-xs transition-colors", checked ? "bg-accent-soft/60 hairline" : "bg-sunken")}>
                  <Checkbox checked={checked} onCheckedChange={(v) => set({ roles: v ? [...form.roles, r] : form.roles.filter((x) => x !== r) })} aria-label={ROLE_META[r].label} className="mt-0.5" />
                  <span>
                    <span className="block font-medium">{ROLE_META[r].label}</span>
                    <span className="text-muted">{ROLE_META[r].description}</span>
                  </span>
                </label>
              );
            })}
          </div>
        </Field>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Timeout (seconds)" hint="Per call; eval runs also have their own per-case timeout.">
            <Input type="number" min={10} step={10} value={form.timeout} onChange={(e) => set({ timeout: e.target.value })} aria-label="Timeout seconds" />
          </Field>
          <Field label="Max concurrency" hint="Parallel calls to this provider.">
            <Input type="number" min={1} max={32} value={form.concurrency} onChange={(e) => set({ concurrency: e.target.value })} aria-label="Max concurrency" />
          </Field>
        </div>

        {problems.length ? <ul className="list-disc pl-4 text-xs text-warn">{problems.map((p) => <li key={p}>{p}</li>)}</ul> : null}
        <ErrorNote error={save.error} />
      </DialogBody>
      <DialogFooter>
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" disabled={problems.length > 0} loading={save.isPending} onClick={submit}>
          {provider ? "Save changes" : "Add provider"}
        </Button>
      </DialogFooter>
    </>
  );
}
