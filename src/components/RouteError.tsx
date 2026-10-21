import { Component, type ReactNode } from "react";
import { getSettings } from "@/lib/ipc/settings";
import { applyCrashTheme, applyTheme, isThemeChoice } from "@/lib/theme";

/** Dev-only throw so screenshots can show each error boundary. */
export function DevCrash({ level, children }: { level: "app" | "route"; children: ReactNode }) {
  if (import.meta.env.DEV && typeof window !== "undefined") {
    const crash = new URLSearchParams(window.location.search).get("crash");
    if (crash === level) {
      throw new Error(level === "app" ? "Top-level render failed" : "Route render failed");
    }
  }
  return children;
}
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
  private themeSynced = false;

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

  componentDidMount(): void {
    this.syncStoredTheme();
  }

  componentDidUpdate(): void {
    this.syncStoredTheme();
  }

  /** The shell is unmounted on a top-level crash, so read the saved theme here. */
  private syncStoredTheme(): void {
    if (this.props.route || !this.state.error || this.themeSynced) return;
    this.themeSynced = true;
    applyCrashTheme();
    void getSettings("theme")
      .then((value) => {
        if (isThemeChoice(value)) applyTheme(value);
      })
      .catch(() => {
        /* Keep the stored theme, or the dark default. */
      });
  }

  render() {
    if (!this.state.error) return this.props.children;
    const topLevel = !this.props.route;
    if (topLevel) applyCrashTheme();
    const alert = (
      <div role="alert" data-testid="route-error" className="rounded-lg border border-destructive/40 bg-card px-4 py-4 text-card-foreground">
        <h1 className="font-sans text-2xl font-semibold">{t("shell.error")}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{t("shell.renderError")}</p>
        <pre className="mt-3 overflow-auto whitespace-pre-wrap font-mono text-xs">{this.state.error.message}</pre>
        <Button type="button" size="sm" className="mt-3" onClick={() => window.location.reload()}>
          {t("shell.reload")}
        </Button>
      </div>
    );
    if (!topLevel) return alert;
    return (
      <div className="flex min-h-full items-center justify-center bg-background p-6 text-foreground">
        <div className="w-full max-w-md text-center [&_h1]:font-sans [&_pre]:text-left">{alert}</div>
      </div>
    );
  }
}
