import { HELP_CARDS } from "@/lib/shell/help";
import { t } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

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
    <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent aria-label={t("help.title")} className="sm:max-w-xl" showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{t("help.title")}</DialogTitle>
          <DialogDescription>Eight guides for the parts of Skill Hive you use every day.</DialogDescription>
        </DialogHeader>
        <ul className="grid grid-cols-2 gap-2 text-sm">
          {HELP_CARDS.map((key) => (
            <li key={key} data-help-card={key}>
              <Card className="gap-0 rounded-lg py-3 shadow-none">
                <a className="px-3 text-sm leading-5 text-info-text underline-offset-2 hover:underline" href={`https://github.com/crzyc0d3r/skill-hive/blob/main/docs/user-guide.md#${HELP_ANCHORS[key]}`}>{t(key)}</a>
              </Card>
            </li>
          ))}
        </ul>
        <div className="flex justify-end">
          <Button type="button" size="sm" variant="outline" onClick={onClose}>Close</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
