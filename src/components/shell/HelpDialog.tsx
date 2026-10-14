import { HELP_CARDS } from "@/lib/shell/help";
import { t } from "@/lib/i18n";

const HELP_ANCHORS: Record<(typeof HELP_CARDS)[number], string> = {
  "help.card.library": "library",
  "help.card.install": "install-skills",
  "help.card.workspace": "global-workspace",
  "help.card.project": "project-workspace",
  "help.card.backup": "backup",
  "help.card.updates": "updates",
  "help.card.tray": "tray",
  "help.card.cli": "cli",
};

export function HelpDialog({ onClose }: { onClose: () => void }) {
  return (
    <div role="dialog" aria-label={t("help.title")} className="fixed inset-0 z-[70] flex items-center justify-center bg-[rgb(15_23_42/0.4)]">
      <div className="max-h-[80vh] w-[32rem] max-w-[90vw] overflow-auto rounded-lg border border-border bg-popover p-4">
        <h2 className="font-medium">{t("help.title")}</h2>
        <ul className="mt-3 grid grid-cols-2 gap-2 text-sm">
          {HELP_CARDS.map((key) => (
            <li key={key} className="rounded-md border border-border p-2" data-help-card={key}>
              <a href={`https://github.com/crzyc0d3r/skill-hive/blob/main/docs/user-guide.md#${HELP_ANCHORS[key]}`}>{t(key)}</a>
            </li>
          ))}
        </ul>
        <button type="button" className="mt-3 rounded-md border border-border px-2 py-1 text-sm" onClick={onClose}>Close</button>
      </div>
    </div>
  );
}
