// "Merge skills" dialog: a model drafts one SKILL.md out of two or more skills, the user reviews
// and adjusts the draft, and only then is anything written to the library.

import * as React from "react";
import { Link } from "react-router-dom";
import { toast } from "sonner";
import { AlertTriangle, ArrowLeft, Info, Rocket, Sparkles, XCircle } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation, useItems, useProviders } from "@/lib/api/hooks";
import { useUi } from "@/lib/store";
import { cn, formatBytes, formatDuration, formatTokens, pluralize } from "@/lib/utils";
import { Badge, Button, Checkbox, Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Field, Input, Markdown, Progress, SimpleSelect, Skeleton, Tabs, TabsContent, TabsList, TabsTrigger, Textarea } from "@/components/ui";
import { ErrorNote, InlineCode, KindBadge, TagChip } from "@/components/app/common";
import { ALL_ITEMS_QUERY } from "@/features/agents/queries";
import { PROVIDER_KIND_META } from "@/features/settings/shared";

const NAME_RE = /^[a-z0-9]+(-[a-z0-9]+)*$/;
const nameValid = (name: string) => NAME_RE.test(name) && name.length <= 64;
const descriptionValid = (d: string) => d.trim().length >= 20 && d.length <= 1024;

export function MergeSkillsDialog() {
  const ids = useUi((s) => s.mergeIds);
  const openMerge = useUi((s) => s.openMerge);
  return (
    <Dialog open={!!ids} onOpenChange={(o) => !o && openMerge(null)}>
      <DialogContent size="xl">
        {/* Keyed on the ids so a different selection starts from a clean setup step. */}
        {ids ? <MergeFlow key={ids.join("|")} ids={ids} onClose={() => openMerge(null)} /> : null}
      </DialogContent>
    </Dialog>
  );
}

/** What the user can change about the proposal before applying it. */
interface Draft {
  name: string;
  description: string;
  body: string;
  files: T.MergeFile[];
  retireSources: boolean;
  carryEvals: boolean;
  /** Fields edited since the proposal arrived; a validation error on one of them no longer blocks. */
  edited: Set<string>;
}

function MergeFlow({ ids, onClose }: { ids: string[]; onClose: () => void }) {
  const openItem = useUi((s) => s.openItem);
  const clearSelection = useUi((s) => s.clearSelection);
  const { data: allItems, isLoading: itemsLoading } = useItems(ALL_ITEMS_QUERY);
  const { data: providers } = useProviders();

  const [step, setStep] = React.useState<"setup" | "review">("setup");
  const [name, setName] = React.useState("");
  const [guidance, setGuidance] = React.useState("");
  const [providerId, setProviderId] = React.useState<string | null>(null);
  const [retireSources, setRetireSources] = React.useState(true);
  const [carryEvals, setCarryEvals] = React.useState(true);
  const [proposal, setProposal] = React.useState<T.MergeProposal | null>(null);
  const [draft, setDraft] = React.useState<Draft | null>(null);

  const sources = React.useMemo(() => ids.map((id) => ({ id, item: allItems?.find((i) => i.id === id) ?? null })), [ids, allItems]);
  const skills = sources.filter((s) => s.item?.kind === "skill").map((s) => s.item!);
  const sourcesOk = !itemsLoading && sources.every((s) => s.item?.kind === "skill") && skills.length >= 2;

  const list = providers ?? [];
  const defaultProvider = list.find((p) => p.roles.includes("recommender")) ?? list.find((p) => p.roles.includes("grader")) ?? list[0] ?? null;
  const provider = list.find((p) => p.id === providerId) ?? defaultProvider;

  const propose = useApiMutation(
    (request: T.MergeRequest) => api.proposeMerge(request),
    [],
    (p) => {
      setProposal(p);
      setDraft({ name: p.name, description: p.description, body: p.body, files: p.files, retireSources, carryEvals, edited: new Set() });
      setStep("review");
    },
  );

  const apply = useApiMutation(
    (commit: T.MergeCommit) => api.applyMerge(commit),
    [keys.itemsAll, keys.stats, keys.dashboard, keys.deployments, keys.targets, ["item"], ["analyze"]],
    (result) => {
      toast.success(`Merged into ${result.item.name}${result.deployed.length ? `, ${pluralize(result.deployed.length, "deployment")} moved` : ""}`);
      // The merged skill is written even when a later step failed: say what did not happen.
      const problems = [...result.failed.map((f) => `Deployment not moved: ${f.reason}`), ...result.warnings];
      if (problems.length) {
        toast.warning(`${pluralize(problems.length, "step")} did not complete`, { description: problems.join("\n"), duration: 15_000 });
      }
      clearSelection();
      onClose();
      openItem(result.item.id);
    },
  );

  const setupNameOk = name === "" || nameValid(name);
  const canDraft = sourcesOk && setupNameOk && !!provider && !propose.isPending;

  const submitProposal = () => {
    if (!provider) return;
    propose.mutate({ itemIds: skills.map((s) => s.id), name: name || null, guidance: guidance.trim() || null, providerId: provider.id });
  };

  if (step === "review" && proposal && draft) {
    return (
      <ReviewStep
        proposal={proposal}
        draft={draft}
        setDraft={setDraft}
        pending={apply.isPending}
        error={apply.error}
        onBack={() => setStep("setup")}
        onClose={onClose}
        onApply={() =>
          apply.mutate({
            sourceIds: proposal.sources.map((s) => s.item.id),
            name: draft.name,
            description: draft.description,
            body: draft.body,
            frontmatter: proposal.frontmatter,
            files: draft.files,
            tags: proposal.tags,
            retireSources: draft.retireSources,
            carryEvals: draft.carryEvals,
          })
        }
      />
    );
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>Merge skills</DialogTitle>
        <DialogDescription>A model reads the skills and writes one SKILL.md that keeps everything distinct. Nothing changes until you accept the draft.</DialogDescription>
      </DialogHeader>
      <DialogBody className="flex flex-col gap-5 pb-4">
        <section className="flex flex-col gap-2">
          <div className="text-[11px] font-medium uppercase tracking-wide text-subtle">Sources ({sources.length})</div>
          <ul className="surface divide-y overflow-hidden">
            {sources.map(({ id, item }) => (
              <li key={id} className="flex items-center gap-3 px-3 py-2 text-xs">
                {item ? (
                  <>
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-1.5">
                        <span className="mono text-[13px] font-medium">{item.name}</span>
                        {item.kind !== "skill" ? <KindBadge kind={item.kind} /> : null}
                        <span className="num text-subtle">~{formatTokens(item.tokenEstimate)} tokens</span>
                        {item.deploymentCount > 0 ? (
                          <Badge tone="ok">
                            <Rocket /> {pluralize(item.deploymentCount, "deployment")}
                          </Badge>
                        ) : null}
                      </div>
                      <p className="truncate text-muted">{item.description}</p>
                      {item.kind !== "skill" ? (
                        <p className="mt-1 flex items-center gap-1 text-warn">
                          <AlertTriangle className="size-3" /> Only skills can be merged; remove this {item.kind} from the selection.
                        </p>
                      ) : null}
                    </div>
                  </>
                ) : itemsLoading ? (
                  <Skeleton className="h-8 w-full" />
                ) : (
                  <p className="flex items-center gap-1 text-warn">
                    <AlertTriangle className="size-3" /> This item is no longer in the library.
                  </p>
                )}
              </li>
            ))}
          </ul>
          {!itemsLoading && skills.length < 2 && sources.every((s) => s.item?.kind === "skill") ? <p className="text-xs text-warn">Pick at least two skills to merge.</p> : null}
        </section>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field label="Name (optional)" hint={!setupNameOk ? "Lowercase letters, digits and single hyphens, up to 64 characters" : "Leave empty to let the model propose one. Use a source's name to replace that skill in place."}>
            <Input value={name} onChange={(e) => setName(e.target.value)} placeholder={skills[0] ? skills[0].name : "merged-skill"} className="mono" aria-label="Name" spellCheck={false} />
          </Field>
          <Field
            label="Provider"
            hint={
              list.length === 0 ? (
                <span className="flex flex-wrap items-center gap-1 text-warn">
                  <AlertTriangle className="size-3" /> No provider can draft the merge.
                  <Link to="/settings/providers" onClick={onClose} className="text-info underline-offset-4 hover:underline">
                    Add a provider
                  </Link>
                </span>
              ) : (
                "The model that reads the sources and writes the draft."
              )
            }
          >
            <SimpleSelect
              value={provider?.id ?? ""}
              onChange={setProviderId}
              options={list.map((p) => ({ value: p.id, label: `${p.name} · ${PROVIDER_KIND_META[p.kind].label}` }))}
              placeholder={list.length ? "Choose a provider…" : "No providers"}
              disabled={list.length === 0}
              aria-label="Provider"
            />
          </Field>
        </div>
        <Field label="Guidance (optional)" hint="Anything the model should favour or keep; it reads the sources either way.">
          <Textarea value={guidance} onChange={(e) => setGuidance(e.target.value)} rows={2} placeholder="Keep the python workflow; prefer the shorter wording…" aria-label="Guidance" />
        </Field>

        <div className="flex flex-col gap-2">
          <CheckboxRow checked={retireSources} onChange={setRetireSources} label="Retire the originals and move their deployments to the merged skill" />
          <CheckboxRow checked={carryEvals} onChange={setCarryEvals} label="Carry their evaluation cases over" />
        </div>

        {propose.isPending ? (
          <div className="flex flex-col gap-2 rounded-lg bg-sunken px-3 py-2.5 text-xs">
            <span className="text-muted">
              Asking {provider?.name} to combine {skills.length} skills… this can take a minute.
            </span>
            <Progress value={0} indeterminate size="sm" aria-label="Drafting the merged skill" />
          </div>
        ) : null}
        <ErrorNote error={propose.error} />
      </DialogBody>
      <DialogFooter>
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" disabled={!canDraft} loading={propose.isPending} onClick={submitProposal}>
          <Sparkles /> Draft merged skill
        </Button>
      </DialogFooter>
    </>
  );
}

function ReviewStep({
  proposal,
  draft,
  setDraft,
  pending,
  error,
  onBack,
  onClose,
  onApply,
}: {
  proposal: T.MergeProposal;
  draft: Draft;
  setDraft: React.Dispatch<React.SetStateAction<Draft | null>>;
  pending: boolean;
  error: unknown;
  onBack: () => void;
  onClose: () => void;
  onApply: () => void;
}) {
  const [bodyTab, setBodyTab] = React.useState("edit");
  const set = (patch: Partial<Draft>, field?: string) =>
    setDraft((d) => (d ? { ...d, ...patch, edited: field ? new Set(d.edited).add(field) : d.edited } : d));

  const replaces = proposal.sources.find((s) => s.item.name === draft.name) ?? null;
  const nameOk = nameValid(draft.name);
  const descOk = descriptionValid(draft.description);
  const bodyOk = draft.body.trim().length > 0;
  const findings = proposal.validation.findings;
  // A finding blocks until the field it is about is edited; `body.*` findings carry no field,
  // so they map to the body. Anything else is re-checked by the backend on apply.
  const fieldOf = (f: T.Finding) => f.field ?? (f.code.startsWith("body") ? "body" : null);
  const blockingError = findings.some((f) => {
    if (f.severity !== "error") return false;
    const field = fieldOf(f);
    return !!field && !draft.edited.has(field);
  });
  const canApply = nameOk && descOk && bodyOk && !blockingError && !pending;

  const stats = [
    `Instructions ${formatTokens(proposal.bodyTokensBefore)} → ${formatTokens(proposal.bodyTokensAfter)} tokens`,
    `Description ${proposal.descriptionTokensBefore} → ${proposal.descriptionTokensAfter} tokens`,
    ...(proposal.evalCases > 0 ? [`${pluralize(proposal.evalCases, "eval case")} carried`] : []),
    `${proposal.providerName}${proposal.model ? ` · ${proposal.model}` : ""} · ${formatTokens(proposal.inputTokens)} in / ${formatTokens(proposal.outputTokens)} out · ${formatDuration(proposal.latencyMs)}`,
  ];

  return (
    <>
      <DialogHeader>
        <DialogTitle>Review the merged skill</DialogTitle>
        <DialogDescription>
          Drafted from {proposal.sources.map((s) => s.item.name).join(", ")}. Adjust anything below; the sources are untouched until you create it.
        </DialogDescription>
      </DialogHeader>
      <DialogBody className="flex flex-col gap-4 pb-4">
        <ul className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted">
          {stats.map((s) => (
            <li key={s} className="num">
              {s}
            </li>
          ))}
        </ul>

        {proposal.notes ? (
          <div className="flex items-start gap-2 rounded-lg bg-info-soft px-3 py-2 text-xs text-info">
            <Sparkles className="mt-0.5 size-3.5 shrink-0" />
            <span className="selectable">{proposal.notes}</span>
          </div>
        ) : null}
        {proposal.warnings.map((w, i) => (
          <div key={i} className="flex items-start gap-2 rounded-lg bg-warn-soft px-3 py-2 text-xs text-warn">
            <AlertTriangle className="mt-0.5 size-3.5 shrink-0" />
            <span className="selectable">{w}</span>
          </div>
        ))}
        {proposal.dropped.length ? (
          <div className="rounded-lg bg-warn-soft px-3 py-2 text-xs text-warn">
            <div className="flex items-center gap-2 font-medium">
              <AlertTriangle className="size-3.5 shrink-0" /> Left out
            </div>
            <ul className="mt-1 list-disc pl-5 selectable">
              {proposal.dropped.map((d, i) => (
                <li key={i}>{d}</li>
              ))}
            </ul>
          </div>
        ) : null}
        {findings.length ? (
          <div className="flex flex-col gap-1.5">
            {findings.map((f, i) => (
              <div key={i} className={cn("flex items-start gap-2 rounded-lg px-3 py-2 text-xs", f.severity === "error" ? "bg-danger-soft text-danger" : f.severity === "warning" ? "bg-warn-soft text-warn" : "bg-info-soft text-info")}>
                {f.severity === "error" ? <XCircle className="mt-0.5 size-3.5 shrink-0" /> : f.severity === "warning" ? <AlertTriangle className="mt-0.5 size-3.5 shrink-0" /> : <Info className="mt-0.5 size-3.5 shrink-0" />}
                <div>
                  <div className="font-medium text-fg">
                    {f.message}
                    {f.field ? <InlineCode className="ml-1.5">{f.field}</InlineCode> : null}
                  </div>
                  <div className="mono text-[11px] text-muted">{f.code}</div>
                </div>
              </div>
            ))}
          </div>
        ) : null}

        <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_minmax(0,2fr)]">
          <Field
            label="Name"
            hint={
              !nameOk ? (
                "Lowercase letters, digits and single hyphens, up to 64 characters"
              ) : replaces ? (
                <span className="text-warn">
                  Replaces {replaces.item.name} in place: it keeps its id, deployments and run history{draft.carryEvals ? "; its evaluation suite becomes the union of the sources'" : " and its own evaluation suite"}.
                </span>
              ) : (
                "Also the directory name. Use a source's name to replace that skill in place."
              )
            }
          >
            <Input value={draft.name} onChange={(e) => set({ name: e.target.value }, "name")} className="mono" aria-label="Name" spellCheck={false} />
          </Field>
          <Field label={`Description (${draft.description.length}/1024)`} hint={descOk ? "Agents decide whether to load the skill from this sentence alone." : "Between 20 and 1024 characters."}>
            <Textarea value={draft.description} onChange={(e) => set({ description: e.target.value }, "description")} rows={3} aria-label="Description" />
          </Field>
        </div>

        <Tabs value={bodyTab} onValueChange={setBodyTab} className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-muted">Instructions (markdown)</span>
            <TabsList aria-label="Instructions view">
              <TabsTrigger value="edit">Edit</TabsTrigger>
              <TabsTrigger value="preview">Preview</TabsTrigger>
            </TabsList>
          </div>
          <TabsContent value="edit" className="mt-0">
            <Textarea value={draft.body} onChange={(e) => set({ body: e.target.value }, "body")} rows={16} className="mono text-xs leading-5" spellCheck={false} aria-label="Instructions" />
          </TabsContent>
          <TabsContent value="preview" className="mt-0">
            <div className="max-h-[420px] overflow-y-auto rounded-md bg-sunken px-4 py-3">{draft.body.trim() ? <Markdown>{draft.body}</Markdown> : <p className="text-xs text-muted">Nothing to preview.</p>}</div>
          </TabsContent>
        </Tabs>

        {draft.files.length ? (
          <section className="flex flex-col gap-1.5">
            <div className="text-[11px] font-medium uppercase tracking-wide text-subtle">Bundled files</div>
            <div className="surface divide-y overflow-hidden">
              {draft.files.map((f, i) => (
                <label key={`${f.fromItemId}:${f.path}`} className="flex cursor-default items-center gap-3 px-3 py-1.5 text-xs hover:bg-sunken/60">
                  <Checkbox checked={f.keep} onCheckedChange={(v) => set({ files: draft.files.map((x, j) => (j === i ? { ...x, keep: v === true } : x)) }, "files")} aria-label={`Keep ${f.dest}`} />
                  <span className="mono min-w-0 flex-1 truncate">{f.dest}</span>
                  <Badge tone="outline">{f.fromName}</Badge>
                  <span className="num text-subtle">{formatBytes(f.sizeBytes)}</span>
                </label>
              ))}
            </div>
            <p className="text-xs text-subtle">Files the body refers to stay checked.</p>
          </section>
        ) : null}

        {proposal.tags.length ? (
          <section className="flex flex-col gap-1.5">
            <div className="text-[11px] font-medium uppercase tracking-wide text-subtle">Tags</div>
            <div className="flex flex-wrap gap-1.5">
              {proposal.tags.map((t) => (
                <TagChip key={t} tag={t} />
              ))}
            </div>
          </section>
        ) : null}

        <div className="flex flex-col gap-2">
          <CheckboxRow checked={draft.retireSources} onChange={(v) => set({ retireSources: v })} label="Retire the originals and move their deployments to the merged skill" />
          <CheckboxRow checked={draft.carryEvals} onChange={(v) => set({ carryEvals: v })} label="Carry their evaluation cases over" />
        </div>
        <ErrorNote error={error} />
      </DialogBody>
      <DialogFooter>
        <Button variant="ghost" onClick={onBack} className="mr-auto">
          <ArrowLeft /> Back
        </Button>
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button variant="primary" disabled={!canApply} loading={pending} onClick={onApply}>
          {replaces ? `Replace ${replaces.item.name}` : "Create merged skill"}
        </Button>
      </DialogFooter>
    </>
  );
}

function CheckboxRow({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <label className="flex cursor-default items-center gap-2 text-xs">
      <Checkbox checked={checked} onCheckedChange={(v) => onChange(v === true)} aria-label={label} />
      <span className="text-muted">{label}</span>
    </label>
  );
}
