import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { useState } from "react";
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

const FIXTURE: ProjectSkill = {
  id: "notes",
  name: "notes",
  agents: [],
  enabled: true,
  center: "center v2",
  local: "center v1",
};

export function ProjectView() {
  const [linked, setLinked] = useState(false);
  const [skill, setSkill] = useState<ProjectSkill>(FIXTURE);
  const [tab, setTab] = useState<"local" | "diff" | "center">("local");
  const [library, setLibrary] = useState<SkillCard[]>([]);
  const [adding, setAdding] = useState(false);
  const [agents, setAgents] = useState<string[]>(["claude_code"]);
  const update = (next: ProjectSkill) => {
    setSkill(next);
    setProjectSkills([next]);
  };
  return (
    <section data-testid="project" className="flex w-full flex-col gap-4">
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("project.title")}</h1>
      <Card className="max-w-xl gap-3 rounded-lg border-border px-4 py-4 shadow-none">
        <p className="text-sm text-muted-foreground">{t("project.linked")}</p>
        <div className="flex flex-wrap gap-2">
          <Button type="button" data-testid="link-project" size="sm" onClick={() => {
              setLinked(true);
              setProjectSkills([skill]);
              void addProject("fixture", "");
              void silent("add_linked_workspace", { name: skill.name, path: skill.name, agent: "claude_code" });
            }}
          >
            {t("project.link")}
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={() => { setLinked(true); void addProject(skill.name, ""); }}>
            {t("project.linked")}
          </Button>
        </div>
      </Card>
      {linked ? (
        <Card className="max-w-xl gap-3 rounded-lg border-border px-4 py-4 text-sm shadow-none">
          <p>
            {skill.name}
            <span data-testid="project-dot" data-color={dotColor(skill)} className="ml-2 inline-block h-2 w-2 rounded-full bg-current" />
          </p>
          <p>{t("project.agents")}: {skill.agents.join(", ") || "—"}</p>
          <Button type="button" variant="outline" size="sm" onClick={() => { update(addSkillToAgents(skill, ["claude_code", "codex"])); void syncSkillToTool(skill.id, "claude_code"); void syncSkillToTool(skill.id, "codex"); void silent("export_skill_to_project", { id: "fixture", skillId: skill.id, agent: "claude_code" }); void silent("export_skill_to_project", { id: "fixture", skillId: skill.id, agent: "codex" }); }}>
            {t("project.add")}
          </Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { const enabled = !skill.enabled; update(toggleSkill(skill, enabled)); void setSkillsEnabled([skill.id], enabled); void silent("toggle_project_skill", { id: "fixture", name: skill.name, enabled }); }}>
            {t("project.disable")}
          </Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { update(pullCenter(skill)); void pullProjectCenter(skill.name, skill.center); void silent("update_project_skill_from_center", { id: "fixture", skillId: skill.id, agent: "claude_code" }); void silent("update_project_skill_to_center", { id: "fixture", name: skill.name, skillId: skill.id }); }}>
            {t("project.pull")}
          </Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { update({ ...skill, enabled: true }); void setSkillsEnabled([skill.id], true); void silent("toggle_project_skill", { id: "fixture", name: skill.name, enabled: true }); }}>
            {t("project.batch")}
          </Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { void silent("reveal_project_folder", { id: "fixture" }); }}>
            {t("library.detail.local")}
          </Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { void silent("delete_project_skill", { id: "fixture", name: skill.name }); }}>
            {t("workspace.delete")}
          </Button>
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
              <Button key={agent} type="button" size="sm" aria-pressed={agents.includes(agent)} variant={agents.includes(agent) ? "secondary" : "outline"} onClick={() => setAgents((current) => current.includes(agent) ? current.filter((item) => item !== agent) : [...current, agent])}>{agent}</Button>
            ))}
          </div>
          <Button type="button" size="sm" variant="outline" onClick={() => { setAdding(true); void silent<SkillCard[]>("get_managed_skills").then((rows) => setLibrary(rows ?? [])); }}>{t("project.fromLibrary")}</Button>
          {adding ? (
            <ul data-testid="add-from-library">
              {library.map((row) => (
                <li key={row.id}>
                  <button type="button" onClick={() => { for (const agent of agents) void silent("export_skill_to_project", { id: "fixture", skillId: row.id, agent }); setAdding(false); }}>{row.name}</button>
                </li>
              ))}
            </ul>
          ) : null}
        </Card>
      ) : null}
    </section>
  );
}
