import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { fileDiffs, lcsDiff, filterSkills, moveById, type LibraryFilter, type SkillCard } from "@/features/library/model";
import { windowRows } from "@/lib/perf";
import { badgeFor } from "@/features/generate/model";
import { useGenerate } from "@/features/generate/store";
import { t } from "@/lib/i18n";
import { GitStatusButton } from "@/features/library/GitStatusButton";
import { UpdateFlow } from "@/features/library/UpdateFlow";
import { EmptyState, Screen, SearchField } from "@/components/chrome";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
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
  const [openId, setOpenId] = useState<string | null>(initialId ?? null);
  useEffect(() => {
    if (initialId) setOpenId(initialId);
  }, [initialId]);
  const [tab, setTab] = useState<"local" | "diff" | "source" | "edit">("local");
  const [rename, setRename] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [syncOpen, setSyncOpen] = useState(false);
  const [tagDraft, setTagDraft] = useState<Record<string, string>>({});
  const [menuId, setMenuId] = useState<string | null>(null);
  const [checked, setChecked] = useState<string | null>(null);
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

  return (
    <Screen testId="library" title={t("library.title")}>
      <div className="flex flex-wrap items-center gap-2">
        <GitStatusButton />
        <Button type="button" size="sm" variant="outline" disabled>{t("library.needsAttention")}</Button>
      </div>
      <UpdateFlow />
      <div className="flex flex-wrap items-center gap-2">
        <SearchField
          aria-label={t("library.search")}
          data-search
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <div role="group" aria-label={t("library.title")} className="flex gap-1">
          {(["all", "enabled", "available"] as const).map((item) => (
            <Button key={item} type="button" size="sm" variant={filter === item ? "default" : "outline"} aria-pressed={filter === item} onClick={() => setFilter(item)}>
              {t(`library.filter.${item}`)}
            </Button>
          ))}
        </div>
        <Button type="button" size="sm" variant="outline" onClick={() => setLayout(layout === "grid" ? "list" : "grid")}>
          {layout === "grid" ? t("library.view.list") : t("library.view.grid")}
        </Button>
      </div>
      <div className="mt-2 flex flex-wrap gap-2">
        {sources.map((item) => (
          <Button key={item} type="button" variant="outline" size="sm" onClick={() => setSource(source === item ? null : item)}>
            {item}
          </Button>
        ))}
        <Button type="button" variant="outline" size="sm" onClick={() => setTag(tag === "untagged" ? null : "untagged")}>
          Untagged
        </Button>
        {tags.map((item) => (
          <Button key={item} type="button" variant="outline" size="sm" onClick={() => setTag(tag === item ? null : item)} onContextMenu={(event) => { event.preventDefault(); setRename(item); setRenameValue(item); }}>
            {item}
          </Button>
        ))}
      </div>
      {rename ? (
        <form
          className="mt-2 flex flex-wrap gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            onRenameTag(rename, renameValue);
            setRename(null);
          }}
        >
          <Input aria-label={t("library.tag.rename")} value={renameValue} onChange={(event) => setRenameValue(event.target.value)} className="h-8" />
          <Button type="submit" size="sm">{t("library.tag.rename")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { onDeleteTag(rename); setRename(null); }}>
            {t("library.tag.delete")}
          </Button>
        </form>
      ) : null}
      <ol data-testid="preset-order" className="mt-3 flex flex-wrap gap-2">
        {memberIds.map((id) => (
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
            className="rounded-md border border-border px-2 py-1 text-xs"
          >
            {t("library.reorder")} {skills.find((skill) => skill.id === id)?.name ?? id}
          </li>
        ))}
      </ol>
      <p className="sr-only">{presetName}: {members.join(", ")}</p>
      {checked ? <p role="status">{t("library.checked")} {checked}</p> : null}
      {selected.length > 0 ? (
        <div className="mt-2 flex flex-wrap gap-2">
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
          className="mt-2 flex gap-2"
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
          className="mt-2 rounded-md border border-border p-3"
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
          <Button type="submit" variant="outline" size="sm">{t("library.sync")}</Button>
        </form>
      ) : null}
      {rendered.length === 0 ? <EmptyState title={t("library.empty")} /> : null}
      <ul data-testid="library-cards" data-layout={layout} className={layout === "grid" ? "grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3" : "space-y-2"}>
        {rendered.map((skill) => (
          <li key={skill.id}>
            <Card className="gap-2 rounded-lg border-border bg-card py-3 shadow-none">
            <div className="px-3">
            <label className="flex items-start gap-2">
              <input type="checkbox" aria-label={`Select ${skill.name}`} checked={selected.includes(skill.id)} onChange={() => setSelected((current) => current.includes(skill.id) ? current.filter((id) => id !== skill.id) : [...current, skill.id], ) } />
              <button type="button" className="text-left" onClick={() => { setOpenId(skill.id); setTab("local"); }}>
                <span className="block font-medium whitespace-normal">{skill.name}</span>
                {skill.draft ? <Badge data-testid="draft-badge" variant="warning">{t("library.badge.draft")}</Badge> : null}
                {badgeFor(seed, skill.id) === "tested" ? <Badge data-testid="tested-badge" variant="success">{t("library.badge.tested")}</Badge> : null}
                {badgeFor(seed, skill.id) === "regressed" ? <Badge data-testid="regressed-badge" variant="warning">{t("library.badge.regressed")}</Badge> : null}
                <span className="block text-sm text-muted-foreground">{skill.description}</span>
              </button>
            </label>
            <label className="mt-2 flex items-center gap-2 text-xs">
              <input type="checkbox" aria-label={`${t("library.membership")} ${skill.name}`} checked={memberIds.includes(skill.id)} onChange={() => onToggleMember(skill.id)} />
              {t("library.membership")}
            </label>
            <div className="mt-2 flex flex-wrap gap-1">
              {skill.tags.map((item) => (
                <button key={item} type="button" className="text-xs" onClick={() => onRemoveTag(skill.id, item)}>
                  {t("library.tag.remove")} {item}
                </button>
              ))}
            </div>
            <div className="mt-2 flex gap-1" data-testid={`sync-dots-${skill.id}`}>
              {skill.agents.map((agent) => (
                <span key={agent} data-filled="true" className="inline-block h-2 w-2 rounded-full bg-primary" title={agent} />
              ))}
            </div>
            <button type="button" className="mt-2 text-xs" aria-label={t("library.menu")} onClick={() => setMenuId(menuId === skill.id ? null : skill.id)}>
              {t("library.menu")}
            </button>
            {menuId === skill.id ? (
              <div className="mt-1 flex gap-2">
                <Link to={`/skills/${skill.id}/edit`} className="text-xs">{t("library.menu.edit")}</Link>
                <Link to={`/skills/${skill.id}/tests`} className="text-xs" aria-label={t("library.runTests")}>{t("library.menu.tests")}</Link>
                <button type="button" className="text-xs" onClick={() => setChecked(skill.name)}>{t("library.menu.check")}</button>
                <button type="button" className="text-xs" onClick={() => onDeleteSkills([skill.id])}>{t("library.menu.delete")}</button>
              </div>
            ) : null}
            <form
              className="mt-2 flex gap-1"
              onSubmit={(event) => {
                event.preventDefault();
                const value = tagDraft[skill.id]?.trim();
                if (value) onAddTag(skill.id, value);
                setTagDraft((current) => ({ ...current, [skill.id]: "" }));
              }}
            >
              <Input aria-label={`${t("library.tag.add")} ${skill.name}`} value={tagDraft[skill.id] ?? ""} onChange={(event) => setTagDraft((current) => ({ ...current, [skill.id]: event.target.value }))} className="h-8 w-24" />
              <button type="submit" className="text-xs">{t("library.tag.add")}</button>
            </form>
            </div>
            </Card>
          </li>
        ))}
      </ul>
      {open ? (
        <aside data-testid="detail-panel" className="mt-4 rounded-lg border border-border p-3">
          <div className="flex flex-wrap gap-2">
            {(["local", "diff", "source", "edit"] as const).map((item) => (
              <Button key={item} type="button" aria-pressed={tab === item} variant="outline" size="sm" onClick={() => setTab(item)}>
                {t(`library.detail.${item}`)}
              </Button>
            ))}
          </div>
          <div className="mt-2 flex flex-wrap gap-2">
            {["claude_code", "codex"].map((agent) => (
              <Button key={agent} type="button" aria-pressed={open.agents.includes(agent)} variant="outline" size="sm" onClick={() => onToggleAgent(open.id, agent)}>
                {agent}
              </Button>
            ))}
          </div>
          {tab === "edit" ? <Link to={`/skills/${open.id}/edit`}>{t("library.detail.edit")}</Link> : null}
          {tab === "local" ? <pre className="mt-2 whitespace-pre-wrap text-sm">{open.body}</pre> : null}
          {tab === "source" ? <pre className="mt-2 whitespace-pre-wrap text-sm">{open.sourceBody}</pre> : null}
          {tab === "diff" ? (
            <>
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
          <p className="mt-3 text-sm">{t("library.detail.projects")}</p>
        </aside>
      ) : null}
    </Screen>
  );
}
