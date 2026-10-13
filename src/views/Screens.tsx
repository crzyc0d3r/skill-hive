import { useLocation, useOutletContext } from "react-router-dom";
import type { ShellOutletContext } from "../components/shell/AppShell";
import type { ThemeChoice } from "../lib/theme";

export function DashboardView() {
  const { listing } = useOutletContext<ShellOutletContext>();
  return (
    <section>
      <h1 className="font-mono text-[22px] font-semibold tracking-tight">Dashboard</h1>
      <p className="mt-2 max-w-prose text-sm text-muted-foreground">
        The library is empty until Phase 1. Directories added under the development base appear here
        when the file watcher emits.
      </p>
      <ul data-testid="base-listing" className="mt-4 space-y-1">
        {listing.length === 0 ? (
          <li className="text-sm text-muted-foreground">No skill directories yet.</li>
        ) : (
          listing.map((name) => (
            <li key={name} className="font-mono text-sm">
              {name}
            </li>
          ))
        )}
      </ul>
    </section>
  );
}

export function SettingsView() {
  const { theme, setTheme } = useOutletContext<ShellOutletContext>();
  const choices: ThemeChoice[] = ["light", "dark", "system"];
  return (
    <section>
      <h1 className="font-mono text-[22px] font-semibold">Settings</h1>
      <h2 className="mt-4 text-sm font-medium">Appearance</h2>
      <div role="group" aria-label="Theme" className="mt-2 flex gap-2">
        {choices.map((choice) => (
          <button
            key={choice}
            type="button"
            aria-pressed={theme === choice}
            className={`rounded-md border px-3 py-1 text-sm ${
              theme === choice
                ? "border-ring bg-primary text-primary-foreground"
                : "border-border bg-secondary text-secondary-foreground"
            }`}
            onClick={() => setTheme(choice)}
          >
            {choice[0].toUpperCase() + choice.slice(1)}
          </button>
        ))}
      </div>
    </section>
  );
}

export function PlaceholderView() {
  const location = useLocation();
  return (
    <section>
      <h1 className="font-mono text-[22px] font-semibold">Placeholder</h1>
      <p className="mt-2 text-sm text-muted-foreground">
        {location.pathname} arrives in a later phase.
      </p>
    </section>
  );
}

export function TokensView() {
  const swatches = [
    "background",
    "foreground",
    "sidebar",
    "card",
    "muted",
    "primary",
    "success",
    "warning",
    "destructive",
    "info",
  ];
  return (
    <section>
      <h1 className="font-mono text-[22px] font-semibold">Tokens</h1>
      <ul className="mt-4 grid grid-cols-2 gap-2">
        {swatches.map((name) => (
          <li key={name} className="flex items-center gap-2 text-sm">
            <span
              className="inline-block h-6 w-6 rounded-sm border border-border"
              style={{ background: `var(--${name})` }}
            />
            {name}
          </li>
        ))}
      </ul>
    </section>
  );
}
