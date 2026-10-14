import { t } from "@/lib/i18n";

export function GitStatusButton() {
  return (
    <button type="button" className="rounded-md border border-border px-2 py-1 text-sm">{t("library.gitStatus")}</button>
  );
}
