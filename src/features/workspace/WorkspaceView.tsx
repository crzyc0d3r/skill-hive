import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { SearchIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { useContext, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { lcsDiff, presetFill, presetLabel, type SkillCard } from "@/features/library/model";
import type { AgentRow, PresetRow } from "@/lib/ipc/catalog";
import { deleteManagedSkills, silent, syncSkillToTool, unsyncSkillFromTool } from "@/lib/ipc/actions";
import { useDebounced } from "@/lib/use-debounced";
import { t } from "@/lib/i18n";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { StatePanel } from "@/components/chrome";
import { WorkspaceSkeleton } from "@/components/skeletons";
import { RetryContext, useShellPhase } from "@/lib/view-state";

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
  onApply: (id: string) => void | Promise<void>;
  onCreate: (name: string) => void | Promise<void>;
}) {
  const phase = useShellPhase();
  const retry = useContext(RetryContext);
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
  const [pickerQuery, setPickerQuery] = useState("");
  const [coverageSkills, setCoverageSkills] = useState(library);
  const [loadedPresets, setLoadedPresets] = useState<PresetRow[] | null>(null);
  useEffect(() => {
    setCoverageSkills(library);
  }, [library]);
  useEffect(() => {
    void silent<{ id: string; name: string; skillIds: string[] }[]>("get_presets").then((rows) => {
      if (Array.isArray(rows) && rows.length) setLoadedPresets(rows.map((row) => ({ ...row, synced: 0 })));
    });
    void silent<string | null>("get_active_preset").then((id) => {
      if (typeof id === "string" && id) setApplied(id);
    });
  }, []);
  const shownPresets = loadedPresets ?? presets;

  const debouncedQuery = useDebounced(query, 150);
  const debouncedPicker = useDebounced(pickerQuery, 150);
  const tags = useMemo(() => [...new Set(rows.flatMap((row) => row.tags))], [rows]);
  const visible = rows.filter((row) => {
    if (tag && !row.tags.includes(tag)) return false;
    if (!debouncedQuery.trim()) return true;
    return row.name.toLowerCase().includes(debouncedQuery.trim().toLowerCase());
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
      if (pending.kind === "remove" && agentKey) {
        pending.ids.forEach((id) => {
          void unsyncSkillFromTool(id, agentKey);
          const name = rows.find((row) => row.id === id)?.name ?? id;
          void silent("delete_global_local_skill", { tool: agentKey, name });
        });
      }
      if (pending.kind === "delete") {
        void deleteManagedSkills(pending.ids);
        if (agentKey) {
          pending.ids.forEach((id) => {
            const name = rows.find((row) => row.id === id)?.name ?? id;
            void silent("delete_global_local_skill", { tool: agentKey, name });
          });
        }
      }
    } else {
      setRows((current) => current.map((row) => (pending.ids.includes(row.id) ? { ...row, status: "in_sync", managed: true } : row)));
      if (agentKey) {
        pending.ids.forEach((id) => {
          const name = rows.find((row) => row.id === id)?.name ?? id;
          if (pending.kind === "upload") void syncSkillToTool(id, agentKey);
          if (pending.kind === "pull") void silent("import_global_local_skill_to_center", { tool: agentKey, name });
        });
      }
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
            void onCreate(name);
          }}
        >
          <Input aria-label={t("preset.create")} value={name} onChange={(event) => setName(event.target.value)} className="h-8 w-56" />
          <Button type="submit" variant="outline" size="sm">{t("preset.new")}</Button>
        </form>
        {phase === "loading" ? <WorkspaceSkeleton /> : null}
        {phase === "error" ? <StatePanel tone="error" title={t("shell.error")} action={<Button type="button" size="sm" onClick={retry}>{t("banner.retry")}</Button>} /> : null}
        {phase === "ready" ? <>
        <div className="mt-3 flex flex-wrap gap-2" data-testid="preset-bar">
          {shownPresets.map((preset) => {
            const members = coverageSkills.filter((skill) => preset.skillIds.includes(skill.id));
            const total = preset.skillIds.length || members.length;
            const synced = preset.skillIds.filter((id) => {
              const skill = coverageSkills.find((item) => item.id === id);
              return Boolean(skill && skill.agents.length > 0);
            }).length;
            const fill = presetFill(synced, total);
            return (
              <Button key={preset.id} type="button" data-fill={fill} aria-pressed={applied === preset.id} variant="outline" size="sm" onClick={() => {
                const tool = agents[0]?.key ?? "claude_code";
                setCoverageSkills((current) => current.map((skill) => {
                  if (!preset.skillIds.includes(skill.id) || skill.agents.includes(tool)) return skill;
                  return { ...skill, agents: [...skill.agents, tool] };
                }));
                setApplied(preset.id);
                void onApply(preset.id);
                void silent("switch_preset", { presetId: preset.id });
                void silent("apply_preset_to_coding_agents", { presetId: preset.id });
                void silent<string[]>("get_preset_skill_order", { presetId: preset.id });
              }}>
                {presetLabel(preset.name, synced, total)}
              </Button>
            );
          })}
        </div>
        <ul className="mt-4 grid grid-cols-2 gap-3" data-testid="agent-catalog">
          {agents.map((agent) => (
            <li key={agent.key} className="rounded-lg border border-border bg-card p-3 text-sm shadow-none">
              <Link to={`/global-workspace/${agent.key}`}>{agent.name}</Link>
              <div className="text-muted-foreground">{agent.path}</div>
            </li>
          ))}
        </ul>
        </> : null}
      </section>
    );
  }

  return (
    <section data-testid="global-workspace">
      <div data-testid="agent-page">
        <h1 className="font-mono text-[22px] font-semibold">{agentKey}</h1>
        <div className="mt-3 flex flex-wrap gap-2">
          <div className="relative">
            <SearchIcon className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" aria-hidden="true" />
            <Input aria-label={t("workspace.search")} data-search value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("workspace.search")} className="h-8 w-56 pl-8" />
          </div>
          <Button type="button" variant="outline" size="sm" onClick={() => setLayout(layout === "grid" ? "list" : "grid")}>
            {layout === "grid" ? t("workspace.view.list") : t("workspace.view.grid")}
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={() => setAdding(true)}>{t("workspace.add")}</Button>
        </div>
        <div className="mt-2 flex flex-wrap gap-2">
          {tags.map((item) => (
            <Button key={item} type="button" variant="outline" size="sm" onClick={() => setTag(tag === item ? null : item)}>
              {item}
            </Button>
          ))}
        </div>
        {selected.length > 0 ? (
          <div className="mt-2 flex gap-2">
            <Button type="button" variant="outline" size="sm" onClick={() => setPending({ ids: selected, kind: "remove" })}>{t("workspace.batch.remove")}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => setPending({ ids: selected, kind: "delete" })}>{t("workspace.batch.delete")}</Button>
          </div>
        ) : null}
        {notice ? <p role="status">{notice}</p> : null}
        <ul data-layout={layout} className={layout === "grid" ? "mt-4 grid grid-cols-2 gap-3" : "mt-4 space-y-2"}>
          {visible.map((card) => (
            <li key={card.id}>
              <Card className="group/card gap-2 rounded-lg border-border px-3 py-3 shadow-none">
                <div className="flex items-center gap-2">
                  <input type="checkbox" className={`size-4 rounded border-input ${selected.length > 0 ? "opacity-100" : "opacity-0 group-hover/card:opacity-100"}`} aria-label={`Select ${card.name}`} checked={selected.includes(card.id)} onChange={() => setSelected((current) => current.includes(card.id) ? current.filter((id) => id !== card.id) : [...current, card.id])} />
                  <button type="button" className="font-medium" onClick={() => { setOpenId(card.id); setSheet("local"); if (agentKey) void silent<{ body: string; description: string }>("get_global_local_skill_document", { tool: agentKey, name: card.name }).then((doc) => { if (doc?.body) setRows((current) => current.map((row) => row.id === card.id ? { ...row, body: doc.body } : row)); }); }}>{card.name}</button>
                  <Badge variant="outline" className="ml-auto font-normal">
                    {card.status === "local_only" ? t("workspace.localOnly") : card.status === "center_changed" ? t("workspace.centerChanged") : t("workspace.inSync")}
                  </Badge>
                </div>
                <div className="flex flex-wrap gap-1">
                  <Button type="button" variant="ghost" size="sm" onClick={() => setPending({ ids: [card.id], kind: "upload" })}>{t("workspace.upload")}</Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => setPending({ ids: [card.id], kind: "pull" })}>{t("workspace.pull")}</Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => setPending({ ids: [card.id], kind: "remove" })}>{t("workspace.remove")}</Button>
                  <Button type="button" variant="ghost" size="sm" className="text-destructive" onClick={() => setPending({ ids: [card.id], kind: "delete" })}>{t("workspace.delete")}</Button>
                </div>
              </Card>
            </li>
          ))}
        </ul>
        {pending ? (
          <ConfirmDialog
            label={t("workspace.confirm")}
            detail={pending.kind === "upload" ? t("workspace.upload") : pending.kind === "pull" ? t("workspace.pull") : pending.kind === "remove" ? t("workspace.remove") : t("workspace.delete")}
            confirmLabel={t("workspace.confirm")}
            onConfirm={applyPending}
          />
        ) : null}
        {adding ? (
          <div data-testid="add-sheet" className="mt-3 rounded-md border border-border p-3">
            <Input aria-label={t("workspace.picker.search")} value={pickerQuery} onChange={(event) => setPickerQuery(event.target.value)} className="mb-2 h-8" />
            {library.filter((skill) => skill.name.toLowerCase().includes(debouncedPicker.trim().toLowerCase())).map((skill) => (
              <button key={skill.id} type="button" className="mr-2 text-sm" onClick={() => { setRows((current) => current.some((row) => row.id === skill.id) ? current : [...current, { id: skill.id, name: skill.name, status: "in_sync", tags: skill.tags, body: skill.body, center: skill.sourceBody, managed: true }]); if (agentKey) void syncSkillToTool(skill.id, agentKey); setAdding(false); }}>
                {skill.name}
              </button>
            ))}
          </div>
        ) : null}
        {open ? (
          <aside data-testid="workspace-detail" className="mt-3 rounded-md border border-border p-3">
            <div className="flex gap-2">
              <Button type="button" variant="outline" size="sm" onClick={() => setSheet("local")}>{t("library.detail.local")}</Button>
              <Button type="button" variant="outline" size="sm" onClick={() => setSheet("diff")}>{t("library.detail.diff")}</Button>
              <Button type="button" variant="outline" size="sm" onClick={() => setSheet("center")}>{t("workspace.detail.center")}</Button>
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
