import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { t } from "@/lib/i18n";
import { bodyStats, parseFrontmatter, renderLoaded, skillMarkdown, validateSkill, type SkillDraft } from "@/lib/authoring/model";
import { ensureSkill, fixSkill, publishSkill, restoreSkill, saveSkill, setValidationCounts, snapshotSkill, updateSkill, useAuthoring } from "@/lib/authoring/store";
import { CodeEditor } from "@/features/editor/CodeEditor";
import { Screen } from "@/components/chrome";

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

  if (!draft) {
    return (
      <Screen testId="skill-editor" title={id || t("library.empty")}>
        <p className="text-sm text-muted-foreground">{t("library.empty")}</p>
      </Screen>
    );
  }
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
        <Button type="button" variant="outline" size="sm" onClick={() => updateSkill({ ...draft, profile: draft.profile === "portable" ? "claude-code" : "portable" })}>{t("editor.portable")}</Button>
        <Button type="button" variant="outline" size="sm">{t("editor.rename")}</Button>
      </div>
      <div className="mt-2 flex flex-wrap gap-2">
        <Button type="button" variant="outline" size="sm" onClick={() => saveSkill(draft)}>{t("editor.save")}</Button>
        <Button type="button" variant="outline" size="sm" onClick={() => snapshotSkill(draft.id, "snapshot")}>{t("editor.snapshot")}</Button>
        <Button type="button" variant="default" size="sm" disabled={blocked} title={blocked ? t("editor.blocking", { count: String(errors) }) : ""} onClick={() => publish(false)}>{t("editor.publish")}</Button>
        <Button type="button" variant="outline" size="sm" disabled={blocked} title={blocked ? t("editor.blocking", { count: String(errors) }) : ""} onClick={() => publish(false)}>{t("editor.package")}</Button>
        <Button type="button" variant="outline" size="sm" disabled={blocked} title={blocked ? t("editor.blocking", { count: String(errors) }) : ""} onClick={() => publish(false)}>{t("editor.deploy")}</Button>
        <Button type="button" disabled title={t("editor.runTests.later")} variant="outline" size="sm">{t("library.menu.tests")}</Button>
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
          {rename ? <Input aria-label={t("editor.renameFile")} value={rename} onChange={(event) => setRename(event.target.value)} className="mt-2 h-8 w-full" /> : null}
          <form
            className="mt-2"
            onSubmit={(event) => {
              event.preventDefault();
              const resolved = importPath.split(/[/\\]/).includes("..") || importPath.startsWith("/");
              setImportError(resolved ? "invalid_input path escapes the skill directory" : null);
            }}
          >
            <Input aria-label={t("editor.importPath")} value={importPath} onChange={(event) => setImportPath(event.target.value)} className="h-8 w-full" />
          </form>
          {importError ? <p role="alert">{importError}</p> : null}
        </aside>
        <div className="min-w-0">
          <label className="block text-sm">
            {t("editor.description")}
            <Input aria-label={t("editor.description")} value={descriptionOf(draft)} onChange={(event) => changeDescription(event.target.value)} className="mt-1 h-8 w-full" />
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
              <Button key={item} type="button" role="tab" aria-selected={tab === item} variant={tab === item ? "default" : "outline"} size="sm" className="whitespace-normal text-left" onClick={() => setTab(item)}>
                {t(`editor.${item}`)}
              </Button>
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
                  <Button key={item} type="button" aria-pressed={preview === item} variant="outline" size="sm" onClick={() => setPreview(item)}>{t(`editor.${item}`)}</Button>
                ))}
              </div>
              {preview === "loaded" ? (
                <div className="mt-2">
                  <pre className="whitespace-pre-wrap">{loaded.text}</pre>
                  {loaded.showRun
                    ? loaded.commands.map((command) => (
                        <Button key={command} type="button" variant="outline" size="sm" className="mt-1" onClick={() => setRan(command)}>{t("editor.runNow")}</Button>
                      ))
                    : <p>{t("editor.shellOff")}</p>}
                  {ran ? <p>{ran}</p> : null}
                  <Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => updateSkill({ ...draft, shellPreview: !draft.shellPreview })}
                  >
                    {t("editor.shellToggle")}
                  </Button>
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
          <Button type="button" variant="default" size="sm" className="mt-2" onClick={() => publish(true)}>{t("editor.confirm.action")}</Button>
        </div>
      ) : null}
    </section>
  );
}

function descriptionOf(draft: SkillDraft): string {
  return parseFrontmatter(skillMarkdown(draft)).keys.find((entry) => entry.name === "description")?.value ?? "";
}
