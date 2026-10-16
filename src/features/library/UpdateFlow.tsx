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
    <div data-testid="updates" className="space-y-2 text-sm">
      <Button type="button" data-testid="check-all" variant="outline" size="sm" onClick={() => { setOpen(true); void checkAllSkillUpdates(); }}>{t("library.checkAll")}</Button>
      <Button type="button" variant="outline" size="sm" className="ml-2" onClick={() => { setOpen(true); void checkAllSkillUpdates(); }}>{t("library.menu.update")}</Button>
      {open ? (
        <div role="dialog" aria-label={t("library.removal.title")} className="rounded-md border border-border p-3">
          <p>{t("library.removal.title")}</p>
          <ul>{plan.paths.map((path) => <li key={path}>{path}</li>)}</ul>
          <Button type="button" data-testid="update-anyway" variant="default" size="sm" className="mt-2" onClick={() => setApplied(applyRemoval(plan, plan.token))}>
            {t("library.removal.anyway")}
          </Button>
        </div>
      ) : null}
      {applied ? <p role="status">{t("library.updateAll")}</p> : null}
      <Button type="button" variant="outline" size="sm" onClick={() => setApplied(false)}>{t("library.relink")}</Button>
      <Button type="button" variant="outline" size="sm" className="ml-2">{t("library.keepLocal")}</Button>
    </div>
  );
}
