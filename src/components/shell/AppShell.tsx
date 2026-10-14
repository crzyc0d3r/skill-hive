import { useQuery } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useMemo, useState } from "react";
import { Link, NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Toaster } from "sonner";
import { NAV_ITEMS } from "../../app/routes";
import { BASE_LISTING_KEY, SKILLS_KEY } from "../../lib/events/files";
import { listBaseEntries } from "../../lib/ipc/settings";
import { applyTheme, isThemeChoice, type ThemeChoice } from "../../lib/theme";
import { getSettings, setSettings } from "../../lib/ipc/settings";
import { applyTextSize, isTextSize, t } from "../../lib/i18n";
import { listAgents, listPresets, listSkills, type AgentRow, type PresetRow } from "@/lib/ipc/catalog";
import { createPreset, deletePreset, reorderPresets, updatePreset } from "@/lib/ipc/actions";
import { moveById } from "@/features/library/model";
import { lobsterAgents } from "@/features/lobster/model";
import { useProjectDot } from "@/features/project/store";
import { SKILL_TEMPLATES } from "@/lib/authoring/model";
import { closePicker, createSkill, openPicker, useAuthoring } from "@/lib/authoring/store";
import { CloseGuardDialog } from "./CloseGuardDialog";
import { HelpDialog } from "./HelpDialog";
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
  { id: "3", href: "/agents" },
  { id: "4", href: "/install" },
  { id: "5", href: "/backup" },
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
  const [helpOpen, setHelpOpen] = useState(false);
  const [closeOpen, setCloseOpen] = useState(false);
  const projectDot = useProjectDot();
  const authoring = useAuthoring();
  const [templateId, setTemplateId] = useState("blank");
  const [templateName, setTemplateName] = useState("draft-skill");
  const lobsterRows = lobsterAgents(
    (typeof localStorage !== "undefined" ? localStorage.getItem("skill-hive-home") ?? "" : "").split(",").filter(Boolean),
  );
  const showUpdate = typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-app-update") === "1";
  const [restartNow, setRestartNow] = useState(
    () => typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-restart-now") === "1",
  );
  useEffect(() => {
    const onRestart = () => setRestartNow(localStorage.getItem("skill-hive-restart-now") === "1");
    window.addEventListener("skill-hive-restart", onRestart);
    return () => window.removeEventListener("skill-hive-restart", onRestart);
  }, []);

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
      } else       if (event.key === "/") {
        event.preventDefault();
        setShortcutsOpen(true);
      } else if (key === "n" && event.shiftKey) {
        event.preventDefault();
        openPicker("agent");
      } else if (key === "n") {
        event.preventDefault();
        openPicker("skill");
      } else if (key === "g" && event.shiftKey) {
        event.preventDefault();
        navigate("/generate/agent");
      } else if (key === "g") {
        event.preventDefault();
        navigate("/generate/skill");
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
    const actions = [
      ...ACTIONS,
      { id: "new-skill", label: t("palette.newSkill"), href: "action:skill" },
      { id: "new-agent", label: t("palette.newAgent"), href: "action:agent" },
      { id: "generate-skill", label: t("palette.generateSkill"), href: "action:generate-skill" },
      { id: "generate-agent", label: t("palette.generateAgent"), href: "action:generate-agent" },
    ];
    const actionHits = actions.filter((action) => !needle || action.label.toLowerCase().includes(needle)).map((action) => ({
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
            {(agents.data ?? []).filter((agent) => agent.installed && agent.enabled).map((agent) => (
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
                  const next = moveById(shownPresets, dragPreset, preset.id, (item) => item.id);
                  setPresetRows(next);
                  void reorderPresets(next.map((item) => item.id));
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
                    <button type="button" className="text-xs" aria-label={`${t("preset.delete")} ${preset.name}`} onClick={() => { setPresetRows(shownPresets.filter((item) => item.id !== preset.id)); void deletePreset(preset.id); }}>{t("preset.delete")}</button>
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
                  void updatePreset(editingPreset, presetName);
                  setEditingPreset(null);
                }}
              >
                <input aria-label={t("preset.edit")} value={presetName} onChange={(event) => setPresetName(event.target.value)} className="w-full rounded-md border border-input px-1 py-0.5 text-xs" />
                <button type="submit" className="text-xs">{t("preset.save")}</button>
              </form>
            ) : null}
            <button
              type="button"
              className="flex h-8 items-center rounded-md px-2 text-sm hover:bg-accent"
              onClick={() => {
                const name = presetName.trim() || "Preset";
                void createPreset(name).then((id) => {
                  setPresetRows([...shownPresets, { id, name, skillIds: [], synced: 0 }]);
                });
              }}
            >
              {collapsed ? "+" : t("preset.new")}
            </button>
            <div className="mt-3 px-2 text-xs text-muted-foreground">{t("sidebar.projects")}</div>
            <NavLink to="/project/fixture" className="flex h-8 items-center gap-2 rounded-md px-2 text-sm hover:bg-accent">
              <span data-testid="sidebar-project-dot" data-color={projectDot} className="inline-block h-2 w-2 rounded-full bg-current" />
              {collapsed ? "P" : t("project.title")}
            </NavLink>
            {lobsterRows.length === 0 ? null : (
              <>
                <div className="mt-3 px-2 text-xs text-muted-foreground">{t("lobster.title")}</div>
                <NavLink to="/lobster-workspace" className="flex h-8 items-center rounded-md px-2 text-sm hover:bg-accent">
                  {collapsed ? "L" : t("lobster.all")}
                </NavLink>
                {lobsterRows.map((agent) => (
                  <NavLink key={agent.key} to={`/lobster-workspace/${agent.key}`} className="flex h-8 items-center rounded-md px-2 text-sm hover:bg-accent">
                    {collapsed ? agent.name.slice(0, 1) : agent.name}
                  </NavLink>
                ))}
              </>
            )}
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
            <div data-testid="toolbar" className="flex items-center gap-2 text-xs text-muted-foreground">
              <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setHelpOpen(true)}>{t("help.title")}</button>
              <button type="button" data-testid="request-close" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setCloseOpen(true)}>{t("close.dialog")}</button>
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
                agents: ((agents.data ?? []) as AgentRow[]).filter((agent) => agent.installed && agent.enabled),
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
        <span>{authoring.counts ? t("status.validation", { errors: String(authoring.counts.errors), warnings: String(authoring.counts.warnings) }) : "Ready"}</span>
        <Link to="/runs/run-fixture" data-testid="run-chip" className="rounded px-1 hover:bg-accent">{t("run.chip")}</Link>
        <span>Skill Hive 1.0.0</span>
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
                  runAction(matches[0].href, navigate);
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
                        runAction(action.href, navigate);
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
      {authoring.picker ? (
        <div role="dialog" aria-label={t("editor.template.title")} className="fixed inset-0 z-[70] flex items-center justify-center bg-[rgb(15_23_42/0.4)]">
          <form
            className="w-80 rounded-lg border border-border bg-popover p-4"
            onSubmit={(event) => {
              event.preventDefault();
              if (authoring.picker === "skill") {
                const draft = createSkill(templateId, templateName);
                navigate(`/skills/${draft.id}/edit`);
              } else {
                closePicker();
                navigate("/agents/reviewer/edit");
              }
            }}
          >
            <h2 className="font-medium">{t("editor.template.title")}</h2>
            <div className="mt-2 flex flex-col gap-1">
              {SKILL_TEMPLATES.map((item) => (
                <button key={item.id} type="button" aria-pressed={templateId === item.id} className="rounded-md border border-border px-2 py-1 text-left text-sm" onClick={() => setTemplateId(item.id)}>
                  {t(item.labelKey)}
                </button>
              ))}
            </div>
            <input aria-label={t("editor.template.name")} value={templateName} onChange={(event) => setTemplateName(event.target.value)} className="mt-2 w-full rounded-md border border-input px-2 py-1 text-sm" />
            <button type="submit" className="mt-2 rounded-md bg-primary px-2 py-1 text-sm text-primary-foreground">{t("editor.template.create")}</button>
            <button type="button" className="mt-2 ml-2 rounded-md border border-border px-2 py-1 text-sm" onClick={() => closePicker()}>{t("agents.cancel")}</button>
          </form>
        </div>
      ) : null}
      {showUpdate || restartNow ? (
        <div role="status" data-testid="update-toast" className="fixed bottom-8 right-3 z-50 rounded-md border border-border bg-popover px-3 py-2 text-sm">
          {restartNow ? (
            <button type="button" onClick={() => setRestartNow(true)}>{t("settings.about.restart")}</button>
          ) : (
            t("startup.toast")
          )}
        </div>
      ) : null}
      {helpOpen ? <HelpDialog onClose={() => setHelpOpen(false)} /> : null}
      {closeOpen ? (
        <CloseGuardDialog
          onRemember={(choice) => {
            localStorage.setItem("skill-hive-close", choice);
            void setSettings("closeBehaviour", choice);
          }}
          onClose={() => setCloseOpen(false)}
        />
      ) : null}
      <Toaster />
    </div>
  );
}

function runAction(href: string, navigate: (path: string) => void): void {
  if (href === "action:skill") openPicker("skill");
  else if (href === "action:agent") openPicker("agent");
  else if (href === "action:generate-skill") navigate("/generate/skill");
  else if (href === "action:generate-agent") navigate("/generate/agent");
  else navigate(href);
}

export interface ShellOutletContext {
  listing: string[];
  skills: import("@/features/library/model").SkillCard[];
  presets: PresetRow[];
  setPresets: (next: PresetRow[]) => void;
  agents: AgentRow[];
  theme: ThemeChoice;
  setTheme: (choice: ThemeChoice) => void;
}
