import { t } from "@/lib/i18n";

function phrase(job: string, message: string): string {
  if (job === "sync" || message === "sync" || job === "backup" || message === "backup") return t("job.sync");
  if (job === "library" || message === "library") return t("job.library");
  if (job === "scan" || message === "scan") return t("job.scan");
  if (job === "install" || message === "install") return t("job.install");
  if (job === "import" || message === "import") return t("job.import");
  if (job === "effectiveness" || message === "effectiveness") return t("job.effectiveness");
  if (message && message !== job && message.includes(" ")) return /[….]$/.test(message) ? message : `${message}…`;
  return t("job.working");
}

export function jobLabel(job: string, message: string, current: number, total: number): { text: string; percent: number } {
  const percent = total <= 0 ? 0 : Math.min(100, Math.max(0, Math.round((current / total) * 100)));
  const count = total > 0 ? t("job.count", { current: String(current), total: String(total) }) : "";
  const body = count ? `${phrase(job, message)} ${percent}% · ${count}` : `${phrase(job, message)} ${percent}%`;
  return { text: t("shell.progress", { job: body }), percent };
}
