import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useContext, useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { CheckIcon, GripVerticalIcon, LayoutGridIcon, ListIcon, MoreHorizontalIcon, PlusIcon, XIcon } from "lucide-react";
import { fileDiffs, lcsDiff, filterSkills, groupSkillCards, moveById, type LibraryFilter, type SkillCard, type SkillFile } from "@/features/library/model";
import { useDebounced } from "@/lib/use-debounced";
import { badgeFor } from "@/features/generate/model";
import { useGenerate } from "@/features/generate/store";
import { applyAvailableSkillUpdates, checkSkillUpdate, silent } from "@/lib/ipc/actions";
import { listAgents, type AgentRow } from "@/lib/ipc/catalog";
import { agentDisplayName, detectedAgents } from "@/lib/agents";
import { AgentMark } from "@/components/app/AgentBadge";
import { SourceBadge } from "@/components/app/SourceBadge";
import { sourceLabel } from "@/lib/source";
import { useShellBand } from "@/lib/shell-width";
import { toast } from "sonner";
import { useVirtualGrid } from "@/lib/virtual-grid";
import { t } from "@/lib/i18n";
import { cn } from "cn";
import { GitStatusButton } from "@/features/library/GitStatusButton";
import { UpdateFlow } from "@/features/library/UpdateFlow";
import { EmptyState, ErrorState, PrimaryLink, Screen, SearchField } from "@/components/chrome";
import { LibrarySkeleton } from "@/components/skeletons";
import { RetryContext, useViewPhase } from "@/lib/view-state";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";

const menuItem = "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm outline-none hover:bg-accent hover:text-accent-foreground";

function AgentPlacement({
  skill,
  agents,
  showAll,
  query,
  onQuery,
  onShowAll,
  onToggle,
}: {
  skill: SkillCard;
  agents: AgentRow[];
  showAll: boolean;
  query: string;
  onQuery: (value: string) => void;
  onShowAll: () => void;
  onToggle: (agent: string) => void;
}) {
  const installedKeys = new Set(skill.agents);
  const installed = [
    ...agents.filter((agent) => installedKeys.has(agent.key)),
    ...skill.agents.filter((key) => !agents.some((agent) => agent.key === key)).map((key) => ({ key, name: agentDisplayName(key, agents) })),
  ];
  const needle = query.trim().toLowerCase();
  const choices = (showAll ? agents : detectedAgents(agents)).filter((agent) => {
    if (installedKeys.has(agent.key)) return false;
    if (!needle) return true;
    return agent.name.toLowerCase().includes(needle);
  });
  return (
    <div className="mt-3 space-y-3" data-testid="agent-placement">
      <div>
        <h2 className="text-sm font-medium">{t("library.installedIn")}</h2>
        {installed.length === 0 ? <p className="mt-1 text-sm text-muted-foreground">{t("library.installedNone")}</p> : null}
        <ul className="mt-2 space-y-2">
          {installed.map((agent) => {
            return (
              <li key={agent.key} className="flex items-center gap-2 rounded-md border border-border px-2 py-1.5 text-sm">
                <AgentMark agent={agent.key} name={agent.name} />
                <span className="min-w-0 flex-1">
                  <span className="block font-medium">{agent.name}</span>
                  <span className="block truncate font-mono text-xs text-muted-foreground" title={`~/.agents/${agent.key}/skills/${skill.name}`}>{`~/.agents/${agent.key}/skills/${skill.name}`}</span>
                </span>
                <span className="text-xs text-muted-foreground">{skill.updateStatus && skill.updateStatus !== "clean" ? t("library.outOfDate") : t("library.synced")}</span>
                <Button type="button" variant="ghost" size="sm" className="ml-auto" aria-label={`${t("library.removeAgent")} ${agent.name}`} onClick={() => onToggle(agent.key)}>
                  {t("library.removeAgent")}
                </Button>
              </li>
            );
          })}
        </ul>
        {(skill.locations?.length ?? 0) > 1 ? (
          <details className="mt-2 text-sm">
            <summary>{t("library.locations", { count: String(skill.locations?.length ?? 0) })}</summary>
            <ul className="mt-1 space-y-1 font-mono text-xs text-muted-foreground">
              {skill.locations?.map((path) => (
                <li key={path}>{path}</li>
              ))}
            </ul>
          </details>
        ) : null}
      </div>
      <Popover>
        <PopoverTrigger asChild>
          <Button type="button" variant="outline" size="sm">{t("library.addAgent")}</Button>
        </PopoverTrigger>
        <PopoverContent className="w-72 p-2" align="start">
          <Input aria-label={t("library.addAgent.search")} value={query} onChange={(event) => onQuery(event.target.value)} className="h-8" />
          <ul className="mt-2 max-h-56 space-y-1 overflow-auto">
            {choices.map((agent) => (
                <li key={agent.key}>
                  <button type="button" className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => onToggle(agent.key)}>
                    <AgentMark agent={agent.key} name={agent.name} />
                    {agent.name}
                  </button>
                </li>
            ))}
          </ul>
          <Button type="button" variant="ghost" size="sm" className="mt-1" aria-pressed={showAll} onClick={onShowAll}>
            {t("library.showAllAgents")}
          </Button>
        </PopoverContent>
      </Popover>
    </div>
  );
}

function InstalledTiles({ skill, agents }: { skill: SkillCard; agents: AgentRow[] }) {
  const installed = skill.agents.map((key) => ({ key, name: agentDisplayName(key, agents) }));
  const shown = installed.slice(0, 3);
  const extra = installed.length - shown.length;
  const names = installed.map((agent) => agent.name).join(", ");
  return (
    <div className="flex items-center justify-end gap-1" data-testid={`sync-dots-${skill.id}`} title={names}>
      {shown.map((agent) => (
        <span key={agent.key} data-filled="true" title={agent.name} aria-label={agent.name}>
          <AgentMark agent={agent.key} name={agent.name} />
        </span>
      ))}
      {extra > 0 ? <span className="text-xs text-muted-foreground" title={names}>+{extra}</span> : null}
    </div>
  );
}

export function LibraryView({
  skills,
  initialId,
  memberIds = [],
  presetName = "Research",
  onRenameTag,
  onAddTag,
  onSync,
  onToggleMember,
  onReorderMembers,
  onDeleteSkills,
  onDeleteTag,
  onRemoveTag,
  onSetEnabled,
  onBatchTag,
  onToggleAgent,
}: {
  skills: SkillCard[];
  initialId?: string | null;
  memberIds?: string[];
  presetName?: string;
  onRenameTag: (from: string, to: string) => void | Promise<void>;
  onAddTag: (skillId: string, tag: string) => void | Promise<void>;
  onSync: (ids: string[], agents: string[]) => void | Promise<void>;
  onToggleMember: (skillId: string) => void | Promise<void>;
  onReorderMembers: (ids: string[]) => void | Promise<void>;
  onDeleteSkills: (ids: string[]) => void | Promise<void>;
  onDeleteTag: (tag: string) => void | Promise<void>;
  onRemoveTag: (skillId: string, tag: string) => void | Promise<void>;
  onSetEnabled: (ids: string[], enabled: boolean) => void | Promise<void>;
  onBatchTag: (ids: string[], tag: string) => void | Promise<void>;
  onToggleAgent: (skillId: string, agent: string) => void | Promise<void>;
}) {
  const phase = useViewPhase();
  const [tagsOpen, setTagsOpen] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const band = useShellBand();
  const retry = useContext(RetryContext);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<LibraryFilter>("all");
  const [source, setSource] = useState<string | null>(null);
  const [tag, setTag] = useState<string | null>(null);
  const [layout, setLayout] = useState<"grid" | "list">("grid");
  const [selected, setSelected] = useState<string[]>([]);
  const [selecting, setSelecting] = useState(false);
  const [openId, setOpenId] = useState<string | null>(initialId ?? null);
  const [placements, setPlacements] = useState<{ id: string; name: string }[]>([]);
  useEffect(() => {
    if (initialId) setOpenId(initialId);
  }, [initialId]);
  const [tab, setTab] = useState<"local" | "diff" | "source" | "edit">("local");
  const [rename, setRename] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [syncOpen, setSyncOpen] = useState(false);
  const [tagDraft, setTagDraft] = useState<Record<string, string>>({});
  const [menuId, setMenuId] = useState<string | null>(null);
  const [checked, setChecked] = useState<{ name: string; status: string } | null>(null);
  const [dragId, setDragId] = useState<string | null>(null);
  const [batchTag, setBatchTag] = useState("");
  const [batchOpen, setBatchOpen] = useState(false);
  const [catalogAgents, setCatalogAgents] = useState<AgentRow[]>([]);
  const [showAllAgents, setShowAllAgents] = useState(false);
  const [agentQuery, setAgentQuery] = useState("");
  const [sourceRef, setSourceRef] = useState<string | null>(null);
  const { seed } = useGenerate();
  useEffect(() => {
    void listAgents().then((rows) => {
      if (Array.isArray(rows)) setCatalogAgents(rows);
    });
  }, []);
  useEffect(() => {
    if (!openId) return;
    void silent<{ path?: string }>("get_source_skill_document", { skillId: openId }).then((doc) => {
      setSourceRef(doc?.path ?? null);
    });
  }, [openId]);
  const [detail, setDetail] = useState<{ id: string; body: string } | null>(null);
  useEffect(() => {
    if (!openId) return;
    const card = skills.find((skill) => skill.id === openId);
    if (!card || card.body) {
      setDetail(null);
      return;
    }
    let live = true;
    setDetail(null);
    void silent<{ body?: string }>("get_skill_document", { skillId: openId }).then((doc) => {
      if (live) setDetail({ id: openId, body: doc?.body ?? "" });
    });
    return () => {
      live = false;
    };
  }, [openId, skills]);

  const [fileMap, setFileMap] = useState<Record<string, SkillFile[]>>({});
  useEffect(() => {
    if (!openId || tab !== "diff") return;
    const card = skills.find((skill) => skill.id === openId);
    if (!card || (card.files && card.files.length > 0) || fileMap[openId]) return;
    let live = true;
    void silent<SkillFile[]>("get_skill_files", { skillId: openId }).then((files) => {
      if (live && Array.isArray(files)) setFileMap((current) => ({ ...current, [openId]: files }));
    });
    return () => {
      live = false;
    };
  }, [openId, tab, skills, fileMap]);

  const debouncedQuery = useDebounced(query, 150);
  const grouped = useMemo(() => groupSkillCards(skills), [skills]);
  const sources = useMemo(() => [...new Set(grouped.map((skill) => skill.source))], [grouped]);
  const tags = useMemo(() => [...new Set(grouped.flatMap((skill) => skill.tags))], [grouped]);
  const visible = useMemo(() => filterSkills(grouped, debouncedQuery, filter, source, tag), [grouped, debouncedQuery, filter, source, tag]);
  const grid = useVirtualGrid(visible, layout);
  const open = grouped.find((skill) => skill.id === openId) ?? null;
  const openBody = open && !open.body && detail?.id === open.id ? detail.body : (open?.body ?? "");
  const rendered = grid.rendered;
  const members = memberIds.map((id) => skills.find((skill) => skill.id === id)?.name ?? id);
  const showChecks = selecting || selected.length > 0;

  function toggleSelected(id: string) {
    setSelected((current) => {
      const next = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
      if (next.length > 0) setSelecting(true);
      return next;
    });
  }

  return (
    <Screen testId="library" title={t("library.title")} lede={t("library.emptyBody")} actions={<PrimaryLink to="/install">{t("dashboard.quick.install")}</PrimaryLink>}>
      <div className="flex flex-wrap items-center gap-2">
        <SearchField
          aria-label={t("library.search")}
          placeholder={t("library.search")}
          data-search
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <div role="group" aria-label={t("library.title")} className="inline-flex rounded-lg bg-muted p-0.5">
          {(["all", "enabled", "available"] as const).map((item) => (
            <Button
              key={item}
              type="button"
              size="sm"
              variant="ghost"
              aria-pressed={filter === item}
              className={cn("h-7 rounded-md px-3 shadow-none", filter === item ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:bg-transparent")}
              onClick={() => setFilter(item)}
            >
              {t(`library.filter.${item}`)}
            </Button>
          ))}
        </div>
        <Button type="button" variant="outline" size="sm" aria-pressed={tagsOpen} onClick={() => setTagsOpen((current) => !current)}>{t("library.filterTags")}</Button>
        <div className="ml-auto flex items-center gap-1">
          <div>
            <Button type="button" variant="outline" size="sm" aria-expanded={moreOpen} onClick={() => setMoreOpen((current) => !current)}>{t("shell.more")}</Button>
            <div className={moreOpen ? "mt-2 flex flex-col gap-1" : "sr-only"}>
              <GitStatusButton />
              <UpdateFlow skills={skills} />
            </div>
          </div>
          <Button type="button" size="icon-sm" variant={layout === "grid" ? "secondary" : "ghost"} aria-label={t("library.view.grid")} title={t("library.view.grid")} onClick={() => setLayout("grid")}>
            <LayoutGridIcon aria-hidden="true" />
          </Button>
          <Button type="button" size="icon-sm" variant={layout === "list" ? "secondary" : "ghost"} aria-label={t("library.view.list")} title={t("library.view.list")} onClick={() => setLayout("list")}>
            <ListIcon aria-hidden="true" />
          </Button>
          <Button
            type="button"
            size="icon-sm"
            variant={showChecks ? "secondary" : "ghost"}
            aria-pressed={showChecks}
            aria-label="Selection"
            title="Selection"
            onClick={() => {
              if (showChecks) {
                setSelecting(false);
                setSelected([]);
              } else {
                setSelecting(true);
              }
            }}
          >
            <CheckIcon aria-hidden="true" />
          </Button>
        </div>
      </div>
      <div className={tagsOpen ? "flex flex-wrap items-center gap-1.5" : "sr-only"}>
        {sources.map((item) => (
          <Button key={item} type="button" variant={source === item ? "secondary" : "outline"} size="xs" className="rounded-full shadow-none" onClick={() => setSource(source === item ? null : item)}>
            {sourceLabel(item)}
          </Button>
        ))}
        <Button type="button" variant={tag === "untagged" ? "secondary" : "outline"} size="xs" className="rounded-full shadow-none" onClick={() => setTag(tag === "untagged" ? null : "untagged")}>
          Untagged
        </Button>
        {tags.map((item) => (
          <Button
            key={item}
            type="button"
            variant={tag === item ? "secondary" : "outline"}
            size="xs"
            className="rounded-full shadow-none"
            onClick={() => setTag(tag === item ? null : item)}
            onContextMenu={(event) => { event.preventDefault(); setRename(item); setRenameValue(item); }}
          >
            {item}
          </Button>
        ))}
      </div>
      {rename ? (
        <form
          className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-card p-2"
          onSubmit={(event) => {
            event.preventDefault();
            onRenameTag(rename, renameValue);
            setRename(null);
          }}
        >
          <Input aria-label={t("library.tag.rename")} value={renameValue} onChange={(event) => setRenameValue(event.target.value)} className="h-8 w-48" />
          <Button type="submit" size="sm">{t("library.tag.rename")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { onDeleteTag(rename); setRename(null); }}>
            {t("library.tag.delete")}
          </Button>
        </form>
      ) : null}
      <details className="sr-only">
        <summary className="cursor-pointer text-sm font-medium">{t("library.presetOrder")}</summary>
      <ol data-testid="preset-order" className="mt-2 space-y-1">
        {memberIds.map((id) => {
          const name = skills.find((skill) => skill.id === id)?.name ?? id;
          return (
            <li
              key={id}
              draggable
              data-member={id}
              onDragStart={() => setDragId(id)}
              onDragOver={(event) => event.preventDefault()}
              onDrop={() => {
                if (!dragId) return;
                onReorderMembers(moveById(memberIds, dragId, id, (item) => item));
                setDragId(null);
              }}
              className="flex items-center gap-2 rounded-md px-1 py-1 text-sm"
            >
              <button type="button" className="text-muted-foreground" title={t("library.reorder")} aria-label={t("library.reorder")}>
                <GripVerticalIcon className="size-3" aria-hidden="true" />
              </button>
              <span>{name}{"\u200b"}</span>
            </li>
          );
        })}
      </ol>
      </details>
      <p className="sr-only">{presetName}: {members.join(", ")}</p>
      {checked ? (
        <p role="status">
          <span>{t("library.checked")} {checked.name}</span>
          {checked.status ? <span> {checked.status}</span> : null}
        </p>
      ) : null}
      {selected.length > 0 ? (
        <div data-testid="bulk-bar" className="sticky top-0 z-20 flex flex-wrap items-center gap-2 rounded-lg border border-border bg-card px-3 py-2 shadow-sm">
          <span className="text-xs text-muted-foreground">{t("library.selected", { count: String(selected.length) })}</span>
          <Button type="button" size="sm" onClick={() => { const ids = [...selected]; void onSetEnabled(ids, true); toast.success(t("library.batch.enable"), { action: { label: t("shell.undo"), onClick: () => void onSetEnabled(ids, false) } }); }}>{t("library.batch.enable")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { const ids = [...selected]; void onSetEnabled(ids, false); toast.success(t("library.batch.disable"), { action: { label: t("shell.undo"), onClick: () => void onSetEnabled(ids, true) } }); }}>{t("library.batch.disable")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => setBatchOpen(true)}>{t("library.batch.tags")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { onDeleteSkills(selected); setSelected([]); }}>{t("library.batch.delete")}</Button>
          <Button type="button" variant="ghost" size="sm" onClick={() => setSelected([])}>{t("library.bulkClear")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { void applyAvailableSkillUpdates(); void silent("batch_update_skills", { ids: selected }); }}>{t("library.batch.update")}</Button>
          <Button type="button" size="sm" onClick={() => setSyncOpen(true)}>
            {t("library.sync")}
          </Button>
        </div>
      ) : null}
      {batchOpen ? (
        <form
          className="flex gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            if (batchTag.trim()) onBatchTag(selected, batchTag.trim());
            setBatchOpen(false);
            setBatchTag("");
          }}
        >
          <Input aria-label={t("library.batch.tags")} value={batchTag} onChange={(event) => setBatchTag(event.target.value)} className="h-8" />
          <Button type="submit" variant="outline" size="sm">{t("library.batch.tags")}</Button>
        </form>
      ) : null}
      {syncOpen ? (
        <form
          className="rounded-lg border border-border bg-card p-3"
          onSubmit={(event) => {
            event.preventDefault();
            const data = new FormData(event.currentTarget);
            const chosen = (showAllAgents ? catalogAgents : detectedAgents(catalogAgents))
              .filter((agent) => data.get(agent.key) === "on")
              .map((agent) => agent.key);
            const previous = selected.map((id) => ({ id, agents: skills.find((skill) => skill.id === id)?.agents ?? [] }));
            void onSync(selected, chosen);
            toast.success(t("library.sync"), { action: { label: t("shell.undo"), onClick: () => { for (const row of previous) void onSync([row.id], row.agents); } } });
            setSyncOpen(false);
          }}
        >
          {(showAllAgents ? catalogAgents : detectedAgents(catalogAgents)).map((agent) => (
            <label key={agent.key} className="mr-3 text-sm">
              <input type="checkbox" name={agent.key} /> {agent.name}
            </label>
          ))}
          <Button type="button" variant="ghost" size="sm" aria-pressed={showAllAgents} onClick={() => setShowAllAgents((current) => !current)}>{t("library.showAllAgents")}</Button>
          <Button type="submit" size="sm">{t("library.sync")}</Button>
        </form>
      ) : null}
      {phase === "loading" ? <LibrarySkeleton /> : null}
      {phase === "error" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
      {phase === "empty" || (phase === "ready" && rendered.length === 0) ? (
        <EmptyState title={t("library.emptyTitle")} body={t("library.emptyBody")} action={<PrimaryLink to="/install">{t("dashboard.quick.install")}</PrimaryLink>} />
      ) : null}
      <div className={cn("flex min-w-0 flex-col gap-4", open && band >= 1440 && "flex-row items-start")}>
      {phase === "ready" ? <div ref={grid.ref} className="max-h-[36rem] min-w-0 flex-1 overflow-auto">
      <ul
        data-testid="library-cards"
        data-layout={layout}
        data-columns={open && band >= 1920 ? 3 : open && band >= 1440 ? 2 : grid.columns}
        className={layout === "grid" ? "grid items-stretch gap-3" : "space-y-2"}
        style={layout === "grid" ? { gridTemplateColumns: `repeat(${open && band >= 1920 ? 3 : open && band >= 1440 ? 2 : grid.columns}, minmax(0, 1fr))` } : undefined}
      >
        {rendered.map((skill) => (
          <li key={skill.id} data-virtual-card="" className="h-full">
            <Card className={cn("group/card flex h-full flex-col gap-0 overflow-hidden rounded-lg border-border bg-card py-0 shadow-none transition-colors hover:border-foreground/15", layout === "list" && "flex-row")}>
              <div className="flex items-start gap-2.5 px-3 pt-3">
                <input
                  type="checkbox"
                  aria-label={`Select ${skill.name}`}
                  checked={selected.includes(skill.id)}
                  onChange={() => toggleSelected(skill.id)}
                  className="mt-1 size-4 shrink-0 rounded border-border accent-primary"
                />
                <button type="button" className="min-w-0 flex-1 text-left" onClick={() => { setOpenId(skill.id); setTab("local"); }}>
                  <span className="block font-medium whitespace-normal">{skill.name}</span>
                  <span className="mt-0.5 line-clamp-2 block text-[13px] leading-5 text-muted-foreground">{skill.description}</span>
                </button>
                <DropdownMenu modal={false} open={menuId === skill.id} onOpenChange={(next) => setMenuId(next ? skill.id : null)}>
                  <DropdownMenuTrigger asChild>
                    <Button type="button" variant="ghost" size="icon-xs" aria-label={t("library.menu")} title={t("library.menu")} className="text-muted-foreground opacity-70 group-hover/card:opacity-100 data-[state=open]:opacity-100">
                      <MoreHorizontalIcon aria-hidden="true" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-44">
                    <Link to={`/skills/${skill.id}/edit`} className={menuItem}>{t("library.menu.edit")}</Link>
                    <Link to={`/skills/${skill.id}/tests`} className={menuItem} aria-label={t("library.runTests")}>{t("library.menu.tests")}</Link>
                    <button type="button" className={menuItem} onClick={() => { setChecked({ name: skill.name, status: "" }); void checkSkillUpdate(skill.id).then((status) => setChecked({ name: skill.name, status: status || "" })); }}>{t("library.menu.check")}</button>
                    <button type="button" className={menuItem} onClick={() => { void checkSkillUpdate(skill.id).then((status) => { setChecked({ name: skill.name, status: status || "" }); void silent("update_skill", { skillId: skill.id }); }); if (skill.source === "local") void silent("reimport_local_skill", { skillId: skill.id }); }}>{skill.source === "local" ? t("library.menu.reimport") : t("library.menu.update")}</button>
                    <button type="button" className={cn(menuItem, "text-destructive")} onClick={() => onDeleteSkills([skill.id])}>{t("library.menu.delete")}</button>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
              <div className="mt-2 flex flex-wrap items-center gap-1 px-3">
                {skill.draft ? <Badge data-testid="draft-badge" variant="warning">{t("library.badge.draft")}</Badge> : null}
                {badgeFor(seed, skill.id) === "tested" ? <Badge data-testid="tested-badge" variant="success">{t("library.badge.tested")}</Badge> : null}
                {badgeFor(seed, skill.id) === "regressed" ? <Badge data-testid="regressed-badge" variant="warning">{t("library.badge.regressed")}</Badge> : null}
                {skill.tags.map((item) => (
                  <Badge key={item} variant="secondary" className="group/tag gap-0.5 pr-1 font-normal">
                    {item}
                    <button type="button" aria-label={`${t("library.tag.remove")} ${item}`} title={`${t("library.tag.remove")} ${item}`} className="rounded-full opacity-0 group-hover/tag:opacity-100" onClick={() => onRemoveTag(skill.id, item)}>
                      <XIcon className="size-3" aria-hidden="true" />
                    </button>
                  </Badge>
                ))}
                <Popover>
                  <PopoverTrigger asChild>
                    <Button type="button" variant="ghost" size="icon-xs" aria-label={t("library.tag.add")} title={t("library.tag.add")} className="opacity-0 group-hover/card:opacity-100">
                      <PlusIcon aria-hidden="true" />
                    </Button>
                  </PopoverTrigger>
                  <PopoverContent className="w-56 p-2" align="start">
                    <form
                      className="flex gap-1"
                      onSubmit={(event) => {
                        event.preventDefault();
                        const value = tagDraft[skill.id]?.trim();
                        if (value) onAddTag(skill.id, value);
                        setTagDraft((current) => ({ ...current, [skill.id]: "" }));
                      }}
                    >
                      <Input aria-label={`${t("library.tag.add")} ${skill.name}`} value={tagDraft[skill.id] ?? ""} onChange={(event) => setTagDraft((current) => ({ ...current, [skill.id]: event.target.value }))} className="h-8" />
                      <Button type="submit" size="sm">{t("library.tag.add")}</Button>
                    </form>
                  </PopoverContent>
                </Popover>
              </div>
              <div className="mt-auto space-y-2 border-t border-border px-3 py-2">
                <div className="flex flex-wrap items-center gap-2">
                  <SourceBadge source={skill.source} />
                  <span className="relative inline-flex size-4 shrink-0 items-center justify-center" title={t("library.membership")}>
                    <span
                      aria-hidden="true"
                      className={cn("size-2 rounded-full", memberIds.includes(skill.id) ? "bg-primary shadow-[0_0_0_3px_var(--muted)]" : "bg-muted-foreground/30")}
                    />
                    <Checkbox
                      aria-label={`${t("library.membership")} ${skill.name}`}
                      checked={memberIds.includes(skill.id)}
                      onCheckedChange={() => onToggleMember(skill.id)}
                      className="absolute inset-0 opacity-0 focus-visible:opacity-100"
                    />
                  </span>
                  {memberIds.includes(skill.id) ? <span className="whitespace-nowrap text-xs text-muted-foreground">{presetName}</span> : null}
                </div>
                  <span className="text-xs text-muted-foreground">{skill.agents.length === 1 ? t("library.inAgent") : t("library.inAgents", { count: String(skill.agents.length) })}</span>
                  <InstalledTiles skill={skill} agents={catalogAgents} />
              </div>
            </Card>
          </li>
        ))}
      </ul>
      </div> : null}
      {open && phase === "ready" ? (
        <aside data-testid="detail-panel" className={cn("rounded-lg border border-border bg-card p-3", band < 1440 && "fixed inset-y-0 right-0 z-30 w-[min(560px,100%)] overflow-auto shadow-lg", band >= 1920 ? "w-[520px] shrink-0" : band >= 1440 ? "w-[440px] shrink-0" : undefined)}>
          <div className="mb-3 flex items-start justify-between gap-3">
            <div className="min-w-0">
              <h2 className="font-sans text-title font-semibold">{open.name}</h2>
              <p className="mt-1 text-sm text-muted-foreground">{open.description}</p>
            </div>
            <Button asChild size="sm"><Link to={`/skills/${open.id}/edit`}>{t("library.detail.edit")}</Link></Button>
          </div>
          <div className="inline-flex rounded-lg bg-muted p-0.5">
            {(["local", "diff", "source", "edit"] as const).map((item) => (
              <Button key={item} type="button" aria-pressed={tab === item} variant="ghost" size="sm" className={cn("h-7", tab === item && "bg-background shadow-sm")} onClick={() => setTab(item)}>
                {item === "local" ? t("library.detail.content") : item === "diff" ? t("library.detail.changes") : item === "source" ? t("library.detail.tests") : t("library.detail.history")}
              </Button>
            ))}
          </div>
          <AgentPlacement
            skill={open}
            agents={catalogAgents}
            showAll={showAllAgents}
            query={agentQuery}
            onQuery={setAgentQuery}
            onShowAll={() => setShowAllAgents((current) => !current)}
            onToggle={(agent) => onToggleAgent(open.id, agent)}
          />
          {tab === "edit" ? <Link to={`/skills/${open.id}/edit`}>{t("library.detail.edit")}</Link> : null}
          {tab === "local" && open && !open.body && detail?.id !== open.id ? <div data-testid="detail-loading" className="mt-3 h-24 animate-pulse rounded-md bg-muted" /> : null}
          {tab === "local" ? (
            <details className="mt-3">
              <summary className="cursor-pointer text-sm text-muted-foreground">{t("library.viewSource")}</summary>
              <pre className="mt-2 whitespace-pre-wrap font-mono text-sm">{openBody}</pre>
            </details>
          ) : null}
          {tab === "source" ? <p className="mt-3 text-sm">{open.description}</p> : null}
          {tab === "edit" ? <p className="mt-3 text-sm text-muted-foreground">{t("library.detail.history")}</p> : null}
          {tab === "local" ? (
            <div className="mt-3" data-testid="origin-meta">
              <p className="text-sm">{sourceLabel(open.source)}</p>
              <p className="sr-only">{t("library.origin.type")}: {open.source}</p>
              {open.sourceRef || sourceRef ? <p className="mt-1 truncate font-mono text-xs text-muted-foreground">{open.sourceRef || sourceRef}</p> : null}
            </div>
          ) : null}
          {tab === "diff" ? (
            <>
              <div data-testid="side-by-side" className="mt-2 grid gap-2 md:grid-cols-2">
                <pre className="whitespace-pre-wrap rounded-md border border-border p-2 font-mono text-sm">{openBody}</pre>
                <pre className="whitespace-pre-wrap rounded-md border border-border p-2 font-mono text-sm">{open.sourceBody}</pre>
              </div>
              <ul data-testid="diff-view" className="mt-2 font-mono text-sm">
                {lcsDiff(openBody, open.sourceBody).map((row, index) => (
                  <li key={`${row.kind}-${index}`} data-kind={row.kind}>
                    {row.kind === "add" ? "+" : row.kind === "del" ? "-" : " "} {row.text}
                  </li>
                ))}
              </ul>
              <ul data-testid="file-diff" className="mt-2 font-mono text-sm">
                {!(open.files && open.files.length) && !fileMap[open.id] ? <li data-testid="detail-loading" className="h-16 animate-pulse rounded-md bg-muted" /> : null}
                {fileDiffs((open.files && open.files.length ? open.files : fileMap[open.id]) ?? []).map((file) => (
                  <li key={file.path}>
                    <span>{file.path}</span>
                    {file.rows.map((row, index) => (
                      <span key={`${file.path}-${index}`} data-kind={row.kind}> {row.kind === "add" ? "+" : row.kind === "del" ? "-" : " "}{row.text}</span>
                    ))}
                  </li>
                ))}
              </ul>
            </>
          ) : null}
          <div className="sr-only">
            <button type="button" onClick={() => { void silent<{ id: string; name: string }[]>("get_projects").then((rows) => setPlacements(rows ?? [])); }}>{t("library.detail.projects")}</button>
            <ul>
              {placements.map((project) => (
                <li key={project.id}>
                  <button type="button" onClick={() => { if (open) void silent("export_skill_to_project", { id: project.id, skillId: open.id, agent: "claude_code" }); }}>{project.name}</button>
                </li>
              ))}
            </ul>
          </div>
        </aside>
      ) : null}
      {phase !== "ready" && initialId ? (
        <aside data-testid="detail-panel" className="rounded-lg border border-border bg-card p-3">
          {phase === "loading" ? <div data-testid="detail-loading" className="h-40 animate-pulse rounded-md bg-muted" /> : null}
          {phase === "error" || phase === "offline" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
          {phase === "empty" ? <EmptyState title={t("library.installedNone")} body={t("library.emptyBody")} /> : null}
        </aside>
      ) : null}
      </div>
    </Screen>
  );
}
