import { useEffect, useMemo, useState } from "react";
import { lcsDiff, filterSkills, type LibraryFilter, type SkillCard } from "@/lib/phase1/model";
import { t } from "@/lib/i18n";

export function LibraryView({
  skills,
  initialId,
  onRenameTag,
  onAddTag,
  onSync,
}: {
  skills: SkillCard[];
  initialId?: string | null;
  onRenameTag?: (from: string, to: string) => void;
  onAddTag?: (skillId: string, tag: string) => void;
  onSync?: (ids: string[], agents: string[]) => void;
}) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<LibraryFilter>("all");
  const [source, setSource] = useState<string | null>(null);
  const [tag, setTag] = useState<string | null>(null);
  const [layout, setLayout] = useState<"grid" | "list">("grid");
  const [selected, setSelected] = useState<string[]>([]);
  const [openId, setOpenId] = useState<string | null>(initialId ?? null);
  useEffect(() => {
    if (initialId) setOpenId(initialId);
  }, [initialId]);
  const [tab, setTab] = useState<"local" | "diff" | "source">("local");
  const [rename, setRename] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [syncOpen, setSyncOpen] = useState(false);
  const [tagDraft, setTagDraft] = useState<Record<string, string>>({});

  const sources = useMemo(() => [...new Set(skills.map((skill) => skill.source))], [skills]);
  const tags = useMemo(() => [...new Set(skills.flatMap((skill) => skill.tags))], [skills]);
  const visible = filterSkills(skills, query, filter, source, tag);
  const open = skills.find((skill) => skill.id === openId) ?? null;
  const rendered = visible.length > 100 ? visible.slice(0, 100) : visible;

  return (
    <section data-testid="library">
      <h1 className="font-mono text-[22px] font-semibold">{t("library.title")}</h1>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <input
          aria-label={t("library.search")}
          data-search
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          className="rounded-md border border-input bg-background px-2 py-1 text-sm"
        />
        {(["all", "enabled", "available"] as const).map((item) => (
          <button
            key={item}
            type="button"
            aria-pressed={filter === item}
            className="rounded-md border border-border px-2 py-1 text-sm"
            onClick={() => setFilter(item)}
          >
            {t(`library.filter.${item}`)}
          </button>
        ))}
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setLayout(layout === "grid" ? "list" : "grid")}>
          {layout === "grid" ? t("library.view.list") : t("library.view.grid")}
        </button>
      </div>
      <div className="mt-2 flex flex-wrap gap-2">
        {sources.map((item) => (
          <button key={item} type="button" className="rounded-full border border-border px-2 py-0.5 text-xs" onClick={() => setSource(source === item ? null : item)}>
            {item}
          </button>
        ))}
        <button type="button" className="rounded-full border border-border px-2 py-0.5 text-xs" onClick={() => setTag(tag === "untagged" ? null : "untagged")}>
          Untagged
        </button>
        {tags.map((item) => (
          <button
            key={item}
            type="button"
            className="rounded-full border border-border px-2 py-0.5 text-xs"
            onClick={() => setTag(tag === item ? null : item)}
            onContextMenu={(event) => {
              event.preventDefault();
              setRename(item);
              setRenameValue(item);
            }}
          >
            {item}
          </button>
        ))}
      </div>
      {rename ? (
        <form
          className="mt-2 flex gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            onRenameTag?.(rename, renameValue);
            setRename(null);
          }}
        >
          <input aria-label={t("library.tag.rename")} value={renameValue} onChange={(event) => setRenameValue(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
          <button type="submit" className="rounded-md bg-primary px-2 py-1 text-sm text-primary-foreground">{t("library.tag.rename")}</button>
        </form>
      ) : null}
      {selected.length > 0 ? (
        <div className="mt-2">
          <button type="button" className="rounded-md bg-primary px-2 py-1 text-sm text-primary-foreground" onClick={() => setSyncOpen(true)}>
            {t("library.sync")}
          </button>
        </div>
      ) : null}
      {syncOpen ? (
        <form
          className="mt-2 rounded-md border border-border p-3"
          onSubmit={(event) => {
            event.preventDefault();
            const data = new FormData(event.currentTarget);
            const agents = ["claude_code", "codex"].filter((agent) => data.get(agent) === "on");
            onSync?.(selected, agents);
            setSyncOpen(false);
          }}
        >
          <label className="mr-3 text-sm"><input type="checkbox" name="claude_code" /> Claude Code</label>
          <label className="mr-3 text-sm"><input type="checkbox" name="codex" /> Codex</label>
          <button type="submit" className="rounded-md border border-border px-2 py-1 text-sm">{t("library.sync")}</button>
        </form>
      ) : null}
      {rendered.length === 0 ? <p className="mt-4 text-sm text-muted-foreground">{t("library.empty")}</p> : null}
      <ul data-testid="library-cards" data-layout={layout} className={layout === "grid" ? "mt-4 grid grid-cols-2 gap-3" : "mt-4 space-y-2"}>
        {rendered.map((skill) => (
          <li key={skill.id} className="rounded-lg border border-border bg-card p-3">
            <label className="flex items-start gap-2">
              <input
                type="checkbox"
                aria-label={`Select ${skill.name}`}
                checked={selected.includes(skill.id)}
                onChange={() =>
                  setSelected((current) =>
                    current.includes(skill.id) ? current.filter((id) => id !== skill.id) : [...current, skill.id],
                  )
                }
              />
              <button type="button" className="text-left" onClick={() => { setOpenId(skill.id); setTab("local"); }}>
                <span className="block font-medium whitespace-normal">{skill.name}</span>
                <span className="block text-sm text-muted-foreground">{skill.description}</span>
              </button>
            </label>
            <div className="mt-2 flex gap-1" data-testid={`sync-dots-${skill.id}`}>
              {skill.agents.map((agent) => (
                <span key={agent} data-filled="true" className="inline-block h-2 w-2 rounded-full bg-primary" title={agent} />
              ))}
            </div>
            <form
              className="mt-2 flex gap-1"
              onSubmit={(event) => {
                event.preventDefault();
                const value = tagDraft[skill.id]?.trim();
                if (value) onAddTag?.(skill.id, value);
                setTagDraft((current) => ({ ...current, [skill.id]: "" }));
              }}
            >
              <input
                aria-label={`${t("library.tag.add")} ${skill.name}`}
                value={tagDraft[skill.id] ?? ""}
                onChange={(event) => setTagDraft((current) => ({ ...current, [skill.id]: event.target.value }))}
                className="w-24 rounded-md border border-input px-1 py-0.5 text-xs"
              />
              <button type="submit" className="text-xs">{t("library.tag.add")}</button>
            </form>
          </li>
        ))}
      </ul>
      {open ? (
        <aside data-testid="detail-panel" className="mt-4 rounded-lg border border-border p-3">
          <div className="flex gap-2">
            {(["local", "diff", "source"] as const).map((item) => (
              <button key={item} type="button" aria-pressed={tab === item} className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setTab(item)}>
                {t(`library.detail.${item}`)}
              </button>
            ))}
          </div>
          {tab === "local" ? <pre className="mt-2 whitespace-pre-wrap text-sm">{open.body}</pre> : null}
          {tab === "source" ? <pre className="mt-2 whitespace-pre-wrap text-sm">{open.sourceBody}</pre> : null}
          {tab === "diff" ? (
            <ul data-testid="diff-view" className="mt-2 font-mono text-sm">
              {lcsDiff(open.body, open.sourceBody).map((row, index) => (
                <li key={`${row.kind}-${index}`} data-kind={row.kind}>
                  {row.kind === "add" ? "+" : row.kind === "del" ? "-" : " "} {row.text}
                </li>
              ))}
            </ul>
          ) : null}
        </aside>
      ) : null}
    </section>
  );
}
