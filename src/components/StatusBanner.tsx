import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { t } from "../lib/i18n";

export function StatusBanner({ message, onRetry }: { message: string | null; onRetry?: () => void }) {
  if (!message) {
    return <div data-testid="status-banner" hidden />;
  }
  return (
    <Alert
      data-testid="status-banner"
      variant="destructive"
      className="rounded-none border-x-0 border-t-0 bg-destructive-subtle text-destructive-subtle-foreground"
    >
      <AlertTitle className="sr-only">{t("banner.retry")}</AlertTitle>
      <AlertDescription className="flex items-center justify-between gap-3 text-destructive-subtle-foreground">
        <span>{message}</span>
        <Button type="button" size="sm" variant="outline" onClick={onRetry}>
          {t("banner.retry")}
        </Button>
      </AlertDescription>
    </Alert>
  );
}
