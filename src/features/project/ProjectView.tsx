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
import { addProject, pullProjectCenter, setSkillsEnabled, syncSkillToTool } from "@/lib/ipc/actions";

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
    <section data-testid="project" className="flex w-full flex-col gap-4">
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("project.title")}</h1>
      <Card className="max-w-xl gap-3 rounded-lg border-border px-4 py-4 shadow-none">
        <p className="text-sm text-muted-foreground">{t("project.linked")}</p>
        <div className="flex flex-wrap gap-2">
          <Button type="button" data-testid="link-project" size="sm" onClick={() => {
              setLinked(true);
              setProjectSkills([skill]);
              void addProject("fixture", "");
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
          <Button type="button" variant="outline" size="sm" onClick={() => { update(addSkillToAgents(skill, ["claude_code", "codex"])); void syncSkillToTool(skill.id, "claude_code"); void syncSkillToTool(skill.id, "codex"); }}>
            {t("project.add")}
          </Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { update(toggleSkill(skill, !skill.enabled)); void setSkillsEnabled([skill.id], !skill.enabled); }}>
            {t("project.disable")}
          </Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { update(pullCenter(skill)); void pullProjectCenter(skill.name, skill.center); }}>
            {t("project.pull")}
          </Button>
          <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { update({ ...skill, enabled: true }); void setSkillsEnabled([skill.id], true); }}>
            {t("project.batch")}
          </Button>
          <div className="flex gap-2">
            <span>{t("library.detail.local")}</span>
            <span>{t("library.detail.diff")}</span>
            <span>{t("workspace.detail.center")}</span>
          </div>
        </Card>
      ) : null}
    </section>
  );
}
