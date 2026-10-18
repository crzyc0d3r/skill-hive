import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useEffect, useState } from "react";
import { applyRemoval } from "@/features/library/updates";
import { applyAvailableSkillUpdates, checkAllSkillUpdates, silent } from "@/lib/ipc/actions";
import type { SkillCard } from "@/features/library/model";
import { t } from "@/lib/i18n";

export function UpdateFlow({ skills = [] }: { skills?: SkillCard[] }) {
  const [open, setOpen] = useState(false);
  const [applied, setApplied] = useState<boolean | null>(null);
  const [available, setAvailable] = useState(0);
  const [paths, setPaths] = useState<string[]>([]);
  const [relinkPath, setRelinkPath] = useState("");
  const plan = { paths, token: paths.join("|") };

  useEffect(() => {
    void silent<number>("check_all_skill_updates").then((count) => {
      if (typeof count === "number") setAvailable(count);
    });
  }, []);

  return (
    <div data-testid="updates" className="flex flex-wrap items-center gap-1">
      <Button type="button" data-testid="check-all" variant="ghost" size="sm" className="text-muted-foreground" onClick={() => { setOpen(true); void checkAllSkillUpdates().then((count) => { setAvailable(count); setPaths(skills.filter((skill) => skill.updateStatus === "pending_removals").map((skill) => skill.name)); }); }}>{t("library.checkAll")}</Button>
      <Button type="button" variant="ghost" size="sm" className="text-muted-foreground" onClick={() => { setOpen(true); void applyAvailableSkillUpdates().then((count) => setApplied((count ?? 0) > 0)); }}>{t("library.menu.update")}{available > 0 ? ` (${available})` : ""}</Button>
      {open ? (
        <div role="dialog" aria-label={t("library.removal.title")} className="basis-full rounded-lg border border-border bg-card p-3">
          <p>{t("library.removal.title")}</p>
          <ul>{plan.paths.map((path) => <li key={path}>{path}</li>)}</ul>
          <Button type="button" data-testid="update-anyway" variant="default" size="sm" className="mt-2" onClick={() => { setApplied(applyRemoval(plan, plan.token)); void applyAvailableSkillUpdates(); }}>
            {t("library.removal.anyway")}
          </Button>
          <Input aria-label={t("library.relink")} value={relinkPath} onChange={(event) => setRelinkPath(event.target.value)} className="mt-2 h-8" />
          <Button type="button" variant="ghost" size="sm" onClick={() => { const path = relinkPath.trim(); if (!path) return; for (const skill of skills) void silent("relink_local_skill_source", { skillId: skill.id, path }); }}>{t("library.relink")}</Button>
          <Button type="button" variant="ghost" size="sm" onClick={() => { for (const skill of skills.filter((item) => item.updateStatus === "source_missing" || item.source === "local")) void silent("detach_local_skill_source", { skillId: skill.id }); }}>{t("library.keepLocal")}</Button>
        </div>
      ) : null}
      {applied ? <p role="status" className="basis-full">{t("library.updateAll")}</p> : null}
    </div>
  );
}
