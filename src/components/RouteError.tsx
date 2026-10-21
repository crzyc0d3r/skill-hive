import { Component, type ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { silent } from "@/lib/ipc/actions";
import { t } from "@/lib/i18n";

interface Props {
  children: ReactNode;
  route?: string;
}

interface State {
  error: Error | null;
}

export class RouteErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error): void {
    const route = this.props.route ?? (typeof location !== "undefined" ? location.pathname : "");
    void silent("log_frontend_error", {
      message: error.message,
      stack: error.stack ?? "",
      route,
    });
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div role="alert" data-testid="route-error" className="rounded-lg border border-destructive/40 bg-card px-4 py-4">
        <h1 className="font-mono text-lg font-semibold">{t("shell.error")}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{t("shell.renderError")}</p>
        <pre className="mt-3 overflow-auto whitespace-pre-wrap font-mono text-xs">{this.state.error.message}</pre>
        <Button type="button" size="sm" className="mt-3" onClick={() => window.location.reload()}>
          {t("shell.reload")}
        </Button>
      </div>
    );
  }
}
