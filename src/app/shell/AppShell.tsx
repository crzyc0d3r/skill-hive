import * as React from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import {
  Compass,
  Crosshair,
  FlaskConical,
  Hexagon,
  Home,
  Layers,
  Library,
  Monitor,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Search,
  Settings,
  Sun,
  type LucideIcon,
} from "lucide-react";
import { Toaster, toast } from "sonner";
import type { HiveEvent } from "@/lib/api/bindings";
import { cn, modKey } from "@/lib/utils";
import { applyTheme, useUi, type Theme } from "@/lib/store";
import { useAppInfo, useHiveEvents, useStats } from "@/lib/api/hooks";
import { Button, Kbd, Tooltip, TooltipProvider } from "@/components/ui";
import { CommandPalette } from "./CommandPalette";
import { ItemDetailSheet } from "@/features/library/ItemDetailSheet";
import { ProgressBanner } from "./ProgressBanner";

export interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
  hint: string;
  shortcut: string;
}

export const NAV: NavItem[] = [
  { to: "/", label: "Home", icon: Home, hint: "Overview of your hive", shortcut: "1" },
  { to: "/library", label: "Library", icon: Library, hint: "Everything you have collected", shortcut: "2" },
  { to: "/discover", label: "Discover", icon: Compass, hint: "Collect from this machine or online", shortcut: "3" },
  { to: "/find", label: "Find", icon: Crosshair, hint: "The right skills for a job", shortcut: "4" },
  { to: "/agents", label: "Agents", icon: Monitor, hint: "What each agent tool has installed", shortcut: "5" },
  { to: "/loadouts", label: "Loadouts", icon: Layers, hint: "Curated sets to deploy together", shortcut: "6" },
  { to: "/evals", label: "Evals", icon: FlaskConical, hint: "Measure whether a skill helps", shortcut: "7" },
];

function ThemeToggle() {
  const theme = useUi((s) => s.theme);
  const setTheme = useUi((s) => s.setTheme);
  const next: Record<Theme, Theme> = { system: "light", light: "dark", dark: "system" };
  const Icon = theme === "dark" ? Moon : theme === "light" ? Sun : Monitor;
  return (
    <Tooltip content={`Theme: ${theme}`} side="right">
      <Button variant="ghost" size="icon-sm" onClick={() => setTheme(next[theme])} aria-label="Toggle theme">
        <Icon />
      </Button>
    </Tooltip>
  );
}

function Sidebar() {
  const collapsed = useUi((s) => s.sidebarCollapsed);
  const toggle = useUi((s) => s.toggleSidebar);
  const setPaletteOpen = useUi((s) => s.setPaletteOpen);
  const { data: stats } = useStats();
  const { data: info } = useAppInfo();

  return (
    <aside
      className={cn(
        "flex h-full shrink-0 flex-col border-r bg-sunken/60 transition-[width] duration-200",
        collapsed ? "w-[60px]" : "w-[var(--sidebar-w)]",
      )}
    >
      <div className={cn("drag-region flex h-12 items-center gap-2 px-3", collapsed && "justify-center px-0")}>
        <div className="no-drag flex size-7 items-center justify-center rounded-lg bg-accent text-accent-fg shadow-sm">
          <Hexagon className="size-4" strokeWidth={2.4} />
        </div>
        {!collapsed ? (
          <div className="no-drag leading-tight">
            <div className="text-[13px] font-semibold tracking-[-0.01em]">Skill Hive</div>
            <div className="text-[11px] text-subtle">v{info?.version ?? "…"}</div>
          </div>
        ) : null}
      </div>

      <div className="px-2 pb-2">
        <button
          type="button"
          onClick={() => setPaletteOpen(true)}
          className={cn(
            "flex h-8 w-full items-center gap-2 rounded-lg bg-elevated px-2.5 text-xs text-muted hairline transition-colors hover:text-fg",
            collapsed && "justify-center px-0",
          )}
        >
          <Search className="size-3.5" />
          {!collapsed ? (
            <>
              <span className="flex-1 text-left">Search or jump to…</span>
              <Kbd>{modKey()}</Kbd>
              <Kbd>K</Kbd>
            </>
          ) : null}
        </button>
      </div>

      <nav className="flex flex-1 flex-col gap-0.5 px-2">
        {NAV.map((item) => (
          <Tooltip key={item.to} content={collapsed ? item.label : null} side="right">
            <NavLink
              to={item.to}
              end={item.to === "/"}
              className={({ isActive }) =>
                cn(
                  "group flex h-8 items-center gap-2.5 rounded-lg px-2.5 text-[13px] transition-colors",
                  isActive ? "bg-elevated text-fg shadow-sm hairline font-medium" : "text-muted hover:bg-elevated/70 hover:text-fg",
                  collapsed && "justify-center px-0",
                )
              }
            >
              <item.icon className="size-4 shrink-0" />
              {!collapsed ? (
                <>
                  <span className="flex-1">{item.label}</span>
                  {item.to === "/library" && stats ? <span className="num text-[11px] text-subtle">{stats.total}</span> : null}
                </>
              ) : null}
            </NavLink>
          </Tooltip>
        ))}
      </nav>

      <div className={cn("flex items-center gap-1 border-t p-2", collapsed ? "flex-col" : "justify-between")}>
        <Tooltip content="Settings" side="right">
          <NavLink
            to="/settings"
            className={({ isActive }) =>
              cn(
                "flex h-7 items-center gap-2 rounded-md px-2 text-xs text-muted hover:bg-elevated hover:text-fg",
                isActive && "bg-elevated text-fg",
              )
            }
          >
            <Settings className="size-4" />
            {!collapsed ? "Settings" : null}
          </NavLink>
        </Tooltip>
        <div className={cn("flex items-center", collapsed && "flex-col")}>
          <ThemeToggle />
          <Tooltip content={collapsed ? "Expand sidebar" : "Collapse sidebar"} side="right">
            <Button variant="ghost" size="icon-sm" onClick={toggle} aria-label="Toggle sidebar">
              {collapsed ? <PanelLeftOpen /> : <PanelLeftClose />}
            </Button>
          </Tooltip>
        </div>
      </div>
    </aside>
  );
}

function useGlobalShortcuts() {
  const navigate = useNavigate();
  const setPaletteOpen = useUi((s) => s.setPaletteOpen);
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen(true);
        return;
      }
      if (mod && !e.shiftKey && !e.altKey && /^[1-7]$/.test(e.key)) {
        const item = NAV.find((n) => n.shortcut === e.key);
        if (item) {
          e.preventDefault();
          navigate(item.to);
        }
      }
      if (mod && e.key === ",") {
        e.preventDefault();
        navigate("/settings");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate, setPaletteOpen]);
}

export function AppShell() {
  const theme = useUi((s) => s.theme);
  const location = useLocation();
  useGlobalShortcuts();

  React.useEffect(() => {
    applyTheme(theme);
    const mq = window.matchMedia?.("(prefers-color-scheme: dark)");
    const listener = () => applyTheme(theme);
    mq?.addEventListener?.("change", listener);
    return () => mq?.removeEventListener?.("change", listener);
  }, [theme]);

  useHiveEvents(
    React.useCallback((event: HiveEvent) => {
      if (event.type === "run_updated" && (event.status === "done" || event.status === "failed" || event.status === "cancelled")) {
        const label = event.status === "done" ? "Evaluation finished" : event.status === "failed" ? "Evaluation failed" : "Evaluation cancelled";
        (event.status === "done" ? toast.success : event.status === "failed" ? toast.error : toast)(label, {
          description: `Run ${event.run_id.slice(0, 8)}`,
        });
      }
    }, []),
  );

  return (
    <TooltipProvider delayDuration={300}>
      <div className="flex h-full w-full overflow-hidden bg-bg text-fg">
        <Sidebar />
        <main className="relative flex min-w-0 flex-1 flex-col">
          <ProgressBanner />
          <div key={location.pathname} className="min-h-0 flex-1">
            <Outlet />
          </div>
        </main>
        <CommandPalette />
        <ItemDetailSheet />
        <Toaster
          position="bottom-right"
          theme={document.documentElement.classList.contains("dark") ? "dark" : "light"}
          toastOptions={{
            className: "!bg-elevated !text-fg !border-line !shadow-md !rounded-xl !text-[13px]",
          }}
        />
      </div>
    </TooltipProvider>
  );
}
