import { useState } from "react";
import { t } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";

export function CloseGuardDialog({
  onRemember,
  onClose,
}: {
  onRemember: (choice: "minimize" | "quit") => void;
  onClose: () => void;
}) {
  const [remember, setRemember] = useState(false);
  return (
    <Dialog open onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent aria-label={t("close.dialog")} className="sm:max-w-md" showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{t("close.dialog")}</DialogTitle>
          <DialogDescription>Choose what the window does when you close it.</DialogDescription>
        </DialogHeader>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox aria-label={t("close.remember")} checked={remember} onCheckedChange={(value) => setRemember(value === true)} />
          <span>{t("close.remember")}</span>
        </label>
        <div className="flex justify-end gap-2">
          <Button type="button" size="sm" variant="outline" onClick={() => { if (remember) onRemember("minimize"); onClose(); }}>{t("settings.close.minimize")}</Button>
          <Button type="button" size="sm" variant="destructive" onClick={() => { if (remember) onRemember("quit"); onClose(); }}>{t("settings.close.quit")}</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
