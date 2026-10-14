import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { t } from "@/lib/i18n";
import { bodyStats, parseFrontmatter, renderLoaded, skillMarkdown, validateSkill, type SkillDraft } from "@/lib/phase3/model";
import { ensureSkill, fixSkill, publishSkill, restoreSkill, saveSkill, setValidationCounts, snapshotSkill, updateSkill, useAuthoring } from "@/lib/phase3/store";
import { CodeEditor } from "./CodeEditor";

const TABS = ["problems", "preview", "versions"] as const;

export function SkillEditor() {
  const { id = "" } = useParams();
  const authoring = useAuthoring();
  const [tab, setTab] = useState<(typeof TABS)[number]>("problems");
  const [preview, setPreview] = useState<"listing" | "loaded" | "resources">("listing");
  const [confirm, setConfirm] = useState<string[] | null>(null);
  const [rename, setRename] = useState<string | null>(null);
  const [importPath, setImportPath] = useState("");
  const [importError, setImportError] = useState<string | null>(null);
  const [ran, setRan] = useState<string | null>(null);
  const draft = authoring.skills[id] ?? null;

  useEffect(() => {
    const seeded = ensureSkill(id);
    if (seeded) setValidationCounts(validateSkill(seeded));
    return () => setValidationCounts(null);
  }, [id]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey;
      if (mod && event.shiftKey && event.key.toLowerCase() === "s") {
        event.preventDefault();
        if (draft) snapshotSkill(draft.id, "snapshot");
        return;
      }
      if (mod && event.key.toLowerCase() === "s") {
        event.preventDefault();
        if (draft) saveSkill(draft);
        return;
      }
      if (mod && event.key.toLowerCase() === "j") {
        event.preventDefault();
        setTab((current) => TABS[(TABS.indexOf(current) + 1) % TABS.length]);
      }
      if (event.key === "F2") {
        event.preventDefault();
        setRename("SKILL.md");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [draft]);

  if (!draft) return <p>{t("library.empty")}</p>;
  const findings = validateSkill(draft);
  const errors = findings.filter((item) => item.severity === "error").length;
  const stats = bodyStats(draft);
  const loaded = renderLoaded(draft);
  const blocked = errors > 0;

  function changeDescription(value: string) {
    const files = draft!.files.map((file) =>
      file.path.endsWith("SKILL.md") || file.path === "skill.md"
        ? { ...file, content: file.content.replace(/description: .*/, `description: ${value}`) }
        : file,
    );
    const next = { ...draft!, files };
    updateSkill(next);
    setValidationCounts(validateSkill(next));
  }

  function changeBody(value: string) {
    const current = skillMarkdown(draft!);
    const doc = parseFrontmatter(current);
    const head = current.slice(0, current.length - doc.body.length);
    const files = draft!.files.map((file) => (file.path.endsWith("SKILL.md") || file.path === "skill.md" ? { ...file, content: `${head}${value}` } : file));
    const next = { ...draft!, files };
    updateSkill(next);
    setValidationCounts(validateSkill(next));
  }

  function publish(confirmed: boolean) {
    const result = publishSkill(draft!.id, confirmed);
    if (result.kind === "confirm") setConfirm(result.agents);
    else setConfirm(null);
  }

  return (
    <section data-testid="skill-editor" className="flex min-h-0 flex-col">
      <h1 className="font-mono text-lg font-semibold">{draft.name}</h1>
      <div className="mt-2 flex flex-wrap gap-2 text-sm">
        <label>
          {t("editor.profile")}
          <select aria-label={t("editor.profile")} value={draft.profile} onChange={(event) => updateSkill({ ...draft, profile: event.target.value as SkillDraft["profile"] })} className="ml-2 rounded-md border border-input px-1 py-0.5">
            <option value="portable">portable</option>
            <option value="claude-code">claude-code</option>
          </select>
        </label>
        <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => updateSkill({ ...draft, profile: draft.profile === "portable" ? "claude-code" : "portable" })}>{t("editor.portable")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("editor.rename")}</button>
      </div>
      <div className="mt-2 flex flex-wrap gap-2">
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => saveSkill(draft)}>{t("editor.save")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => snapshotSkill(draft.id, "snapshot")}>{t("editor.snapshot")}</button>
        <button type="button" className="rounded-md bg-primary px-2 py-1 text-sm text-primary-foreground disabled:opacity-50" disabled={blocked} title={blocked ? t("editor.blocking", { count: String(errors) }) : ""} onClick={() => publish(false)}>{t("editor.publish")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm disabled:opacity-50" disabled={blocked} title={blocked ? t("editor.blocking", { count: String(errors) }) : ""}>{t("editor.package")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm disabled:opacity-50" disabled={blocked} title={blocked ? t("editor.blocking", { count: String(errors) }) : ""}>{t("editor.deploy")}</button>
        <button type="button" disabled title={t("editor.runTests.later")} className="rounded-md border border-border px-2 py-1 text-sm opacity-50">{t("library.menu.tests")}</button>
      </div>
      <div className="mt-3 grid min-h-0 grid-cols-[9rem_minmax(0,1fr)_12rem] gap-2">
        <aside aria-label={t("editor.fileTree")} className="min-w-0 rounded-md border border-border p-2 text-sm">
          <ul>
            {draft.files.map((file) => (
              <li key={file.path}>
                <button type="button" className="truncate" onClick={() => setRename(file.path)}>{file.path}</button>
              </li>
            ))}
          </ul>
          {rename ? <input aria-label={t("editor.renameFile")} value={rename} onChange={(event) => setRename(event.target.value)} className="mt-2 w-full rounded-md border border-input px-1 py-0.5 text-xs" /> : null}
          <form
            className="mt-2"
            onSubmit={(event) => {
              event.preventDefault();
              const resolved = importPath.split(/[/\\]/).includes("..") || importPath.startsWith("/");
              setImportError(resolved ? "invalid_input path escapes the skill directory" : null);
            }}
          >
            <input aria-label={t("editor.importPath")} value={importPath} onChange={(event) => setImportPath(event.target.value)} className="w-full rounded-md border border-input px-1 py-0.5 text-xs" />
          </form>
          {importError ? <p role="alert">{importError}</p> : null}
        </aside>
        <div className="min-w-0">
          <label className="block text-sm">
            {t("editor.description")}
            <input aria-label={t("editor.description")} value={descriptionOf(draft)} onChange={(event) => changeDescription(event.target.value)} className="mt-1 w-full rounded-md border border-input px-2 py-1 text-sm" />
          </label>
          <p data-testid="body-gauge" data-amber={stats.amber ? "true" : "false"} className={`mt-2 text-xs ${stats.amber ? "text-warning-text" : "text-muted-foreground"}`}>
            {t("editor.gauge")} {stats.lines} / 500 · {stats.tokens} / 5000
          </p>
          <div className="mt-2">
            <CodeEditor label={t("editor.body")} language="markdown" value={parseFrontmatter(skillMarkdown(draft)).body} onChange={changeBody} />
          </div>
        </div>
        <aside className="min-w-0">
          <div data-testid="editor-tabs" role="tablist" className="flex flex-col gap-1">
            {TABS.map((item) => (
              <button key={item} type="button" role="tab" aria-selected={tab === item} className="rounded-md border border-border px-2 py-1 text-left text-sm" onClick={() => setTab(item)}>
                {t(`editor.${item}`)}
              </button>
            ))}
          </div>
          {tab === "problems" ? (
            <ul data-testid="problems" className="mt-2 space-y-2 text-xs">
              {findings.map((item) => (
                <li key={`${item.ruleId}-${item.path}-${item.line}`}>
                  <span>{item.ruleId}</span> {item.message}
                  {item.fixId ? (
                    <button type="button" className="ml-1 underline" onClick={() => fixSkill(draft.id, item.fixId!)}>{t("editor.fix")}</button>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : null}
          {tab === "preview" ? (
            <div data-testid="preview" className="mt-2 text-xs">
              <div className="flex flex-wrap gap-1">
                {(["listing", "loaded", "resources"] as const).map((item) => (
                  <button key={item} type="button" aria-pressed={preview === item} className="rounded-md border border-border px-1 py-0.5" onClick={() => setPreview(item)}>{t(`editor.${item}`)}</button>
                ))}
              </div>
              {preview === "loaded" ? (
                <div className="mt-2">
                  <pre className="whitespace-pre-wrap">{loaded.text}</pre>
                  {loaded.showRun
                    ? loaded.commands.map((command) => (
                        <button key={command} type="button" className="mt-1 rounded-md border border-border px-1 py-0.5" onClick={() => setRan(command)}>{t("editor.runNow")}</button>
                      ))
                    : <p>{t("editor.shellOff")}</p>}
                  {ran ? <p>{ran}</p> : null}
                  <button
                    type="button"
                    className="mt-2 rounded-md border border-border px-1 py-0.5"
                    onClick={() => updateSkill({ ...draft, shellPreview: !draft.shellPreview })}
                  >
                    {t("editor.shellToggle")}
                  </button>
                </div>
              ) : null}
              {preview === "listing" ? <p className="mt-2">{draft.name}: {descriptionOf(draft)}</p> : null}
              {preview === "resources" ? <ul className="mt-2">{draft.files.map((file) => <li key={file.path}>{file.path}</li>)}</ul> : null}
            </div>
          ) : null}
          {tab === "versions" ? (
            <ul data-testid="versions" className="mt-2 space-y-2 text-xs">
              {(authoring.versions[draft.id] ?? []).filter((version) => !version.hidden).map((version) => (
                <li key={version.id}>
                  <span>{version.id}</span> {version.source} {version.label}
                  <button type="button" className="ml-1 underline" onClick={() => restoreSkill(draft.id, version.id)}>{t("editor.restore")}</button>
                </li>
              ))}
            </ul>
          ) : null}
        </aside>
      </div>
      {confirm ? (
        <div role="dialog" aria-label={t("editor.confirm.title")} className="mt-3 rounded-md border border-border p-3">
          <h2 className="text-sm font-medium">{t("editor.confirm.title")}</h2>
          <ul>{confirm.map((agent) => <li key={agent}>{agent}</li>)}</ul>
          <button type="button" className="mt-2 rounded-md bg-primary px-2 py-1 text-sm text-primary-foreground" onClick={() => publish(true)}>{t("editor.confirm.action")}</button>
        </div>
      ) : null}
    </section>
  );
}

function descriptionOf(draft: SkillDraft): string {
  return parseFrontmatter(skillMarkdown(draft)).keys.find((entry) => entry.name === "description")?.value ?? "";
}
