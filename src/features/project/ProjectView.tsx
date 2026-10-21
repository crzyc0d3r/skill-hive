import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useContext, useEffect, useRef, useState } from "react";
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
import { StatePanel } from "@/components/chrome";
import { ProjectSkeleton } from "@/components/skeletons";
import { RetryContext, useShellPhase } from "@/lib/view-state";

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
  const phase = useShellPhase();
  const retry = useContext(RetryContext);
  const { id = "fixture" } = useParams();
  const [linked, setLinked] = useState(false);
  const [rows, setRows] = useState<ProjectSkill[]>([]);
  const [tab, setTab] = useState<"local" | "diff" | "center">("local");
  const [library, setLibrary] = useState<SkillCard[]>([]);
  const [adding, setAdding] = useState(false);
  const [agents, setAgents] = useState<string[]>(["claude_code"]);
  const [root, setRoot] = useState("");
  const [pickerQuery, setPickerQuery] = useState("");
  const [picked, setPicked] = useState<string[]>([]);
  const [anchor, setAnchor] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<ProjectSkill | null>(null);
  const pinned = useRef(false);
  const skill = rows[0] ?? FIXTURE;

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

  useEffect(() => {
    if (rows.length) setProjectSkills(rows);
  }, [rows]);

  const update = (next: ProjectSkill) => {
    setRows((current) => (current.length ? current.map((item) => (item.id === next.id ? next : item)) : [next]));
  };

  function rememberAgents(next: string[]) {
    setAgents(next);
    void silent("set_settings", { key: "project_last_agents", value: next.join(",") });
  }

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
    <section data-testid="project" className="flex w-full flex-col gap-4">
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("project.title")}</h1>
      {phase === "loading" ? <ProjectSkeleton /> : null}
      {phase === "error" ? <StatePanel tone="error" title={t("shell.error")} action={<Button type="button" size="sm" onClick={retry}>{t("banner.retry")}</Button>} /> : null}
      <Card className="max-w-xl gap-3 rounded-lg border-border px-4 py-4 shadow-none">
        <p className="text-sm text-muted-foreground">{t("project.linked")}</p>
        <label className="block text-sm">
          {t("project.skillsRoot")}
          <Input aria-label={t("project.skillsRoot")} value={root} onChange={(event) => setRoot(event.target.value)} className="mt-1 h-8" />
        </label>
        <div className="flex flex-wrap gap-2">
          <Button type="button" data-testid="link-project" size="sm" onClick={() => {
              pinned.current = true;
              const next = root.trim() && rows.length ? rows : [FIXTURE];
              setRows(next);
              setLinked(true);
              setProjectSkills(next);
              const chosen = next[0] ?? FIXTURE;
              void addProject("fixture", root);
              void silent("add_linked_workspace", { name: chosen.name, path: root || chosen.name, agent: agents[0] ?? "claude_code" });
            }}
          >
            {t("project.link")}
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { setLinked(true); void addProject(skill.name, root); }}>
            {t("project.linked")}
          </Button>
        </div>
      </Card>
      {phase === "ready" && linked ? (
        <Card className="max-w-xl gap-3 rounded-lg border-border px-4 py-4 text-sm shadow-none">
          {(rows.length ? rows : [skill]).map((item) => (
            <div key={item.id} className="space-y-2">
              <p>
                {item.name}
                <span data-testid="project-dot" data-color={dotColor(item)} className="ml-2 inline-block h-2 w-2 rounded-full bg-current" />
              </p>
              <p>{t("project.agents")}: {(item.agents ?? []).join(", ") || "—"}</p>
              <Button type="button" variant="outline" size="sm" onClick={() => { update(addSkillToAgents(item, ["claude_code", "codex"])); void syncSkillToTool(item.id, "claude_code"); void syncSkillToTool(item.id, "codex"); void silent("export_skill_to_project", { id, skillId: item.id, agent: "claude_code" }); void silent("export_skill_to_project", { id, skillId: item.id, agent: "codex" }); }}>
                {t("project.add")}
              </Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { const enabled = !item.enabled; update(toggleSkill(item, enabled)); void setSkillsEnabled([item.id], enabled); void silent("toggle_project_skill", { id, name: item.name, enabled }); }}>
                {t("project.disable")}
              </Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { update(pullCenter(item)); void pullProjectCenter(item.name, item.center); void silent("update_project_skill_from_center", { id, skillId: item.id, agent: "claude_code" }); }}>
                {t("project.pull")}
              </Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { void silent("update_project_skill_from_center", { id, skillId: item.id, agent: agents[0] ?? "claude_code" }); }}>
                {t("project.update")}
              </Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { void silent("update_project_skill_to_center", { id, name: item.name, skillId: item.id }); }}>
                {t("project.updateCenter")}
              </Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => setPendingDelete(item)}>
                {t("workspace.delete")}
              </Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { update({ ...item, enabled: true }); void setSkillsEnabled([item.id], true); void silent("toggle_project_skill", { id, name: item.name, enabled: true }); }}>
                {t("project.batch")}
              </Button>
              <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { void silent("reveal_project_folder", { id }); }}>
                {t("library.detail.local")}
              </Button>
            </div>
          ))}
          <div className="flex gap-2">
            <Button type="button" size="sm" variant={tab === "local" ? "secondary" : "outline"} onClick={() => setTab("local")}>{t("library.detail.local")}</Button>
            <Button type="button" size="sm" variant={tab === "diff" ? "secondary" : "outline"} onClick={() => setTab("diff")}>{t("library.detail.diff")}</Button>
            <Button type="button" size="sm" variant={tab === "center" ? "secondary" : "outline"} onClick={() => setTab("center")}>{t("workspace.detail.center")}</Button>
          </div>
          {tab === "local" ? <pre className="whitespace-pre-wrap font-mono text-sm">{skill.local}</pre> : null}
          {tab === "center" ? <pre className="whitespace-pre-wrap font-mono text-sm">{skill.center}</pre> : null}
          {tab === "diff" ? <pre className="whitespace-pre-wrap font-mono text-sm">{skill.local}{"\n"}{skill.center}</pre> : null}
          <div className="flex flex-wrap gap-2">
            {["claude_code", "codex"].map((agent) => (
              <Button key={agent} type="button" size="sm" aria-pressed={agents.includes(agent)} variant={agents.includes(agent) ? "secondary" : "outline"} onClick={() => rememberAgents(agents.includes(agent) ? agents.filter((item) => item !== agent) : [...agents, agent])}>{agent}</Button>
            ))}
          </div>
          <Button type="button" size="sm" variant="outline" onClick={() => { setAdding(true); void silent<SkillCard[]>("get_managed_skills").then((loaded) => setLibrary(loaded ?? [])); }}>{t("project.fromLibrary")}</Button>
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
        </Card>
      ) : null}
    </section>
  );
}
