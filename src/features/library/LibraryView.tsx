import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { CheckIcon, GripVerticalIcon, LayoutGridIcon, ListIcon, MoreHorizontalIcon, PlusIcon, XIcon } from "lucide-react";
import { fileDiffs, lcsDiff, filterSkills, moveById, type LibraryFilter, type SkillCard } from "@/features/library/model";
import { windowRows } from "@/lib/perf";
import { badgeFor } from "@/features/generate/model";
import { useGenerate } from "@/features/generate/store";
import { checkSkillUpdate, silent } from "@/lib/ipc/actions";
import { t } from "@/lib/i18n";
import { cn } from "cn";
import { GitStatusButton } from "@/features/library/GitStatusButton";
import { UpdateFlow } from "@/features/library/UpdateFlow";
import { EmptyState, Screen, SearchField } from "@/components/chrome";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";

const menuItem = "flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-sm outline-none hover:bg-accent hover:text-accent-foreground";

function agentLabel(agent: string) {
  if (agent === "claude_code") return "Claude";
  if (agent === "codex") return "Codex";
  return agent;
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
  const { seed } = useGenerate();

  const sources = useMemo(() => [...new Set(skills.map((skill) => skill.source))], [skills]);
  const tags = useMemo(() => [...new Set(skills.flatMap((skill) => skill.tags))], [skills]);
  const visible = filterSkills(skills, query, filter, source, tag);
  const open = skills.find((skill) => skill.id === openId) ?? null;
  const rendered = windowRows(visible, 100);
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
    <Screen testId="library" title={t("library.title")}>
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
        <div className="ml-auto flex items-center gap-1">
          <GitStatusButton />
          <UpdateFlow skills={skills} />
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
      <div className="flex flex-wrap items-center gap-1.5">
        {sources.map((item) => (
          <Button key={item} type="button" variant={source === item ? "default" : "secondary"} size="xs" className="rounded-full shadow-none" onClick={() => setSource(source === item ? null : item)}>
            {item}
          </Button>
        ))}
        <Button type="button" variant={tag === "untagged" ? "default" : "secondary"} size="xs" className="rounded-full shadow-none" onClick={() => setTag(tag === "untagged" ? null : "untagged")}>
          Untagged
        </Button>
        {tags.map((item) => (
          <Button
            key={item}
            type="button"
            variant={tag === item ? "default" : "secondary"}
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
      <ol data-testid="preset-order" className="flex flex-wrap items-center gap-1.5">
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
              className="inline-flex items-center gap-1 rounded-full border border-border bg-card px-2 py-0.5 text-xs"
            >
              <button type="button" className="text-muted-foreground" title={t("library.reorder")} aria-label={t("library.reorder")}>
                <GripVerticalIcon className="size-3" aria-hidden="true" />
              </button>
              <span>{name}{"\u200b"}</span>
            </li>
          );
        })}
      </ol>
      <p className="sr-only">{presetName}: {members.join(", ")}</p>
      {checked ? (
        <p role="status">
          <span>{t("library.checked")} {checked.name}</span>
          {checked.status ? <span> {checked.status}</span> : null}
        </p>
      ) : null}
      {selected.length > 0 ? (
        <div className="sticky top-0 z-20 flex flex-wrap items-center gap-2 rounded-lg border border-border bg-card px-3 py-2 shadow-sm">
          <span className="font-mono text-xs text-muted-foreground">{selected.length}</span>
          <Button type="button" variant="outline" size="sm" onClick={() => onSetEnabled(selected, true)}>{t("library.batch.enable")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => onSetEnabled(selected, false)}>{t("library.batch.disable")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => setBatchOpen(true)}>{t("library.batch.tags")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { onDeleteSkills(selected); setSelected([]); }}>{t("library.batch.delete")}</Button>
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
            const agents = ["claude_code", "codex"].filter((agent) => data.get(agent) === "on");
            onSync(selected, agents);
            setSyncOpen(false);
          }}
        >
          <label className="mr-3 text-sm"><input type="checkbox" name="claude_code" /> Claude Code</label>
          <label className="mr-3 text-sm"><input type="checkbox" name="codex" /> Codex</label>
          <Button type="submit" size="sm">{t("library.sync")}</Button>
        </form>
      ) : null}
      {rendered.length === 0 ? <EmptyState title={t("library.empty")} /> : null}
      <ul data-testid="library-cards" data-layout={layout} className={layout === "grid" ? "grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3" : "space-y-2"}>
        {rendered.map((skill) => (
          <li key={skill.id}>
            <Card className={cn("group/card gap-0 overflow-hidden rounded-lg border-border bg-card py-0 shadow-none transition-colors hover:border-foreground/15", layout === "list" && "flex-row")}>
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
                    <button type="button" className={menuItem} onClick={() => { void silent("reimport_local_skill", { skillId: skill.id }); }}>{t("library.menu.update")}</button>
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
              <div className="mt-3 flex items-center justify-between gap-2 border-t border-border px-3 py-2">
                <div className="flex min-w-0 items-center gap-2">
                  <Badge variant="outline" className="font-normal">{skill.source}</Badge>
                  <span className="relative inline-flex size-4 items-center justify-center" title={t("library.membership")}>
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
                  {memberIds.includes(skill.id) ? <span className="truncate text-xs text-muted-foreground">{presetName}</span> : null}
                </div>
                <div className="flex gap-1" data-testid={`sync-dots-${skill.id}`}>
                  {["claude_code", "codex"].map((agent) => {
                    const filled = skill.agents.includes(agent);
                    return (
                      <button key={agent} type="button" data-filled={filled ? "true" : "false"} title={agent} className="inline-flex items-center gap-1 rounded-full bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground" onClick={() => onToggleAgent(skill.id, agent)}>
                        <span className={cn("size-1.5 rounded-full", filled ? "bg-primary" : "bg-muted-foreground/40")} aria-hidden="true" />
                        {agentLabel(agent)}
                      </button>
                    );
                  })}
                </div>
              </div>
            </Card>
          </li>
        ))}
      </ul>
      {open ? (
        <aside data-testid="detail-panel" className="rounded-lg border border-border bg-card p-3">
          <div className="inline-flex rounded-lg bg-muted p-0.5">
            {(["local", "diff", "source", "edit"] as const).map((item) => (
              <Button key={item} type="button" aria-pressed={tab === item} variant="ghost" size="sm" className={cn("h-7", tab === item && "bg-background shadow-sm")} onClick={() => setTab(item)}>
                {t(`library.detail.${item}`)}
              </Button>
            ))}
          </div>
          <div className="mt-2 flex flex-wrap gap-2">
            {["claude_code", "codex"].map((agent) => (
              <Button key={agent} type="button" aria-pressed={open.agents.includes(agent)} variant={open.agents.includes(agent) ? "secondary" : "outline"} size="sm" onClick={() => onToggleAgent(open.id, agent)}>
                {agent}
              </Button>
            ))}
          </div>
          {tab === "edit" ? <Link to={`/skills/${open.id}/edit`}>{t("library.detail.edit")}</Link> : null}
          {tab === "local" ? <pre className="mt-2 whitespace-pre-wrap font-mono text-sm">{open.body}</pre> : null}
          {tab === "source" ? <pre className="mt-2 whitespace-pre-wrap font-mono text-sm">{open.sourceBody}</pre> : null}
          {tab === "local" ? (
            <div className="mt-3">
              <p className="text-xs text-muted-foreground">{open.source} · {open.updateStatus ?? t("library.checked")}</p>
              <ul className="mt-2 text-sm">
                <li>{t("library.detail.source")}: {open.source}</li>
              </ul>
            </div>
          ) : null}
          {tab === "diff" ? (
            <>
              <div data-testid="side-by-side" className="mt-2 grid gap-2 md:grid-cols-2">
                <pre className="whitespace-pre-wrap rounded-md border border-border p-2 font-mono text-sm">{open.body}</pre>
                <pre className="whitespace-pre-wrap rounded-md border border-border p-2 font-mono text-sm">{open.sourceBody}</pre>
              </div>
              <ul data-testid="diff-view" className="mt-2 font-mono text-sm">
                {lcsDiff(open.body, open.sourceBody).map((row, index) => (
                  <li key={`${row.kind}-${index}`} data-kind={row.kind}>
                    {row.kind === "add" ? "+" : row.kind === "del" ? "-" : " "} {row.text}
                  </li>
                ))}
              </ul>
              <ul data-testid="file-diff" className="mt-2 font-mono text-sm">
                {fileDiffs(open.files ?? []).map((file) => (
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
          <div className="mt-3 text-sm">
            <button type="button" className="text-muted-foreground" onClick={() => { void silent<{ id: string; name: string }[]>("get_projects").then((rows) => setPlacements(rows ?? [])); }}>{t("library.detail.projects")}</button>
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
    </Screen>
  );
}
