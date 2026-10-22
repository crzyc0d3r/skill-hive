import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useContext, useEffect, useMemo, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { lcsDiff, moveById, presetFill, presetLabel, type SkillCard } from "@/features/library/model";
import { useShellBand } from "@/lib/shell-width";
import { GripVerticalIcon, MoreHorizontalIcon } from "lucide-react";
import type { AgentRow, PresetRow } from "@/lib/ipc/catalog";
import { deleteManagedSkills, silent, syncSkillToTool, unsyncSkillFromTool } from "@/lib/ipc/actions";
import { useDebounced } from "@/lib/use-debounced";
import { t } from "@/lib/i18n";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { EmptyState, ErrorState, Screen, SearchField } from "@/components/chrome";
import { WorkspaceSkeleton } from "@/components/skeletons";
import { RetryContext, useViewPhase } from "@/lib/view-state";
import { AgentName } from "@/components/app/AgentBadge";
import { agentDisplayName } from "@/lib/agents";

export interface WorkspaceCard {
  id: string;
  name: string;
  status: "local_only" | "in_sync" | "center_changed";
  tags: string[];
  body: string;
  center: string;
  managed: boolean;
  locations?: string[];
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
  const phase = useViewPhase();
  const [params] = useSearchParams();
  const [creating, setCreating] = useState(false);
  const [showAllAgents, setShowAllAgents] = useState(false);
  const retry = useContext(RetryContext);
  const [applied, setApplied] = useState<string | null>(null);
  const [name, setName] = useState("Research");
  const [rows, setRows] = useState(cards);
  useEffect(() => {
    setRows(cards);
  }, [cards]);
  const [query, setQuery] = useState("");
  const [tag, setTag] = useState<string | null>(null);
  const [menuId, setMenuId] = useState<string | null>(null);
  const band = useShellBand();
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
  const [presetOrder, setPresetOrder] = useState<string[] | null>(null);
  const [presetDrag, setPresetDrag] = useState<string | null>(null);
  const orderedPresets = useMemo(() => {
    const ids = presetOrder ?? shownPresets.map((preset) => preset.id);
    return [...shownPresets].sort((left, right) => ids.indexOf(left.id) - ids.indexOf(right.id));
  }, [presetOrder, shownPresets]);

  function skillCount(count: number) {
    return count === 1 ? t("workspace.skillCount.one") : t("workspace.skillsCount", { count: String(count) });
  }

  const debouncedQuery = useDebounced(query, 150);
  const debouncedPicker = useDebounced(pickerQuery, 150);
  const tags = useMemo(() => [...new Set(rows.flatMap((row) => row.tags))], [rows]);
  const visible = rows.filter((row) => {
    if (tag && !row.tags.includes(tag)) return false;
    if (!debouncedQuery.trim()) return true;
    return row.name.toLowerCase().includes(debouncedQuery.trim().toLowerCase());
  });
  const open = rows.find((row) => row.id === openId) ?? null;
  useEffect(() => {
    if (!openId || sheet !== "center") return;
    let live = true;
    void silent<{ body?: string }>("get_skill_document", { skillId: openId }).then((doc) => {
      if (!live || !doc?.body) return;
      setRows((current) => current.map((row) => (row.id === openId && !row.center ? { ...row, center: doc.body ?? "" } : row)));
    });
    return () => {
      live = false;
    };
  }, [openId, sheet]);

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

  if (!agentKey && params.get("view") === "presets") {
    return (
      <Screen testId="global-workspace" title={t("workspace.presets")} lede={t("workspace.presets.lede")}>
        {phase === "loading" ? <WorkspaceSkeleton /> : null}
        {phase === "error" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
        {phase === "empty" ? <EmptyState title={t("workspace.presets")} body={t("workspace.presets.lede")} /> : null}
        {phase === "ready" ? (
          <ol className="space-y-3">
            {orderedPresets.map((preset) => (
              <li
                key={preset.id}
                draggable
                onDragStart={() => setPresetDrag(preset.id)}
                onDragOver={(event) => event.preventDefault()}
                onDrop={() => {
                  if (!presetDrag) return;
                  const ids = presetOrder ?? orderedPresets.map((item) => item.id);
                  setPresetOrder(moveById(ids, presetDrag, preset.id, (id) => id));
                  setPresetDrag(null);
                }}
                className="rounded-lg border border-border bg-card p-3"
              >
                <div className="flex items-start gap-2">
                  <button type="button" aria-label={t("workspace.drag")} className="mt-0.5 cursor-grab text-muted-foreground">
                    <GripVerticalIcon className="size-4" aria-hidden="true" />
                  </button>
                  <div className="min-w-0 flex-1">
                    <p className="font-medium">{preset.name}</p>
                    <p className="text-xs text-muted-foreground">{skillCount(preset.skillIds.length)}</p>
                    <ol className="mt-2 space-y-1">
                      {preset.skillIds.map((id) => (
                        <li key={id} className="flex items-center gap-2 text-sm">
                          <GripVerticalIcon className="size-3.5 text-muted-foreground" aria-hidden="true" />
                          <span>{coverageSkills.find((skill) => skill.id === id)?.name ?? id}</span>
                        </li>
                      ))}
                    </ol>
                  </div>
                </div>
              </li>
            ))}
          </ol>
        ) : null}
      </Screen>
    );
  }

  if (!agentKey) {
    return (
      <Screen testId="global-workspace" title={t("workspace.title")} lede={t("workspace.lede")} actions={<Button type="button" size="sm" onClick={() => setCreating(true)}>{t("preset.new")}</Button>}>
        {creating ? (
          <form
            role="dialog"
            aria-label={t("preset.new")}
            className="mt-3 max-w-md space-y-3 rounded-lg border border-border bg-card p-4"
            onSubmit={(event) => {
              event.preventDefault();
              void onCreate(name);
              setCreating(false);
            }}
          >
            <label className="block text-sm">
              <span className="mb-1.5 block text-muted-foreground">{t("workspace.preset.hint")}</span>
              <Input aria-label={t("preset.create")} value={name} onChange={(event) => setName(event.target.value)} className="h-8" />
            </label>
            <Button type="submit" size="sm">{t("preset.new")}</Button>
          </form>
        ) : null}
        {phase === "loading" ? <WorkspaceSkeleton /> : null}
        {phase === "error" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
        {phase === "empty" ? <EmptyState title={t("workspace.title")} body={t("workspace.lede")} /> : null}
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
              <span key={preset.id} className="inline-flex items-center gap-1.5">
                <Button type="button" data-fill={fill} aria-pressed={applied === preset.id} variant="outline" size="sm" onClick={() => {
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
                  {fill === "partial" ? preset.name : presetLabel(preset.name, synced, total)}
                </Button>
                {fill === "partial" ? <span className="text-xs text-muted-foreground">{t("workspace.presetProgress", { synced: String(synced), total: String(total) })}</span> : null}
              </span>
            );
          })}
        </div>
        <ul className="mt-4 grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(16rem,1fr))]" data-testid="agent-catalog">
          {(showAllAgents ? agents : agents.filter((agent) => agent.installed).slice(0, 8).concat(agents.filter((agent) => agent.installed).length ? [] : agents.slice(0, 4))).map((agent) => {
            const count = coverageSkills.filter((skill) => skill.agents.includes(agent.key)).length;
            return (
            <li key={agent.key} className="flex h-full flex-col rounded-lg border border-border bg-card p-3 text-sm shadow-sm">
              <Link to={`/global-workspace/${agent.key}`} className="font-medium"><AgentName agent={agent.key} displayName={agent.name} /></Link>
              <div className="mt-1 truncate font-mono text-xs text-muted-foreground">{agent.path}</div>
              <p className="mt-2 text-sm">{skillCount(count)}</p>
              <Button asChild type="button" variant="outline" size="sm" className="mt-3 w-fit"><Link to={`/global-workspace/${agent.key}`}>{t("workspace.open")}</Link></Button>
            </li>
            );
          })}
        </ul>
        {showAllAgents ? null : <Button type="button" variant="ghost" size="sm" className="mt-3" onClick={() => setShowAllAgents(true)}>{t("workspace.showAll")}</Button>}
        </> : null}
      </Screen>
    );
  }

  const agent = agents.find((item) => item.key === agentKey);
  if (phase !== "ready") {
    return (
      <Screen testId="global-workspace" title={agentDisplayName(agentKey, agents)} lede={agent?.path ? <span className="font-mono text-xs">{agent.path}</span> : undefined}>
        {phase === "loading" ? <WorkspaceSkeleton /> : null}
        {phase === "error" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
        {phase === "empty" ? <EmptyState title={t("workspace.emptyTitle")} body={t("workspace.emptyBody")} action={<Button type="button" size="sm" onClick={() => setAdding(false)}>{t("workspace.add")}</Button>} /> : null}
      </Screen>
    );
  }

  return (
    <Screen testId="global-workspace" title={agentDisplayName(agentKey, agents)} lede={agent?.path ? <span className="font-mono text-xs">{agent.path}</span> : undefined} actions={<Button type="button" size="sm" onClick={() => setAdding(true)}>{t("workspace.add")}</Button>}>
      <div data-testid="agent-page">
        <div className="flex flex-wrap items-center gap-2" data-testid="apply-preset">
          <span className="text-sm text-muted-foreground">{t("workspace.apply")}</span>
          {orderedPresets.map((preset) => (
            <Button key={preset.id} type="button" variant="outline" size="sm" aria-pressed={applied === preset.id} onClick={() => { setApplied(preset.id); void onApply(preset.id); }}>
              {preset.name}
            </Button>
          ))}
        </div>
        <p className="mt-3 mb-4 max-w-prose text-sm text-muted-foreground">{t("workspace.centerChangedHelp")}</p>
        <div className="flex flex-wrap items-center gap-2">
          <SearchField aria-label={t("workspace.search")} data-search value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("workspace.search")} />
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
        <div className="mt-4 rounded-lg border border-border">
        <table data-testid="workspace-cards" data-layout="table" className="w-full text-sm">
          <thead className="bg-muted/40 text-left text-xs text-muted-foreground">
            <tr>
              <th className="w-10 px-3 py-2" />
              <th className="px-3 py-2">{t("workspace.col.skill")}</th>
              {band === 800 ? null : <th className="px-3 py-2">{t("workspace.from")}</th>}
              <th className="w-12 px-3 py-2" />
            </tr>
          </thead>
          <tbody>
          {visible.map((card) => (
            <tr key={card.id} className="border-t border-border">
              <td className="px-3 py-2">
                <input type="checkbox" className="size-4 rounded border-input" aria-label={`Select ${card.name}`} checked={selected.includes(card.id)} onChange={() => setSelected((current) => current.includes(card.id) ? current.filter((id) => id !== card.id) : [...current, card.id])} />
              </td>
              <td className="px-3 py-2">
                <button type="button" className="font-medium" onClick={() => { setOpenId(card.id); setSheet("local"); if (agentKey) void silent<{ body: string; description: string }>("get_global_local_skill_document", { tool: agentKey, name: card.name }).then((doc) => { if (doc?.body) setRows((current) => current.map((row) => row.id === card.id ? { ...row, body: doc.body } : row)); }); }}>{card.name}</button>
                <Badge variant="outline" className="ml-2 font-normal" title={card.status === "center_changed" ? t("workspace.centerChangedHelp") : undefined}>
                  {card.status === "local_only" ? t("workspace.localOnly") : card.status === "center_changed" ? t("workspace.centerChanged") : t("workspace.inSync")}
                </Badge>
                {(card.locations?.length ?? 0) > 1 ? (
                  <p className="text-xs text-muted-foreground">{t("library.locations", { count: String(card.locations?.length ?? 0) })}</p>
                ) : null}
              </td>
              {band === 800 ? null : <td className="px-3 py-2 text-muted-foreground">{card.status === "local_only" ? t("workspace.foundHere") : t("workspace.fromLibrary")}</td>}
              <td className="relative px-3 py-2 text-right">
                <Button type="button" variant="ghost" size="icon-sm" aria-label={`${t("shell.more")} ${card.name}`} aria-expanded={menuId === card.id} onClick={() => setMenuId(menuId === card.id ? null : card.id)}>
                  <MoreHorizontalIcon aria-hidden="true" />
                </Button>
                {menuId === card.id ? (
                  <div role="menu" className="absolute right-3 z-20 mt-1 w-44 rounded-md border border-border bg-popover p-1 text-left shadow-md">
                    <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); setPending({ ids: [card.id], kind: "upload" }); }}>{t("workspace.upload")}</button>
                    <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); setPending({ ids: [card.id], kind: "pull" }); }}>{t("workspace.pull")}</button>
                    <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); setPending({ ids: [card.id], kind: "remove" }); }}>{t("workspace.remove")}</button>
                    <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm text-destructive hover:bg-accent" onClick={() => { setMenuId(null); setPending({ ids: [card.id], kind: "delete" }); }}>{t("workspace.delete")}</button>
                  </div>
                ) : null}
              </td>
            </tr>
          ))}
          </tbody>
        </table>
        </div>
        {pending ? (
          <ConfirmDialog
            label={t("workspace.confirm")}
            detail={pending.kind === "upload" ? t("workspace.upload") : pending.kind === "pull" ? t("workspace.pull") : pending.kind === "remove" ? t("workspace.remove") : t("workspace.delete")}
            confirmLabel={t("workspace.confirm")}
            onConfirm={applyPending}
          />
        ) : null}
        {adding ? (
          <div data-testid="add-sheet" className="mt-3 rounded-lg border border-border bg-card p-3">
            <Input aria-label={t("workspace.picker.search")} value={pickerQuery} onChange={(event) => setPickerQuery(event.target.value)} className="mb-2 h-8" />
            {library.filter((skill) => skill.name.toLowerCase().includes(debouncedPicker.trim().toLowerCase())).map((skill) => (
              <button key={skill.id} type="button" className="mr-2 text-sm" onClick={() => { setRows((current) => current.some((row) => row.id === skill.id) ? current : [...current, { id: skill.id, name: skill.name, status: "in_sync", tags: skill.tags, body: skill.body, center: skill.sourceBody, managed: true }]); if (agentKey) void syncSkillToTool(skill.id, agentKey); setAdding(false); }}>
                {skill.name}
              </button>
            ))}
          </div>
        ) : null}
        {open ? (
          <aside data-testid="workspace-detail" className="mt-3 rounded-lg border border-border bg-card p-4">
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
    </Screen>
  );
}
