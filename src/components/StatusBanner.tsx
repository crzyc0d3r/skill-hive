import { cn } from "cn";
import { Button } from "@/components/ui/button";
import { t } from "../lib/i18n";

export function StatusBanner({
  message,
  onRetry,
  className,
}: {
  message: string | null;
  onRetry?: () => void;
  className?: string;
}) {
  if (!message) {
    return <div data-testid="status-banner" hidden />;
  }
  return (
    <div
      data-testid="status-banner"
      role="alert"
      className={cn(
        "flex items-center justify-between gap-3 border-b border-destructive/30 bg-destructive-subtle px-4 py-2 text-sm text-destructive-subtle-foreground",
        className,
      )}
    >
      <span>{message}</span>
      <Button
        type="button"
        size="sm"
        variant="outline"
        className="border-destructive-subtle-foreground/40 bg-destructive-subtle text-destructive-subtle-foreground shadow-none hover:bg-destructive-subtle dark:border-destructive-subtle-foreground/40 dark:bg-destructive-subtle dark:text-destructive-subtle-foreground dark:hover:bg-destructive-subtle"
        onClick={onRetry}
      >
        {t("banner.retry")}
      </Button>
    </div>
  );
}
