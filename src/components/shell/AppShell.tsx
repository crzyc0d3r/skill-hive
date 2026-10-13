import { useQuery } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useMemo, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Toaster } from "sonner";
import { NAV_ITEMS } from "../../app/routes";
import { BASE_LISTING_KEY } from "../../lib/events/files";
import { listBaseEntries } from "../../lib/ipc/settings";
import { applyTheme, isThemeChoice, type ThemeChoice } from "../../lib/theme";
import { getSettings, setSettings } from "../../lib/ipc/settings";
import { Kbd } from "../Kbd";
import { StatusBanner } from "../StatusBanner";

const ACTIONS = [
  { id: "settings", label: "Open Settings", href: "/settings" },
  { id: "library", label: "Open Library", href: "/my-skills" },
  { id: "dashboard", label: "Open Dashboard", href: "/" },
];

export function AppShell({ platform = "linux" }: { platform?: string }) {
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [theme, setTheme] = useState<ThemeChoice>("system");
  const [banner, setBanner] = useState<string | null>(null);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onMedia = () => {
      setTheme((current) => {
        if (current === "system") applyTheme("system", media.matches);
        return current;
      });
    };
    media.addEventListener("change", onMedia);
    let stop: (() => void) | undefined;
    void getCurrentWindow()
      .onThemeChanged(({ payload }) => {
        setTheme((current) => {
          if (current === "system") applyTheme("system", payload === "dark");
          return current;
        });
      })
      .then((unlisten) => {
        stop = unlisten;
      })
      .catch(() => {
        /* Vite alone has no Tauri window. */
      });
    return () => {
      media.removeEventListener("change", onMedia);
      stop?.();
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    getSettings("theme")
      .then((value) => {
        if (cancelled) return;
        const choice = isThemeChoice(value) ? value : "system";
        setTheme(choice);
        applyTheme(choice);
      })
      .catch(() => {
        if (!cancelled) setBanner("Could not load settings.");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey;
      if (!mod) return;
      const key = event.key.toLowerCase();
      if (key === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
        setQuery("");
      } else if (key === "b") {
        event.preventDefault();
        setCollapsed((value) => !value);
      } else if (event.key === ",") {
        event.preventDefault();
        navigate("/settings");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate]);

  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return ACTIONS;
    return ACTIONS.filter((action) => action.label.toLowerCase().includes(needle));
  }, [query]);

  const listing = useQuery({
    queryKey: BASE_LISTING_KEY,
    queryFn: () => listBaseEntries().catch(() => [] as string[]),
  });

  const title = NAV_ITEMS.find((item) => item.path === location.pathname)?.nav ?? "Skill Studio";

  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
      {platform === "mac" ? (
        <div data-testid="drag-strip" data-tauri-drag-region className="h-7 shrink-0 bg-sidebar" />
      ) : null}
      <StatusBanner message={banner} />
      <div className="flex min-h-0 flex-1">
        <aside
          data-testid="sidebar"
          data-collapsed={collapsed ? "true" : "false"}
          className="flex shrink-0 flex-col border-r border-border bg-sidebar"
          style={{ width: collapsed ? "56px" : "240px" }}
        >
          <div className="flex h-11 items-center px-3 font-mono text-sm font-medium">
            {collapsed ? "SS" : "Skill Studio"}
          </div>
          <nav aria-label="Primary" className="flex flex-1 flex-col gap-1 px-2">
            {NAV_ITEMS.map((item) => (
              <NavLink
                key={item.path}
                to={item.path}
                title={item.nav}
                className={({ isActive }) =>
                  `flex h-8 items-center rounded-md px-2 text-sm ${
                    isActive ? "bg-primary text-primary-foreground" : "hover:bg-accent"
                  }`
                }
              >
                {collapsed ? item.nav?.slice(0, 1) : item.nav}
              </NavLink>
            ))}
          </nav>
        </aside>
        <div className="flex min-w-0 flex-1 flex-col">
          <header
            data-testid="header"
            className="sticky top-0 z-10 flex h-11 items-center justify-between border-b border-border bg-background px-4"
          >
            <div data-testid="breadcrumb" className="text-sm text-muted-foreground">
              {title}
            </div>
            <div data-testid="toolbar" className="text-xs text-muted-foreground">
              <Kbd>Mod+K</Kbd>
            </div>
          </header>
          <main
            data-testid="content"
            className="min-h-0 flex-1 overflow-auto px-6 py-4"
            style={{ scrollPaddingTop: "44px" }}
          >
            <Outlet
              context={{
                listing: listing.data ?? [],
                theme,
                setTheme: (choice: ThemeChoice) => {
                  setTheme(choice);
                  applyTheme(choice);
                  void setSettings("theme", choice);
                },
              }}
            />
          </main>
        </div>
      </div>
      <footer
        role="status"
        data-testid="status-bar"
        className="flex h-6 items-center justify-between border-t border-border bg-sidebar px-3 text-xs text-muted-foreground"
      >
        <span>Ready</span>
        <span>Skill Studio 0.0.1</span>
      </footer>
      {paletteOpen ? (
        <div
          data-testid="command-palette"
          role="dialog"
          aria-modal="true"
          aria-label="Command palette"
          className="fixed inset-0 z-[60] flex items-start justify-center bg-[rgb(15_23_42/0.4)] pt-24"
          onMouseDown={() => setPaletteOpen(false)}
        >
          <div
            className="w-[32rem] max-w-[90vw] rounded-lg border border-border bg-popover p-3 shadow-md"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <input
              autoFocus
              aria-label="Command search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search skills, presets, and actions"
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring"
            />
            <ul className="mt-2">
              {matches.length === 0 ? (
                <li className="px-2 py-3 text-sm text-muted-foreground">
                  No matches for '{query}'. Try a skill name, a run id, or an action.
                </li>
              ) : (
                matches.map((action) => (
                  <li key={action.id}>
                    <button
                      type="button"
                      className="flex w-full items-center justify-between rounded-md px-2 py-2 text-left text-sm hover:bg-accent"
                      onClick={() => {
                        setPaletteOpen(false);
                        navigate(action.href);
                      }}
                    >
                      {action.label}
                      <Kbd>Enter</Kbd>
                    </button>
                  </li>
                ))
              )}
            </ul>
          </div>
        </div>
      ) : null}
      <Toaster />
    </div>
  );
}

export interface ShellOutletContext {
  listing: string[];
  theme: ThemeChoice;
  setTheme: (choice: ThemeChoice) => void;
}
