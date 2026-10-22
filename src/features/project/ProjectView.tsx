import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useContext, useEffect, useRef, useState } from "react";
import { MoreHorizontalIcon } from "lucide-react";
import { useParams } from "react-router-dom";
import { useDebounced } from "@/lib/use-debounced";
import { t } from "@/lib/i18n";
import {
  addSkillToAgents,
  dotColor,
  pullCenter,
  toggleSkill,
  type ProjectSkill,
} from "@/features/project/model";
import { setProjectSkills } from "@/features/project/store";
import { addProject, pullProjectCenter, setSkillsEnabled, silent, syncSkillToTool } from "@/lib/ipc/actions";
import type { SkillCard } from "@/features/library/model";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { AgentBadge } from "@/components/app/AgentBadge";
import { agentDisplayName } from "@/lib/agents";
import { EmptyState, ErrorState } from "@/components/chrome";
import { ProjectSkeleton } from "@/components/skeletons";
import { demoEnabled } from "@/lib/demo";
import { RetryContext, useViewPhase } from "@/lib/view-state";

const DEMO_ROWS: ProjectSkill[] = [
  { id: "research-notes", name: "research-notes", agents: ["claude_code"], enabled: true, center: "v2", local: "v2" },
  { id: "billing", name: "billing", agents: ["codex"], enabled: true, center: "", local: "local copy" },
  { id: "review-diff", name: "review-diff", agents: [], enabled: true, center: "library", local: "" },
];

const FIXTURE: ProjectSkill = {
  id: "notes",
  name: "notes",
  agents: [],
  enabled: true,
  center: "center v2",
  local: "center v1",
};

interface ProjectSkillRow {
  name: string;
  path: string;
  enabled: boolean;
  agent: string;
}

function fromProjectRow(row: ProjectSkillRow): ProjectSkill {
  return {
    id: row.name,
    name: row.name,
    agents: row.agent ? [row.agent] : [],
    enabled: row.enabled,
    center: row.path,
    local: row.path,
  };
}

function statusLabel(item: ProjectSkill): string {
  if (item.local && item.center && item.local === item.center) return t("project.same");
  if (item.local && item.center) return t("project.differs");
  if (item.local) return t("project.onlyHere");
  return t("project.onlyLibrary");
}

function fromManaged(skill: SkillCard): ProjectSkill {
  return {
    id: skill.id,
    name: skill.name,
    agents: Array.isArray(skill.agents) ? skill.agents : [],
    enabled: skill.enabled,
    center: skill.sourceBody,
    local: skill.body,
  };
}

export function ProjectView() {
  const phase = useViewPhase();
  const retry = useContext(RetryContext);
  const { id = "fixture" } = useParams();
  const [linked, setLinked] = useState(() => demoEnabled());
  const [rows, setRows] = useState<ProjectSkill[]>(() => (demoEnabled() ? DEMO_ROWS : []));
  const [library, setLibrary] = useState<SkillCard[]>([]);
  const [chosen, setChosen] = useState<string[]>([]);
  const [adding, setAdding] = useState(false);
  const [agents, setAgents] = useState<string[]>(["claude_code"]);
  const [root, setRoot] = useState("");
  const [pickerQuery, setPickerQuery] = useState("");
  const [picked, setPicked] = useState<string[]>([]);
  const [anchor, setAnchor] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<ProjectSkill | null>(null);
  const [menuId, setMenuId] = useState<string | null>(null);
  const pinned = useRef(false);
  const previewOnly = useRef(demoEnabled());
  const skill = rows[0] ?? FIXTURE;

  useEffect(() => {
    if (previewOnly.current || rows.length === 0) return;
    setProjectSkills(rows);
  }, [rows]);

  useEffect(() => {
    let current = true;
    pinned.current = false;
    void silent<string | null>("get_settings", { key: "project_last_agents" }).then((value) => {
      if (!current || !value) return;
      const next = value.split(",").map((item) => item.trim()).filter(Boolean);
      if (next.length) setAgents(next);
    });
    void silent<ProjectSkillRow[]>("get_project_skills", { id }).then((projectRows) => {
      if (!current || pinned.current) return;
      if (Array.isArray(projectRows) && projectRows.length) {
        const next = projectRows.map(fromProjectRow);
        setRows(next);
        setProjectSkills(next);
        setLinked(true);
        return;
      }
      void silent<SkillCard[]>("get_managed_skills").then((skills) => {
        if (!current || pinned.current || !Array.isArray(skills) || skills.length === 0) return;
        const next = skills.map(fromManaged);
        setRows(next);
        setProjectSkills(next);
        setLinked(true);
      });
    });
    return () => {
      current = false;
    };
  }, [id]);

  const update = (next: ProjectSkill) => {
    previewOnly.current = false;
    setRows((current) => (current.length ? current.map((item) => (item.id === next.id ? next : item)) : [next]));
  };

  const debouncedPicker = useDebounced(pickerQuery, 150);
  function togglePick(skillId: string, shift: boolean) {
    const visible = library.filter((item) => item.name.toLowerCase().includes(debouncedPicker.trim().toLowerCase()));
    if (shift && anchor) {
      const start = visible.findIndex((item) => item.id === anchor);
      const end = visible.findIndex((item) => item.id === skillId);
      if (start >= 0 && end >= 0) {
        const [from, to] = start < end ? [start, end] : [end, start];
        const range = visible.slice(from, to + 1).map((item) => item.id);
        setPicked((current) => [...new Set([...current, ...range])]);
        return;
      }
    }
    setAnchor(skillId);
    setPicked((current) => current.includes(skillId) ? current.filter((item) => item !== skillId) : [...current, skillId]);
  }

  return (
    <section data-testid="project" className="flex w-full min-w-0 flex-col gap-4">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="font-sans text-display font-semibold tracking-tight">{t("project.title")}</h1>
          {phase === "ready" && linked ? <p className="mt-1 font-mono text-xs text-muted-foreground">{root.trim() || "~/src/notes"}</p> : null}
        </div>
        {phase === "ready" ? (
          <>
          <label className="sr-only">
            {t("project.skillsRoot")}
            <Input aria-label={t("project.skillsRoot")} value={root} onChange={(event) => setRoot(event.target.value)} />
          </label>
          <Button type="button" data-testid="link-project" size="sm" onClick={() => {
            pinned.current = true;
            previewOnly.current = false;
            const next = root.trim() && rows.length ? rows : [FIXTURE];
            setRows(next);
            setLinked(true);
            setProjectSkills(next);
            const linkedSkill = next[0] ?? FIXTURE;
            void addProject("fixture", root);
            void silent("add_linked_workspace", { name: linkedSkill.name, path: root || linkedSkill.name, agent: agents[0] ?? "claude_code" });
          }}>
            {t("project.link")}
          </Button>
          </>
        ) : null}
      </header>
      {phase === "loading" ? <ProjectSkeleton /> : null}
      {phase === "error" ? <ErrorState title={t("shell.error")} detail={t("shell.renderError")} onRetry={retry} /> : null}
      {phase === "empty" ? <EmptyState title={t("project.title")} body={t("project.linked")} action={<Button type="button" size="sm" onClick={() => { setLinked(true); void addProject(skill.name, root); }}>{t("project.link")}</Button>} /> : null}
      {phase === "ready" && rows.length > 0 ? (
        <div className="overflow-hidden rounded-lg border border-border">
          <div className="flex flex-wrap items-center justify-between gap-2 px-3 py-2">
            <p className="text-sm font-medium">{t("project.compareTitle")}</p>
            <div className="flex flex-wrap items-center gap-2">
              <label className="flex items-center gap-2 text-sm">
                <span className="text-muted-foreground">{t("project.agents")}</span>
                <select
                  aria-label={t("project.agents")}
                  className="h-8 rounded-md border border-input bg-background px-2"
                  value={agents.includes("codex") && !agents.includes("claude_code") ? "codex" : agents[0] ?? "claude_code"}
                  onChange={(event) => {
                    const next = [event.target.value];
                    setAgents(next);
                    void silent("set_settings", { key: "project_last_agents", value: next.join(",") });
                  }}
                >
                  <option value="claude_code">{agentDisplayName("claude_code")}</option>
                  <option value="codex">{agentDisplayName("codex")}</option>
                </select>
              </label>
              <Button type="button" size="sm" variant="outline" className="w-fit" onClick={() => { setAdding(true); void silent<SkillCard[]>("get_managed_skills").then((loaded) => setLibrary(loaded ?? [])); }}>{t("project.fromLibrary")}</Button>
            </div>
          </div>
          {chosen.length > 0 ? (
            <div className="flex items-center justify-between gap-2 border-t border-border bg-muted/40 px-3 py-2">
              <span className="text-sm">{chosen.length}</span>
              <Button type="button" size="sm" onClick={() => { for (const item of rows) { if (chosen.includes(item.id)) update(pullCenter(item)); } setChosen([]); }}>{t("project.copySelected")}</Button>
            </div>
          ) : null}
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-muted-foreground">
              <tr>
                <th className="w-10 px-3 py-2" />
                <th className="px-3 py-2">{t("library.title")}</th>
                <th className="px-3 py-2">{t("project.col.status")}</th>
                <th className="hidden px-3 py-2 min-[1440px]:table-cell">{t("project.agents")}</th>
                <th className="w-12 px-3 py-2" />
              </tr>
            </thead>
            <tbody>
              {rows.map((item) => (
                <tr key={item.id} className="border-t border-border">
                  <td className="px-3 py-2">
                    <input type="checkbox" className="size-4 rounded border-input" aria-label={`${t("project.selectAll")} ${item.name}`} checked={chosen.includes(item.id)} onChange={() => setChosen((current) => current.includes(item.id) ? current.filter((id) => id !== item.id) : [...current, item.id])} />
                  </td>
                  <td className="px-3 py-2 font-medium">
                    {item.name}
                    <span className="sr-only">{(item.agents ?? []).map((agent) => agentDisplayName(agent)).join(", ")}</span>
                    <span data-testid={item.name === "notes" ? "project-dot" : undefined} data-color={dotColor(item)} className="sr-only" />
                  </td>
                  <td className="px-3 py-2">{statusLabel(item)}</td>
                  <td className="hidden px-3 py-2 min-[1440px]:table-cell">
                    {(item.agents ?? []).length > 0 ? (
                      <span className="inline-flex flex-wrap gap-1">{item.agents.map((agent) => <AgentBadge key={agent} agent={agent} />)}</span>
                    ) : null}
                  </td>
                  <td className="relative px-3 py-2 text-right">
                    <Button type="button" variant="ghost" size="icon-sm" aria-label={`${t("shell.more")} ${item.name}`} aria-expanded={menuId === item.id} onClick={() => setMenuId(menuId === item.id ? null : item.id)}>
                      <MoreHorizontalIcon aria-hidden="true" />
                    </Button>
                    {menuId === item.id ? (
                      <div role="menu" className="absolute right-3 z-20 mt-1 w-44 rounded-md border border-border bg-popover p-1 text-left shadow-md">
                        <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); update(addSkillToAgents(item, ["claude_code", "codex"])); void syncSkillToTool(item.id, "claude_code"); void syncSkillToTool(item.id, "codex"); void silent("export_skill_to_project", { id, skillId: item.id, agent: "claude_code" }); void silent("export_skill_to_project", { id, skillId: item.id, agent: "codex" }); }}>{t("project.add")}</button>
                        <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); const enabled = !item.enabled; update(toggleSkill(item, enabled)); void setSkillsEnabled([item.id], enabled); void silent("toggle_project_skill", { id, name: item.name, enabled }); }}>{t("project.disable")}</button>
                        <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); update(pullCenter(item)); void pullProjectCenter(item.name, item.center); void silent("update_project_skill_from_center", { id, skillId: item.id, agent: "claude_code" }); }}>{t("project.pull")}</button>
                        <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); void silent("update_project_skill_from_center", { id, skillId: item.id, agent: agents[0] ?? "claude_code" }); }}>{t("project.update")}</button>
                        <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); void silent("update_project_skill_to_center", { id, name: item.name, skillId: item.id }); }}>{t("project.updateCenter")}</button>
                        <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); update({ ...item, enabled: true }); void setSkillsEnabled([item.id], true); void silent("toggle_project_skill", { id, name: item.name, enabled: true }); }}>{t("project.batch")}</button>
                        <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent" onClick={() => { setMenuId(null); void silent("reveal_project_folder", { id }); }}>{t("library.detail.local")}</button>
                        <button type="button" role="menuitem" className="block w-full rounded-sm px-2 py-1.5 text-left text-sm text-destructive hover:bg-accent" onClick={() => { setMenuId(null); setPendingDelete(item); }}>{t("workspace.delete")}</button>
                      </div>
                    ) : null}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {adding ? (
            <div data-testid="add-from-library">
              <Input aria-label={t("workspace.picker.search")} value={pickerQuery} onChange={(event) => setPickerQuery(event.target.value)} className="mb-2 h-8" />
              <Button type="button" size="sm" variant="outline" onClick={() => setPicked(library.map((row) => row.id))}>{t("project.selectAll")}</Button>
              <ul>
                {library.filter((row) => row.name.toLowerCase().includes(debouncedPicker.trim().toLowerCase())).map((row) => (
                  <li key={row.id}>
                    <label className="text-sm">
                      <input type="checkbox" aria-label={`${t("project.selectAll")} ${row.name}`} checked={picked.includes(row.id)} onClick={(event) => togglePick(row.id, event.shiftKey)} onChange={() => undefined} /> {row.name}
                    </label>
                  </li>
                ))}
              </ul>
              <Button type="button" size="sm" onClick={() => { for (const skillId of picked) { for (const agent of agents) void silent("export_skill_to_project", { id, skillId, agent }); } setAdding(false); }}>{t("project.fromLibrary")}</Button>
            </div>
          ) : null}
          {pendingDelete ? (
            <ConfirmDialog
              label={t("workspace.delete")}
              detail={pendingDelete.name}
              confirmLabel={t("workspace.confirm")}
              onConfirm={() => {
                void silent("delete_project_skill", { id, name: pendingDelete.name });
                setRows((current) => current.filter((item) => item.id !== pendingDelete.id));
                setPendingDelete(null);
              }}
            />
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
