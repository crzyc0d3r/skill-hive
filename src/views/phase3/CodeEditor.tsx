import { useEffect, useRef } from "react";

export function CodeEditor({
  value,
  onChange,
  language,
  label,
}: {
  value: string;
  onChange: (value: string) => void;
  language: "markdown" | "yaml" | "json" | "python" | "shell";
  label: string;
}) {
  const host = useRef<HTMLDivElement>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (import.meta.env.MODE === "test") return;
    const parent = host.current;
    if (!parent) return;
    let view: { destroy: () => void } | null = null;
    let cancelled = false;
    void import("codemirror").then(async (cm) => {
      if (cancelled || !parent) return;
      const languageExtension = await languageExtensionFor(language);
      view = new cm.EditorView({
        doc: value,
        extensions: [
          cm.basicSetup,
          languageExtension,
          cm.EditorView.updateListener.of((update) => {
            if (update.docChanged) onChangeRef.current(update.state.doc.toString());
          }),
        ],
        parent,
      });
    });
    return () => {
      cancelled = true;
      view?.destroy();
    };
    // The editor is mounted once per file. Value changes from the form go through the textarea in tests.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [language]);

  if (import.meta.env.MODE === "test") {
    return (
      <textarea
        aria-label={label}
        data-testid="code-editor"
        data-language={language}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-40 w-full rounded-md border border-input bg-background p-2 font-mono text-sm"
      />
    );
  }

  return (
    <div className="min-w-0">
      <div ref={host} data-testid="code-editor" data-language={language} className="min-h-40 overflow-hidden rounded-md border border-input" />
      <textarea aria-label={label} value={value} onChange={(event) => onChange(event.target.value)} className="mt-2 h-24 w-full rounded-md border border-input bg-background p-2 font-mono text-sm" />
    </div>
  );
}

async function languageExtensionFor(language: "markdown" | "yaml" | "json" | "python" | "shell") {
  if (language === "markdown") return (await import("@codemirror/lang-markdown")).markdown();
  if (language === "yaml") return (await import("@codemirror/lang-yaml")).yaml();
  if (language === "json") return (await import("@codemirror/lang-json")).json();
  if (language === "python") return (await import("@codemirror/lang-python")).python();
  const legacy = await import("@codemirror/legacy-modes/mode/shell");
  const languageModule = await import("@codemirror/language");
  return languageModule.StreamLanguage.define(legacy.shell);
}
