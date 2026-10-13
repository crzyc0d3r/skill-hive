import { t } from "../lib/i18n";

export function StatusBanner({ message, onRetry }: { message: string | null; onRetry?: () => void }) {
  if (!message) {
    return <div data-testid="status-banner" hidden />;
  }
  return (
    <div
      data-testid="status-banner"
      role="alert"
      className="flex items-center justify-between gap-3 border-b border-border bg-destructive-subtle px-4 py-2 text-sm text-destructive-subtle-foreground"
    >
      <span>{message}</span>
      <button type="button" className="rounded-md border border-border px-2 py-1" onClick={onRetry}>
        {t("banner.retry")}
      </button>
    </div>
  );
}
