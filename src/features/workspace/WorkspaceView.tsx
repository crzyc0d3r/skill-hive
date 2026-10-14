import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { lcsDiff, presetFill, presetLabel, type SkillCard } from "@/features/library/model";
import type { AgentRow, PresetRow } from "@/lib/ipc/catalog";
import { t } from "@/lib/i18n";

export interface WorkspaceCard {
  id: string;
  name: string;
  status: "local_only" | "in_sync" | "center_changed";
  tags: string[];
  body: string;
  center: string;
  managed: boolean;
}

export function WorkspaceView({
  presets,
  cards,
  agents = [],
  library = [],
  agentKey,
  onApply,
  onCreate,
}: {
  presets: PresetRow[];
  cards: WorkspaceCard[];
  agents?: AgentRow[];
  library?: SkillCard[];
  agentKey?: string;
  onApply?: (id: string) => void;
  onCreate?: (name: string) => void;
}) {
  const [applied, setApplied] = useState<string | null>(null);
  const [name, setName] = useState("Research");
  const [rows, setRows] = useState(cards);
  useEffect(() => {
    setRows(cards);
  }, [cards]);
  const [query, setQuery] = useState("");
  const [tag, setTag] = useState<string | null>(null);
  const [layout, setLayout] = useState<"grid" | "list">("grid");
  const [selected, setSelected] = useState<string[]>([]);
  const [pending, setPending] = useState<{ ids: string[]; kind: "upload" | "pull" | "remove" | "delete" } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const [sheet, setSheet] = useState<"local" | "diff" | "center">("local");
  const [adding, setAdding] = useState(false);

  const tags = useMemo(() => [...new Set(rows.flatMap((row) => row.tags))], [rows]);
  const visible = rows.filter((row) => {
    if (tag && !row.tags.includes(tag)) return false;
    if (!query.trim()) return true;
    return row.name.toLowerCase().includes(query.trim().toLowerCase());
  });
  const open = rows.find((row) => row.id === openId) ?? null;

  function applyPending() {
    if (!pending) return;
    if (pending.kind === "delete" && rows.some((row) => pending.ids.includes(row.id) && row.managed)) {
      setNotice(t("workspace.refused"));
      setPending(null);
      return;
    }
    if (pending.kind === "remove" || pending.kind === "delete") {
      setRows((current) => current.filter((row) => !pending.ids.includes(row.id)));
      setSelected([]);
    } else {
      setRows((current) => current.map((row) => (pending.ids.includes(row.id) ? { ...row, status: "in_sync", managed: true } : row)));
    }
    setPending(null);
  }

  if (!agentKey) {
    return (
      <section data-testid="global-workspace">
        <h1 className="font-mono text-[22px] font-semibold">{t("workspace.title")}</h1>
        <form
          className="mt-3 flex gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            onCreate?.(name);
          }}
        >
          <input aria-label={t("preset.create")} value={name} onChange={(event) => setName(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
          <button type="submit" className="rounded-md border border-border px-2 py-1 text-sm">{t("preset.new")}</button>
        </form>
        <div className="mt-3 flex flex-wrap gap-2" data-testid="preset-bar">
          {presets.map((preset) => {
            const total = preset.skillIds.length;
            const synced = applied === preset.id ? total : 0;
            const fill = presetFill(synced, total);
            return (
              <button
                key={preset.id}
                type="button"
                data-fill={fill}
                className="rounded-full border border-border px-3 py-1 text-sm"
                onClick={() => {
                  setApplied(preset.id);
                  onApply?.(preset.id);
                }}
              >
                {presetLabel(preset.name, synced, total)}
              </button>
            );
          })}
        </div>
        <ul className="mt-4 grid grid-cols-2 gap-3" data-testid="agent-catalog">
          {agents.map((agent) => (
            <li key={agent.key} className="rounded-md border border-border p-3 text-sm">
              <Link to={`/global-workspace/${agent.key}`}>{agent.name}</Link>
              <div className="text-muted-foreground">{agent.path}</div>
            </li>
          ))}
        </ul>
      </section>
    );
  }

  return (
    <section data-testid="global-workspace">
      <div data-testid="agent-page">
        <h1 className="font-mono text-[22px] font-semibold">{agentKey}</h1>
        <div className="mt-3 flex flex-wrap gap-2">
          <input aria-label={t("workspace.search")} data-search value={query} onChange={(event) => setQuery(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
          <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setLayout(layout === "grid" ? "list" : "grid")}>
            {layout === "grid" ? t("workspace.view.list") : t("workspace.view.grid")}
          </button>
          <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setAdding(true)}>{t("workspace.add")}</button>
        </div>
        <div className="mt-2 flex flex-wrap gap-2">
          {tags.map((item) => (
            <button key={item} type="button" className="rounded-full border border-border px-2 py-0.5 text-xs" onClick={() => setTag(tag === item ? null : item)}>
              {item}
            </button>
          ))}
        </div>
        {selected.length > 0 ? (
          <div className="mt-2 flex gap-2">
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setPending({ ids: selected, kind: "remove" })}>{t("workspace.batch.remove")}</button>
            <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setPending({ ids: selected, kind: "delete" })}>{t("workspace.batch.delete")}</button>
          </div>
        ) : null}
        {notice ? <p role="status">{notice}</p> : null}
        <ul data-layout={layout} className={layout === "grid" ? "mt-4 grid grid-cols-2 gap-3" : "mt-4 space-y-2"}>
          {visible.map((card) => (
            <li key={card.id} className="rounded-md border border-border p-3 text-sm">
              <label className="flex items-center gap-2">
                <input type="checkbox" aria-label={`Select ${card.name}`} checked={selected.includes(card.id)} onChange={() => setSelected((current) => current.includes(card.id) ? current.filter((id) => id !== card.id) : [...current, card.id])} />
                <button type="button" className="font-medium" onClick={() => { setOpenId(card.id); setSheet("local"); }}>{card.name}</button>
              </label>
              <span className="ml-2 text-muted-foreground">
                {card.status === "local_only" ? t("workspace.localOnly") : card.status === "center_changed" ? t("workspace.centerChanged") : t("workspace.inSync")}
              </span>
              <div className="mt-2 flex flex-wrap gap-2">
                <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => setPending({ ids: [card.id], kind: "upload" })}>{t("workspace.upload")}</button>
                <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => setPending({ ids: [card.id], kind: "pull" })}>{t("workspace.pull")}</button>
                <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => setPending({ ids: [card.id], kind: "remove" })}>{t("workspace.remove")}</button>
                <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => setPending({ ids: [card.id], kind: "delete" })}>{t("workspace.delete")}</button>
              </div>
            </li>
          ))}
        </ul>
        {pending ? (
          <div role="dialog" aria-label={t("workspace.confirm")} className="mt-3 rounded-md border border-border p-3">
            <p>{pending.kind}</p>
            <button type="button" className="rounded-md bg-primary px-2 py-1 text-sm text-primary-foreground" onClick={applyPending}>{t("workspace.confirm")}</button>
          </div>
        ) : null}
        {adding ? (
          <div data-testid="add-sheet" className="mt-3 rounded-md border border-border p-3">
            {library.map((skill) => (
              <button
                key={skill.id}
                type="button"
                className="mr-2 text-sm"
                onClick={() => {
                  setRows((current) => current.some((row) => row.id === skill.id) ? current : [...current, { id: skill.id, name: skill.name, status: "in_sync", tags: skill.tags, body: skill.body, center: skill.sourceBody, managed: true }]);
                  setAdding(false);
                }}
              >
                {skill.name}
              </button>
            ))}
          </div>
        ) : null}
        {open ? (
          <aside data-testid="workspace-detail" className="mt-3 rounded-md border border-border p-3">
            <div className="flex gap-2">
              <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setSheet("local")}>{t("library.detail.local")}</button>
              <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setSheet("diff")}>{t("library.detail.diff")}</button>
              <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setSheet("center")}>{t("workspace.detail.center")}</button>
            </div>
            {sheet === "local" ? <pre className="mt-2 whitespace-pre-wrap text-sm">{open.body}</pre> : null}
            {sheet === "center" ? <pre className="mt-2 whitespace-pre-wrap text-sm">{open.center}</pre> : null}
            {sheet === "diff" ? (
              <ul className="mt-2 font-mono text-sm">
                {lcsDiff(open.body, open.center).map((row, index) => (
                  <li key={`${row.kind}-${index}`}>{row.text}</li>
                ))}
              </ul>
            ) : null}
          </aside>
        ) : null}
      </div>
    </section>
  );
}
