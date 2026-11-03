// Editor for a skill's eval suite: cases with expectations and deterministic checks, plus trigger
// queries. The draft lives here; dirtiness is derived by comparing JSON with the stored suite.

import * as React from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Check, ChevronDown, ChevronRight, Copy, FlaskConical, FolderOpen, Plus, Trash2, Wand2, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation } from "@/lib/api/hooks";
import { cn, displayPath, pluralize } from "@/lib/utils";
import { Button, Dot, EmptyState, Field, Input, Label, SectionTitle, SimpleSelect, Switch, Textarea, Tooltip, revealPath } from "@/components/ui";
import { ErrorNote, InlineCode } from "@/components/app/common";
import { CHECK_KINDS, CHECK_KIND_LABEL, joinArgs, splitArgs } from "./shared";

const NONE = "__none__";
const SLUG = /^[a-z0-9]+(-[a-z0-9]+)*$/;

function blankCase(id: number, name = `case-${id}`): T.EvalCase {
  return { id, name, prompt: "", expectedOutput: null, files: [], expectations: [""], checks: [] };
}

function nextCaseId(cases: T.EvalCase[]): number {
  return cases.reduce((m, c) => Math.max(m, c.id), 0) + 1;
}

function newCheck(cases: T.EvalCase[]): T.Check {
  const n = cases.reduce((m, c) => m + c.checks.length, 0) + 1;
  return { id: `check-${n}`, expectation: null, kind: "file_exists", path: "" };
}

/** Switch a check to another kind, carrying over the fields that still apply. */
function convertCheck(check: T.Check, kind: T.CheckKind["kind"]): T.Check {
  const prev = check as unknown as Record<string, unknown>;
  const str = (k: string) => (typeof prev[k] === "string" ? (prev[k] as string) : "");
  const base = { id: check.id, expectation: check.expectation ?? null };
  switch (kind) {
    case "file_exists":
      return { ...base, kind, path: str("path") };
    case "file_contains":
      return { ...base, kind, path: str("path"), text: str("text") };
    case "file_matches":
      return { ...base, kind, path: str("path"), pattern: str("pattern") };
    case "output_contains":
      return { ...base, kind, text: str("text") };
    case "output_matches":
      return { ...base, kind, pattern: str("pattern") };
    case "json_pointer":
      return { ...base, kind, path: str("path"), pointer: str("pointer"), equals: prev.equals ?? "" };
    case "command":
      return { ...base, kind, command: str("command"), args: Array.isArray(prev.args) ? (prev.args as string[]) : [] };
  }
}

function validate(suite: T.EvalSuite): string[] {
  const errors: string[] = [];
  const names = new Set<string>();
  for (const c of suite.cases) {
    if (!SLUG.test(c.name)) errors.push(`Case ${c.id}: name must be a slug (lowercase letters, digits, hyphens)`);
    if (names.has(c.name)) errors.push(`Case name "${c.name}" is used twice`);
    names.add(c.name);
    if (!c.prompt.trim()) errors.push(`${c.name}: prompt is empty`);
    if (c.expectations.filter((e) => e.trim()).length === 0 && c.checks.length === 0) errors.push(`${c.name}: add at least one expectation or check`);
  }
  for (const [i, t] of suite.triggerCases.entries()) if (!t.query.trim()) errors.push(`Trigger case ${i + 1}: query is empty`);
  return errors;
}

/** Trim blanks before saving; what is sent is what skill-creator expects. */
function normalize(suite: T.EvalSuite): T.EvalSuite {
  return {
    ...suite,
    cases: suite.cases.map((c) => ({
      ...c,
      name: c.name.trim(),
      prompt: c.prompt.trim(),
      expectedOutput: c.expectedOutput?.trim() ? c.expectedOutput.trim() : null,
      files: c.files.map((f) => f.trim()).filter(Boolean),
      expectations: c.expectations.map((e) => e.trim()).filter(Boolean),
    })),
    triggerCases: suite.triggerCases.map((t) => ({ ...t, query: t.query.trim() })),
  };
}

export function SuiteEditor({ suite, className }: { suite: T.EvalSuite; className?: string }) {
  const qc = useQueryClient();
  const [draft, setDraft] = React.useState<T.EvalSuite>(suite);
  const [expanded, setExpanded] = React.useState<Set<number>>(() => new Set(suite.cases.slice(0, 1).map((c) => c.id)));
  const dirty = JSON.stringify(draft) !== JSON.stringify(suite);
  const errors = React.useMemo(() => validate(draft), [draft]);
  const empty = !suite.exists && draft.cases.length === 0 && draft.triggerCases.length === 0;

  const save = useApiMutation(
    (s: T.EvalSuite) => api.saveEvalSuite(normalize(s)),
    [keys.itemsAll, keys.dashboard],
    (saved) => {
      qc.setQueryData(keys.suite(saved.itemId), saved);
      setDraft(saved);
      toast.success("Suite saved", { description: `${pluralize(saved.cases.length, "case")} · ${pluralize(saved.triggerCases.length, "trigger query", "trigger queries")}` });
    },
  );
  const draftSuite = useApiMutation(
    () => api.draftEvalSuite(suite.itemId),
    [],
    (generated) => {
      setDraft({ ...generated, exists: suite.exists });
      setExpanded(new Set(generated.cases.slice(0, 1).map((c) => c.id)));
      toast("Drafted from the skill description", { description: "Review the prompts and expectations before running." });
    },
  );

  const update = (fn: (s: T.EvalSuite) => T.EvalSuite) => setDraft((s) => fn(s));
  const updateCase = (id: number, fn: (c: T.EvalCase) => T.EvalCase) => update((s) => ({ ...s, cases: s.cases.map((c) => (c.id === id ? fn(c) : c)) }));
  const toggle = (id: number) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const addCase = () => {
    const id = nextCaseId(draft.cases);
    update((s) => ({ ...s, cases: [...s.cases, blankCase(id)] }));
    setExpanded((prev) => new Set([...prev, id]));
  };
  const duplicateCase = (c: T.EvalCase) => {
    const id = nextCaseId(draft.cases);
    const copy: T.EvalCase = { ...structuredClone(c), id, name: `${c.name}-copy` };
    update((s) => {
      const idx = s.cases.findIndex((x) => x.id === c.id);
      const cases = [...s.cases];
      cases.splice(idx + 1, 0, copy);
      return { ...s, cases };
    });
    setExpanded((prev) => new Set([...prev, id]));
  };
  const removeCase = (id: number) => update((s) => ({ ...s, cases: s.cases.filter((c) => c.id !== id) }));

  if (empty) {
    return (
      <div className={cn("flex h-full flex-col", className)}>
        <div className="flex-1 px-6 py-6">
          <EmptyState
            icon={FlaskConical}
            title="No eval suite yet"
            description="A suite is a handful of realistic task prompts with expectations. Skill Hive runs each prompt with and without the skill, grades the outputs, and tells you whether the instructions help."
            action={
              <>
                <Button variant="primary" loading={draftSuite.isPending} onClick={() => draftSuite.mutate(undefined)}>
                  <Wand2 /> Draft a suite
                </Button>
                <Button onClick={addCase}>
                  <Plus /> Start empty
                </Button>
              </>
            }
          />
          <p className="mt-3 text-center text-xs text-subtle">
            Drafting proposes two cases and four trigger queries from the skill description. Saved as <InlineCode>evals/evals.json</InlineCode> (skill-creator compatible) plus <InlineCode>checks.json</InlineCode> and <InlineCode>trigger.json</InlineCode>.
          </p>
          <ErrorNote error={draftSuite.error} className="mx-auto mt-3 max-w-md" />
        </div>
      </div>
    );
  }

  return (
    <div className={cn("flex h-full min-h-0 flex-col", className)}>
      <div className="min-h-0 flex-1 scroll-y px-6 pb-6">
        <div className="flex flex-col gap-6 pt-4">
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted">
            <span>
              Stored in <InlineCode>{displayPath(suite.path)}</InlineCode> as <InlineCode>evals.json</InlineCode>, <InlineCode>checks.json</InlineCode> and <InlineCode>trigger.json</InlineCode>.
            </span>
            {suite.exists ? (
              <Button variant="link" size="sm" onClick={() => void revealPath(suite.path)}>
                <FolderOpen /> Reveal
              </Button>
            ) : (
              <span className="text-subtle">Not saved yet.</span>
            )}
            {!suite.exists ? (
              <Button variant="link" size="sm" loading={draftSuite.isPending} onClick={() => draftSuite.mutate(undefined)}>
                <Wand2 /> Replace with a draft
              </Button>
            ) : null}
          </div>

          <section className="flex flex-col gap-3">
            <SectionTitle
              title={`Cases (${draft.cases.length})`}
              description="Each case is one task prompt. It runs in every arm, with repeats, and is graded against its expectations."
              action={
                <Button size="sm" onClick={addCase}>
                  <Plus /> Add case
                </Button>
              }
            />
            {draft.cases.length === 0 ? (
              <div className="rounded-lg border border-dashed px-3 py-6 text-center text-xs text-muted">No cases. Add one to have something to run.</div>
            ) : (
              draft.cases.map((c) => (
                <CaseCard
                  key={c.id}
                  c={c}
                  open={expanded.has(c.id)}
                  onToggle={() => toggle(c.id)}
                  onChange={(fn) => updateCase(c.id, fn)}
                  onDuplicate={() => duplicateCase(c)}
                  onRemove={() => removeCase(c.id)}
                  newCheck={() => newCheck(draft.cases)}
                  nameError={!SLUG.test(c.name) ? "Use lowercase letters, digits and hyphens" : draft.cases.some((o) => o.id !== c.id && o.name === c.name) ? "Another case has this name" : null}
                />
              ))
            )}
          </section>

          <section className="flex flex-col gap-3">
            <SectionTitle
              title={`Trigger queries (${draft.triggerCases.length})`}
              description="Does the description make the agent pick this skill? Positive queries should trigger it, negative ones should not; both are needed for precision and recall."
              action={
                <Button size="sm" onClick={() => update((s) => ({ ...s, triggerCases: [...s.triggerCases, { query: "", shouldTrigger: true }] }))}>
                  <Plus /> Add query
                </Button>
              }
            />
            {draft.triggerCases.length === 0 ? (
              <div className="rounded-lg border border-dashed px-3 py-6 text-center text-xs text-muted">No trigger queries. The trigger evaluation is skipped without them.</div>
            ) : (
              <div className="surface divide-y overflow-hidden">
                {draft.triggerCases.map((t, i) => (
                  <div key={i} className="flex items-center gap-3 px-3 py-2">
                    <Input
                      value={t.query}
                      onChange={(e) => update((s) => ({ ...s, triggerCases: s.triggerCases.map((x, j) => (j === i ? { ...x, query: e.target.value } : x)) }))}
                      placeholder="What a user might type…"
                      aria-label={`Trigger query ${i + 1}`}
                      className="flex-1"
                    />
                    <label className="flex shrink-0 items-center gap-2 text-xs text-muted">
                      <Switch
                        checked={t.shouldTrigger}
                        onCheckedChange={(v) => update((s) => ({ ...s, triggerCases: s.triggerCases.map((x, j) => (j === i ? { ...x, shouldTrigger: v } : x)) }))}
                        aria-label="Should trigger"
                      />
                      <span className={cn("w-24", t.shouldTrigger ? "text-ok" : "text-muted")}>{t.shouldTrigger ? "Should trigger" : "Should not"}</span>
                    </label>
                    <Tooltip content="Remove query">
                      <Button variant="ghost" size="icon-sm" aria-label="Remove trigger query" onClick={() => update((s) => ({ ...s, triggerCases: s.triggerCases.filter((_, j) => j !== i) }))}>
                        <X />
                      </Button>
                    </Tooltip>
                  </div>
                ))}
              </div>
            )}
          </section>
        </div>
      </div>

      <div className="flex items-center gap-3 border-t bg-elevated px-6 py-2.5">
        <span className="flex items-center gap-2 text-xs text-muted">
          {dirty ? (
            <>
              <Dot tone="warn" /> Unsaved changes
            </>
          ) : (
            <>
              <Check className="size-3.5 text-ok" /> All changes saved
            </>
          )}
        </span>
        {dirty && errors.length ? (
          <span className="truncate text-xs text-danger" title={errors.join("\n")}>
            {errors[0]}
            {errors.length > 1 ? ` (+${errors.length - 1} more)` : ""}
          </span>
        ) : null}
        <ErrorNote error={save.error} className="py-1" />
        <span className="ml-auto flex items-center gap-2">
          <Button variant="ghost" size="sm" disabled={!dirty || save.isPending} onClick={() => setDraft(suite)}>
            Discard
          </Button>
          <Button variant="primary" size="sm" disabled={!dirty || errors.length > 0} loading={save.isPending} onClick={() => save.mutate(draft)}>
            Save suite
          </Button>
        </span>
      </div>
    </div>
  );
}

function CaseCard({
  c,
  open,
  onToggle,
  onChange,
  onDuplicate,
  onRemove,
  newCheck,
  nameError,
}: {
  c: T.EvalCase;
  open: boolean;
  onToggle: () => void;
  onChange: (fn: (c: T.EvalCase) => T.EvalCase) => void;
  onDuplicate: () => void;
  onRemove: () => void;
  newCheck: () => T.Check;
  nameError: string | null;
}) {
  const expectationOptions = c.expectations.map((e) => e.trim()).filter(Boolean);
  return (
    <div className="surface overflow-hidden">
      <div className="flex items-center gap-2 px-3 py-2">
        <button type="button" onClick={onToggle} className="flex size-6 items-center justify-center rounded-md text-muted hover:bg-sunken hover:text-fg" aria-label={open ? "Collapse case" : "Expand case"} aria-expanded={open}>
          {open ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
        </button>
        <div className="flex min-w-0 flex-1 flex-col">
          <Input
            value={c.name}
            onChange={(e) => onChange((x) => ({ ...x, name: e.target.value }))}
            aria-label="Case name"
            placeholder="case-name"
            className={cn("mono h-7 max-w-xs text-xs", nameError && "ring-2 ring-danger/50")}
          />
          {nameError ? <span className="mt-0.5 text-[11px] text-danger">{nameError}</span> : null}
        </div>
        {!open ? (
          <span className="hidden truncate text-xs text-subtle md:inline">
            {pluralize(c.expectations.filter((e) => e.trim()).length, "expectation")} · {pluralize(c.checks.length, "check")}
            {c.files.length ? ` · ${pluralize(c.files.length, "file")}` : ""}
          </span>
        ) : null}
        <Tooltip content="Duplicate case">
          <Button variant="ghost" size="icon-sm" aria-label="Duplicate case" onClick={onDuplicate}>
            <Copy />
          </Button>
        </Tooltip>
        <Tooltip content="Remove case">
          <Button variant="ghost" size="icon-sm" aria-label="Remove case" onClick={onRemove}>
            <Trash2 />
          </Button>
        </Tooltip>
      </div>

      {open ? (
        <div className="grid gap-4 border-t px-4 py-4 lg:grid-cols-2">
          <Field label="Prompt" hint="What the agent is asked to do. The same prompt runs in every arm.">
            <Textarea value={c.prompt} onChange={(e) => onChange((x) => ({ ...x, prompt: e.target.value }))} rows={5} placeholder="Extract the line-item table from invoice.pdf into invoice.csv…" aria-label="Prompt" />
          </Field>
          <div className="flex flex-col gap-4">
            <Field label="Expected output" hint="Optional. Shown to the grader as the reference answer.">
              <Textarea value={c.expectedOutput ?? ""} onChange={(e) => onChange((x) => ({ ...x, expectedOutput: e.target.value }))} rows={2} placeholder="invoice.csv with 14 rows and a total of 1,284.50" aria-label="Expected output" />
            </Field>
            <Field label="Input files" hint="Relative to the skill root, one per line (or comma-separated). Copied into the sandbox before the run.">
              <ListTextarea key={c.files.join("\n")} value={c.files} onChange={(files) => onChange((x) => ({ ...x, files }))} placeholder={"evals/files/invoice.pdf"} aria-label="Input files" />
            </Field>
          </div>

          <div className="flex flex-col gap-1.5">
            <Label>Expectations</Label>
            <div className="flex flex-col gap-1.5">
              {c.expectations.map((e, i) => (
                <div key={i} className="flex items-center gap-1.5">
                  <Input
                    value={e}
                    onChange={(ev) => onChange((x) => ({ ...x, expectations: x.expectations.map((y, j) => (j === i ? ev.target.value : y)) }))}
                    onKeyDown={(ev) => {
                      if (ev.key === "Enter" && e.trim()) {
                        ev.preventDefault();
                        onChange((x) => {
                          const next = [...x.expectations];
                          next.splice(i + 1, 0, "");
                          return { ...x, expectations: next };
                        });
                      }
                    }}
                    placeholder="A CSV with the invoice line items exists"
                    aria-label={`Expectation ${i + 1}`}
                  />
                  <Button variant="ghost" size="icon-sm" aria-label="Remove expectation" onClick={() => onChange((x) => ({ ...x, expectations: x.expectations.filter((_, j) => j !== i) }))}>
                    <X />
                  </Button>
                </div>
              ))}
              <div>
                <Button variant="ghost" size="sm" onClick={() => onChange((x) => ({ ...x, expectations: [...x.expectations, ""] }))}>
                  <Plus /> Add expectation
                </Button>
              </div>
            </div>
            <p className="text-xs text-subtle">Plain statements the grader verifies against the transcript and outputs, citing evidence.</p>
          </div>

          <div className="flex flex-col gap-1.5">
            <Label>Deterministic checks</Label>
            <div className="flex flex-col gap-2">
              {c.checks.map((chk, i) => (
                <CheckEditor
                  key={chk.id}
                  check={chk}
                  expectations={expectationOptions}
                  onChange={(next) => onChange((x) => ({ ...x, checks: x.checks.map((y, j) => (j === i ? next : y)) }))}
                  onRemove={() => onChange((x) => ({ ...x, checks: x.checks.filter((_, j) => j !== i) }))}
                />
              ))}
              <div>
                <Button variant="ghost" size="sm" onClick={() => onChange((x) => ({ ...x, checks: [...x.checks, newCheck()] }))}>
                  <Plus /> Add check
                </Button>
              </div>
            </div>
            <p className="text-xs text-subtle">Verified without a model. A check that names an expectation replaces the grader for it.</p>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function ListTextarea({ value, onChange, placeholder, "aria-label": ariaLabel }: { value: string[]; onChange: (v: string[]) => void; placeholder?: string; "aria-label"?: string }) {
  const [text, setText] = React.useState(value.join("\n"));
  return (
    <Textarea
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => {
        const parsed = text.split(/[\n,]/).map((s) => s.trim()).filter(Boolean);
        if (parsed.join("\n") !== value.join("\n")) onChange(parsed);
      }}
      rows={2}
      placeholder={placeholder}
      aria-label={ariaLabel}
      className="mono min-h-0 text-xs"
      spellCheck={false}
    />
  );
}

function ArgsInput({ args, onChange }: { args: string[]; onChange: (args: string[]) => void }) {
  const [text, setText] = React.useState(joinArgs(args));
  return (
    <Input
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => {
        const parsed = splitArgs(text);
        if (joinArgs(parsed) !== joinArgs(args)) onChange(parsed);
      }}
      placeholder='scripts/verify.py --strict "out dir"'
      aria-label="Arguments"
      className="mono h-7 text-xs"
      spellCheck={false}
    />
  );
}

function CheckEditor({ check, expectations, onChange, onRemove }: { check: T.Check; expectations: string[]; onChange: (c: T.Check) => void; onRemove: () => void }) {
  const set = (patch: Record<string, unknown>) => onChange({ ...check, ...patch } as T.Check);
  const linked = check.expectation ?? null;
  const options = [{ value: NONE, label: <span className="text-muted">Not linked (grader decides)</span> }, ...expectations.map((e) => ({ value: e, label: e }))];
  if (linked && !expectations.includes(linked)) options.push({ value: linked, label: `${linked} (missing)` });
  const field = (label: string, key: string, placeholder: string, mono = true) => (
    <Field label={label} key={key}>
      <Input
        value={typeof (check as unknown as Record<string, unknown>)[key] === "string" ? ((check as unknown as Record<string, string>)[key] ?? "") : ""}
        onChange={(e) => set({ [key]: e.target.value })}
        placeholder={placeholder}
        aria-label={label}
        className={cn("h-7 text-xs", mono && "mono")}
      />
    </Field>
  );
  return (
    <div className="flex flex-col gap-2 rounded-lg bg-sunken p-2.5">
      <div className="flex items-center gap-2">
        <SimpleSelect size="sm" className="w-48 shrink-0" value={check.kind} onChange={(k) => onChange(convertCheck(check, k as T.CheckKind["kind"]))} options={CHECK_KINDS.map((k) => ({ value: k, label: CHECK_KIND_LABEL[k] }))} />
        <SimpleSelect size="sm" className="min-w-0 flex-1" value={linked ?? NONE} onChange={(v) => set({ expectation: v === NONE ? null : v })} options={options} placeholder="Verifies expectation…" />
        <Button variant="ghost" size="icon-sm" aria-label="Remove check" onClick={onRemove}>
          <X />
        </Button>
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        {check.kind === "file_exists" ? field("Path (in the sandbox)", "path", "invoice.csv") : null}
        {check.kind === "file_contains" ? [field("Path", "path", "report.md"), field("Text", "text", "Total: 1,284.50", false)] : null}
        {check.kind === "file_matches" ? [field("Path", "path", "report.md"), field("Regex", "pattern", "^Total:\\s+[\\d,.]+$")] : null}
        {check.kind === "output_contains" ? field("Text in the final answer", "text", "invoice.csv", false) : null}
        {check.kind === "output_matches" ? field("Regex over the final answer", "pattern", "\\b14 rows\\b") : null}
        {check.kind === "json_pointer" ? (
          <>
            {field("JSON file", "path", "result.json")}
            {field("Pointer", "pointer", "/summary/total")}
            <Field label="Equals (JSON or text)">
              <Input
                value={typeof check.equals === "string" ? check.equals : JSON.stringify(check.equals ?? "")}
                onChange={(e) => {
                  const raw = e.target.value;
                  let parsed: unknown;
                  try {
                    parsed = JSON.parse(raw);
                  } catch {
                    parsed = raw;
                  }
                  set({ equals: parsed });
                }}
                placeholder="1284.5"
                aria-label="Equals"
                className="mono h-7 text-xs"
              />
            </Field>
          </>
        ) : null}
        {check.kind === "command" ? (
          <>
            {field("Command", "command", "python3")}
            <Field label="Arguments" hint="Space separated; quote arguments with spaces.">
              <ArgsInput key={joinArgs(check.args)} args={check.args} onChange={(args) => set({ args })} />
            </Field>
          </>
        ) : null}
      </div>
    </div>
  );
}
