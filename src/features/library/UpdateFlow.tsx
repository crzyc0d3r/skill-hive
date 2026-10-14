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
      <button type="button" data-testid="check-all" className="rounded-md border border-border px-2 py-1" onClick={() => { setOpen(true); void checkAllSkillUpdates(); }}>{t("library.checkAll")}</button>
      <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => { setOpen(true); void checkAllSkillUpdates(); }}>{t("library.menu.update")}</button>
      {open ? (
        <div role="dialog" aria-label={t("library.removal.title")} className="rounded-md border border-border p-3">
          <p>{t("library.removal.title")}</p>
          <ul>{plan.paths.map((path) => <li key={path}>{path}</li>)}</ul>
          <button type="button" data-testid="update-anyway" className="mt-2 rounded-md bg-primary px-2 py-1 text-primary-foreground" onClick={() => setApplied(applyRemoval(plan, plan.token))}>
            {t("library.removal.anyway")}
          </button>
        </div>
      ) : null}
      {applied ? <p role="status">{t("library.updateAll")}</p> : null}
      <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => setApplied(false)}>{t("library.relink")}</button>
      <button type="button" className="ml-2 rounded-md border border-border px-2 py-1">{t("library.keepLocal")}</button>
    </div>
  );
}
