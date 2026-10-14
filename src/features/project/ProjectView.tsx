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
import { addProject, setSkillsEnabled, syncSkillToTool } from "@/lib/ipc/actions";
import { persistEffect } from "@/lib/ipc/persist";

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
  const update = (next: ProjectSkill) => {
    setSkill(next);
    setProjectSkills([next]);
  };
  return (
    <section data-testid="project">
      <h1 className="font-mono text-[22px] font-semibold">{t("project.title")}</h1>
      <button
        type="button"
        data-testid="link-project"
        className="mt-3 rounded-md border border-border px-2 py-1 text-sm"
        onClick={() => {
          setLinked(true);
          setProjectSkills([skill]);
          void addProject("fixture", "");
        }}
      >
        {t("project.link")}
      </button>
      <button type="button" className="ml-2 rounded-md border border-border px-2 py-1 text-sm" onClick={() => { setLinked(true); void addProject(skill.name, ""); }}>
        {t("project.linked")}
      </button>
      {linked ? (
        <div className="mt-4 space-y-2 text-sm">
          <p>
            {skill.name}
            <span data-testid="project-dot" data-color={dotColor(skill)} className="ml-2 inline-block h-2 w-2 rounded-full bg-current" />
          </p>
          <p>{t("project.agents")}: {skill.agents.join(", ") || "—"}</p>
          <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => { update(addSkillToAgents(skill, ["claude_code", "codex"])); void syncSkillToTool(skill.id, "claude_code"); void syncSkillToTool(skill.id, "codex"); }}>
            {t("project.add")}
          </button>
          <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => { update(toggleSkill(skill, !skill.enabled)); void setSkillsEnabled([skill.id], !skill.enabled); }}>
            {t("project.disable")}
          </button>
          <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => { update(pullCenter(skill)); persistEffect("project", skill.id, skill.center); }}>
            {t("project.pull")}
          </button>
          <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => { update({ ...skill, enabled: true }); void setSkillsEnabled([skill.id], true); }}>
            {t("project.batch")}
          </button>
          <div className="flex gap-2">
            <span>{t("library.detail.local")}</span>
            <span>{t("library.detail.diff")}</span>
            <span>{t("workspace.detail.center")}</span>
          </div>
        </div>
      ) : null}
    </section>
  );
}
