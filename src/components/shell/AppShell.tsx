import { useQuery } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useMemo, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Toaster } from "sonner";
import { NAV_ITEMS } from "../../app/routes";
import { BASE_LISTING_KEY, SKILLS_KEY } from "../../lib/events/files";
import { listBaseEntries } from "../../lib/ipc/settings";
import { applyTheme, isThemeChoice, type ThemeChoice } from "../../lib/theme";
import { getSettings, setSettings } from "../../lib/ipc/settings";
import { applyTextSize, isTextSize, t } from "../../lib/i18n";
import { listAgents, listPresets, listSkills, type AgentRow, type PresetRow } from "../../lib/phase1/client";
import { moveById } from "../../lib/phase1/model";
import { Kbd } from "../Kbd";
import { StatusBanner } from "../StatusBanner";

const ACTIONS = [
  { id: "settings", label: "Open Settings", href: "/settings" },
  { id: "library", label: "Open Library", href: "/my-skills" },
  { id: "dashboard", label: "Open Dashboard", href: "/" },
];

const SHORTCUTS = [
  { id: "1", href: "/" },
  { id: "2", href: "/my-skills" },
  { id: "3", href: "/install" },
  { id: "4", href: "/global-workspace" },
  { id: "5", href: "/settings" },
];

export function AppShell({ platform = "linux" }: { platform?: string }) {
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [theme, setTheme] = useState<ThemeChoice>("system");
  const [banner, setBanner] = useState<string | null>(null);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  const [shortcutRows, setShortcutRows] = useState(SHORTCUTS);
  const [presetRows, setPresetRows] = useState<PresetRow[] | null>(null);
  const [dragPreset, setDragPreset] = useState<string | null>(null);
  const [editingPreset, setEditingPreset] = useState<string | null>(null);
  const [presetName, setPresetName] = useState("");

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
    try {
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
    } catch {
      /* getCurrentWindow throws outside the webview. */
    }
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
        setBanner(null);
      })
      .catch(() => {
        if (!cancelled) setBanner(t("banner.load"));
      });
    getSettings("textSize")
      .then((value) => {
        if (!cancelled && isTextSize(value)) applyTextSize(value);
      })
      .catch(() => {
        /* text size is optional */
      });
    getSettings("shortcuts")
      .then((value) => {
        if (cancelled || !value) return;
        const parsed = JSON.parse(value) as { id: string; href: string }[];
        if (Array.isArray(parsed) && parsed.every((item) => item.id && item.href)) setShortcutRows(parsed);
      })
      .catch(() => {
        /* shortcuts stay on the defaults */
      });
    return () => {
      cancelled = true;
    };
  }, [reloadKey]);

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
      } else if (key === "f") {
        event.preventDefault();
        document.querySelector<HTMLInputElement>("[data-search]")?.focus();
      } else if (event.key === "/") {
        event.preventDefault();
        setShortcutsOpen(true);
      } else {
        const shortcut = shortcutRows.find((item) => item.id === key);
        if (shortcut) {
          event.preventDefault();
          navigate(shortcut.href);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate, shortcutRows]);

  const skills = useQuery({ queryKey: SKILLS_KEY, queryFn: listSkills });
  const presets = useQuery({ queryKey: ["presets"], queryFn: listPresets });
  const agents = useQuery({ queryKey: ["agents"], queryFn: listAgents });
  const shownPresets = useMemo(() => presetRows ?? presets.data ?? [], [presetRows, presets.data]);

  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const skillHits = (skills.data ?? [])
      .filter((skill) => !needle || skill.name.toLowerCase().includes(needle))
      .map((skill) => ({ id: skill.id, label: skill.name, href: `/my-skills?skill=${skill.id}`, group: "Skills" }));
    const presetHits = shownPresets
      .filter((preset) => !needle || preset.name.toLowerCase().includes(needle))
      .map((preset) => ({ id: preset.id, label: preset.name, href: "/global-workspace", group: "Presets" }));
    const actionHits = ACTIONS.filter((action) => !needle || action.label.toLowerCase().includes(needle)).map((action) => ({
      ...action,
      group: "Actions",
    }));
    return [...skillHits, ...presetHits, ...actionHits];
  }, [query, skills.data, shownPresets]);

  const listing = useQuery({
    queryKey: BASE_LISTING_KEY,
    queryFn: () => listBaseEntries().catch(() => [] as string[]),
  });

  const title = NAV_ITEMS.find((item) => item.path === location.pathname)?.nav ?? "Skill Hive";

  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
      {platform === "mac" ? (
        <div data-testid="drag-strip" data-tauri-drag-region className="h-7 shrink-0 bg-sidebar" />
      ) : null}
      <StatusBanner message={banner} onRetry={() => setReloadKey((value) => value + 1)} />
      <div className="flex min-h-0 flex-1">
        <aside
          data-testid="sidebar"
          data-collapsed={collapsed ? "true" : "false"}
          className="flex shrink-0 flex-col border-r border-border bg-sidebar"
          style={{ width: collapsed ? "56px" : "240px" }}
        >
          <div className="flex h-11 items-center px-3 font-mono text-sm font-medium">
            {collapsed ? "SH" : "Skill Hive"}
          </div>
          <nav aria-label="Primary" className="flex flex-1 flex-col gap-1 overflow-auto px-2">
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
            <div className="mt-3 px-2 text-xs text-muted-foreground">{t("workspace.title")}</div>
            <NavLink to="/global-workspace" className="flex h-8 items-center rounded-md px-2 text-sm hover:bg-accent">
              {collapsed ? "G" : t("workspace.allAgents")}
            </NavLink>
            {(agents.data ?? []).map((agent) => (
              <NavLink key={agent.key} to={`/global-workspace/${agent.key}`} className="flex h-8 items-center rounded-md px-2 text-sm hover:bg-accent">
                {collapsed ? agent.name.slice(0, 1) : agent.name}
              </NavLink>
            ))}
            <div className="mt-3 px-2 text-xs text-muted-foreground">Presets</div>
            {shownPresets.map((preset) => (
              <div
                key={preset.id}
                draggable
                data-preset={preset.id}
                onDragStart={() => setDragPreset(preset.id)}
                onDragOver={(event) => event.preventDefault()}
                onDrop={() => {
                  if (!dragPreset) return;
                  setPresetRows(moveById(shownPresets, dragPreset, preset.id, (item) => item.id));
                  setDragPreset(null);
                }}
                className="flex items-center gap-1"
              >
                <NavLink to="/global-workspace" className="flex h-8 min-w-0 flex-1 items-center rounded-md px-2 text-sm hover:bg-accent">
                  {collapsed ? preset.name.slice(0, 1) : preset.name}
                </NavLink>
                {collapsed ? null : (
                  <>
                    <button type="button" className="text-xs" aria-label={`${t("preset.edit")} ${preset.name}`} onClick={() => { setEditingPreset(preset.id); setPresetName(preset.name); }}>{t("preset.edit")}</button>
                    <button type="button" className="text-xs" aria-label={`${t("preset.delete")} ${preset.name}`} onClick={() => setPresetRows(shownPresets.filter((item) => item.id !== preset.id))}>{t("preset.delete")}</button>
                  </>
                )}
              </div>
            ))}
            {editingPreset ? (
              <form
                className="flex gap-1 px-2"
                onSubmit={(event) => {
                  event.preventDefault();
                  setPresetRows(shownPresets.map((preset) => preset.id === editingPreset ? { ...preset, name: presetName } : preset));
                  setEditingPreset(null);
                }}
              >
                <input aria-label={t("preset.edit")} value={presetName} onChange={(event) => setPresetName(event.target.value)} className="w-full rounded-md border border-input px-1 py-0.5 text-xs" />
                <button type="submit" className="text-xs">{t("preset.save")}</button>
              </form>
            ) : null}
            <NavLink to="/global-workspace" className="flex h-8 items-center rounded-md px-2 text-sm hover:bg-accent">
              {collapsed ? "+" : t("preset.new")}
            </NavLink>
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
                skills: skills.data ?? [],
                presets: shownPresets,
                setPresets: (next: PresetRow[]) => setPresetRows(next),
                agents: (agents.data ?? []) as AgentRow[],
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
        <span>Skill Hive 0.0.1</span>
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
              onKeyDown={(event) => {
                if (event.key === "Enter" && matches[0]) {
                  event.preventDefault();
                  setPaletteOpen(false);
                  navigate(matches[0].href);
                }
              }}
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring"
            />
            <ul className="mt-2">
              {matches.length === 0 ? (
                <li className="px-2 py-3 text-sm text-muted-foreground">{t("palette.empty", { query })}</li>
              ) : (
                matches.map((action) => (
                  <li key={`${action.group}-${action.id}`}>
                    <button
                      type="button"
                      className="flex w-full items-center justify-between rounded-md px-2 py-2 text-left text-sm hover:bg-accent"
                      onClick={() => {
                        setPaletteOpen(false);
                        navigate(action.href);
                      }}
                    >
                      <span>{action.label}</span>
                      <Kbd>Enter</Kbd>
                    </button>
                  </li>
                ))
              )}
            </ul>
          </div>
        </div>
      ) : null}
      {shortcutsOpen ? (
        <div role="dialog" aria-label={t("shortcuts.title")} className="fixed inset-0 z-[60] flex items-center justify-center bg-[rgb(15_23_42/0.4)]">
          <div className="rounded-lg border border-border bg-popover p-4">
            <h2 className="font-medium">{t("shortcuts.title")}</h2>
            <ul className="mt-2 space-y-1 text-sm">
              {shortcutRows.map((item) => (
                <li key={item.id} className="flex items-center gap-2">
                  <span>Mod+{item.id}</span>
                  <input
                    aria-label={`Shortcut ${item.id}`}
                    value={item.href}
                    onChange={(event) => setShortcutRows((current) => current.map((row) => row.id === item.id ? { ...row, href: event.target.value } : row))}
                    className="rounded-md border border-input px-1 py-0.5"
                  />
                </li>
              ))}
              <li>Mod+F</li>
              <li>Mod+/</li>
            </ul>
            <button
              type="button"
              className="mt-3 rounded-md border border-border px-2 py-1 text-sm"
              onClick={() => {
                void setSettings("shortcuts", JSON.stringify(shortcutRows));
              }}
            >
              {t("shortcuts.save")}
            </button>
            <button type="button" className="mt-3 ml-2 rounded-md border border-border px-2 py-1 text-sm" onClick={() => setShortcutsOpen(false)}>
              Close
            </button>
          </div>
        </div>
      ) : null}
      <Toaster />
    </div>
  );
}

export interface ShellOutletContext {
  listing: string[];
  skills: import("../../lib/phase1/model").SkillCard[];
  presets: PresetRow[];
  setPresets: (next: PresetRow[]) => void;
  agents: AgentRow[];
  theme: ThemeChoice;
  setTheme: (choice: ThemeChoice) => void;
}
