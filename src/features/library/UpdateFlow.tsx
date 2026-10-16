import { Button } from "@/components/ui/button";
import { useState } from "react";
import { applyRemoval, removalPlan } from "@/features/library/updates";
import { checkAllSkillUpdates } from "@/lib/ipc/actions";
import { t } from "@/lib/i18n";

export function UpdateFlow() {
  const plan = removalPlan(["notes.txt"]);
  const [open, setOpen] = useState(false);
  const [applied, setApplied] = useState<boolean | null>(null);
  return (
    <div data-testid="updates" className="flex flex-wrap items-center gap-1">
      <Button type="button" data-testid="check-all" variant="ghost" size="sm" className="text-muted-foreground" onClick={() => { setOpen(true); void checkAllSkillUpdates(); }}>{t("library.checkAll")}</Button>
      <Button type="button" variant="ghost" size="sm" className="text-muted-foreground" onClick={() => { setOpen(true); void checkAllSkillUpdates(); }}>{t("library.menu.update")}</Button>
      {open ? (
        <div role="dialog" aria-label={t("library.removal.title")} className="basis-full rounded-lg border border-border bg-card p-3">
          <p>{t("library.removal.title")}</p>
          <ul>{plan.paths.map((path) => <li key={path}>{path}</li>)}</ul>
          <Button type="button" data-testid="update-anyway" variant="default" size="sm" className="mt-2" onClick={() => setApplied(applyRemoval(plan, plan.token))}>
            {t("library.removal.anyway")}
          </Button>
          <Button type="button" variant="ghost" size="sm" className="ml-2" onClick={() => setApplied(false)}>{t("library.relink")}</Button>
          <Button type="button" variant="ghost" size="sm">{t("library.keepLocal")}</Button>
        </div>
      ) : null}
      {applied ? <p role="status" className="basis-full">{t("library.updateAll")}</p> : null}
    </div>
  );
}
