import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ArchiveIcon,
  BookmarkIcon,
  BotIcon,
  CircleHelpIcon,
  DownloadIcon,
  ClipboardListIcon,
  FlaskConicalIcon,
  FolderIcon,
  GitCompareIcon,
  LayoutGridIcon,
  ListChecksIcon,
  GripVerticalIcon,
  HexagonIcon,
  HistoryIcon,
  LayoutDashboardIcon,
  LibraryIcon,
  PencilIcon,
  PlusIcon,
  SearchIcon,
  SettingsIcon,
  Trash2Icon,
  XIcon,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Link, NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Toaster } from "@/components/ui/sonner";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
} from "@/components/ui/sidebar";
import { NAV_ITEMS } from "../../app/routes";
import { BASE_LISTING_KEY, SKILLS_KEY } from "../../lib/events/files";
import { listBaseEntries } from "../../lib/ipc/settings";
import { applyTheme, isThemeChoice, type ThemeChoice } from "../../lib/theme";
import { getSettings, setSettings } from "../../lib/ipc/settings";
import { applyTextSize, isTextSize, setLocale, t } from "../../lib/i18n";
import { demoEnabled } from "@/lib/demo";
import { jobLabel } from "@/lib/job-label";
import { DataPhaseContext, RetryContext } from "@/lib/view-state";
import { DevCrash, RouteErrorBoundary } from "@/components/RouteError";
import { listAgents, listPresets, listSkills, type AgentRow, type PresetRow } from "@/lib/ipc/catalog";
import { readLibraryIndex } from "@/lib/library-cache";
import { createPreset, deletePreset, rebuildTrayMenu, reorderPresets, setToolOrder, silent, switchPreset, updatePreset } from "@/lib/ipc/actions";
import { writeSetting } from "@/lib/ipc/catalog";
import { moveById } from "@/features/library/model";
import { lobsterAgents } from "@/features/lobster/model";
import { useProjectDot } from "@/features/project/store";
import { SKILL_TEMPLATES } from "@/lib/authoring/model";
import { closePicker, createSkill, openPicker, useAuthoring } from "@/lib/authoring/store";
import { CloseGuardDialog } from "./CloseGuardDialog";
import { HelpDialog } from "./HelpDialog";
import { Kbd } from "../Kbd";
import { cn } from "cn";
import { AgentMark } from "@/components/app/AgentBadge";
import { StatusBanner } from "../StatusBanner";
import { shellBand, useShellBand } from "@/lib/shell-width";

const NAV_ICONS: Record<string, typeof LayoutDashboardIcon> = {
  "/": LayoutDashboardIcon,
  "/my-skills": LibraryIcon,
  "/agents": BotIcon,
  "/install": DownloadIcon,
  "/backup": ArchiveIcon,
  "/settings": SettingsIcon,
};

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
  const band = useShellBand();
  const [pinned, setPinned] = useState(
    () => typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-sidebar-pin") === "1",
  );
  const [collapsed, setCollapsed] = useState(() => {
    const width = typeof window === "undefined" ? 1440 : window.innerWidth;
    const next = shellBand(width);
    if (next >= 1440) return false;
    if (next === 1100) return localStorage.getItem("skill-hive-sidebar-pin") !== "1";
    return true;
  });
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
  const [activePresetId, setActivePresetId] = useState<string | null>(null);
  const [paletteIndex, setPaletteIndex] = useState(0);
  const [projectRows, setProjectRows] = useState<{ id: string; name: string; path: string }[]>([]);
  const [skillUpdates, setSkillUpdates] = useState(0);
  const [agentRows, setAgentRows] = useState<AgentRow[] | null>(null);
  const [helpOpen, setHelpOpen] = useState(false);
  const [closeOpen, setCloseOpen] = useState(false);
  const projectDot = useProjectDot();
  const authoring = useAuthoring();
  const [templateId, setTemplateId] = useState("blank");
  const [templateName, setTemplateName] = useState("draft-skill");
  const lobsterRows = lobsterAgents(
    (typeof localStorage !== "undefined" ? localStorage.getItem("skill-hive-home") ?? "" : "").split(",").filter(Boolean),
  );
  const storedUpdate = typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-app-update") === "1";
  const [showUpdate, setShowUpdate] = useState(storedUpdate);
  const queryClient = useQueryClient();
  const [restartNow, setRestartNow] = useState(
    () => typeof localStorage !== "undefined" && localStorage.getItem("skill-hive-restart-now") === "1",
  );
  useEffect(() => {
    void silent<string>("check_app_update").then((value) => {
      if (typeof value === "string" && value.startsWith("available")) setShowUpdate(true);
    });
    void silent<number>("check_all_skill_updates").then((count) => {
      if (typeof count === "number" && count > 0) setSkillUpdates(count);
    });
    void silent<string | null>("get_settings", { key: "skill_update_count" }).then((value) => {
      const count = Number(value);
      if (Number.isFinite(count) && count > 0) setSkillUpdates(count);
    });
    void silent<string | null>("get_active_preset").then((id) => {
      if (typeof id === "string" && id) setActivePresetId(id);
    });
    void silent<{ id: string; name: string; path: string }[]>("get_projects").then((rows) => {
      if (Array.isArray(rows)) setProjectRows(rows);
    });
  }, [reloadKey]);
  useEffect(() => {
    const onRestart = () => setRestartNow(localStorage.getItem("skill-hive-restart-now") === "1");
    window.addEventListener("skill-hive-restart", onRestart);
    return () => window.removeEventListener("skill-hive-restart", onRestart);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.shell = String(band);
    if (band >= 1440) setCollapsed(false);
    else if (band === 1100) setCollapsed(!pinned);
    else setCollapsed(true);
  }, [band, pinned]);

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
    getSettings("ui_text_size")
      .then((value) => {
        if (!cancelled && isTextSize(value)) applyTextSize(value);
      })
      .catch(() => {
        /* text size is optional */
      });
    getSettings("ui_language")
      .then(() => {
        if (!cancelled) setLocale("en");
      })
      .catch(() => {
        /* language is optional */
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
      } else if (key === "r") {
        event.preventDefault();
        setReloadKey((value) => value + 1);
        void queryClient.invalidateQueries();
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
  }, [navigate, queryClient, shortcutRows]);

  const skills = useQuery({
    queryKey: SKILLS_KEY,
    queryFn: listSkills,
    staleTime: 5 * 60_000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    placeholderData: () => (demoEnabled() ? undefined : readLibraryIndex() ?? undefined),
  });
  const presets = useQuery({ queryKey: ["presets"], queryFn: listPresets });
  const agents = useQuery({ queryKey: ["agents"], queryFn: listAgents });
  const dataPhase = !demoEnabled() && skills.isPending && skills.data === undefined ? "loading" : skills.isError && skills.data === undefined ? "error" : "ready";
  const retryData = () => {
    void skills.refetch();
    void presets.refetch();
    void agents.refetch();
  };
  const [job, setJob] = useState<{ job: string; current: number; total: number; message: string; done: boolean } | null>(null);
  useEffect(() => {
    const preview = new URLSearchParams(location.search).get("job");
    if (preview && import.meta.env.DEV) {
      setJob({ job: preview, current: 2, total: 5, message: preview, done: false });
    }
    let unlisten: (() => void) | undefined;
    void import("@tauri-apps/api/event")
      .then(({ listen }) =>
        listen<{ job: string; current: number; total: number; message: string; done: boolean }>("job-progress", (event) => {
          setJob(event.payload.done ? null : event.payload);
        }),
      )
      .then((stop) => {
        unlisten = stop;
      })
      .catch(() => undefined);
    return () => {
      unlisten?.();
    };
  }, [location.search]);
  const shownPresets = useMemo(() => presetRows ?? presets.data ?? [], [presetRows, presets.data]);

  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const skillHits = (skills.data ?? [])
      .filter((skill) => !needle || skill.name.toLowerCase().includes(needle))
      .map((skill) => ({ id: skill.id, label: skill.name, href: `/my-skills?skill=${skill.id}`, group: "Skills" }));
    const presetHits = shownPresets
      .filter((preset) => !needle || preset.name.toLowerCase().includes(needle))
      .map((preset) => ({ id: preset.id, label: preset.name, href: "/global-workspace", group: "Presets" }));
    const projectHits = projectRows
      .filter((project) => !needle || project.name.toLowerCase().includes(needle))
      .map((project) => ({ id: project.id, label: project.name, href: `/project/${project.id}`, group: "Projects" }));
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
    return [...skillHits, ...presetHits, ...projectHits, ...actionHits];
  }, [query, skills.data, shownPresets, projectRows]);

  useEffect(() => {
    setPaletteIndex(0);
  }, [query]);

  const listing = useQuery({
    queryKey: BASE_LISTING_KEY,
    queryFn: () => listBaseEntries().catch(() => [] as string[]),
  });

  const crumb = location.pathname.startsWith("/global-workspace") || location.pathname.startsWith("/project") || location.pathname.startsWith("/lobster")
    ? t("shell.workspaces")
    : location.pathname.startsWith("/effectiveness") || location.pathname.startsWith("/runs") || location.pathname.startsWith("/compare") || location.pathname.includes("/tests") || location.pathname.includes("/history")
      ? t("shell.test")
      : t("shell.manage");
  const sidebarWidth = collapsed ? 56 : band >= 1920 ? 256 : 240;
  const pagePad = band >= 1920 ? "px-8 py-8" : band >= 1440 ? "px-6 py-6" : "px-4 py-4";
  const primaryNav = NAV_ITEMS.filter((item) => item.path !== "/settings");
  const grouped = ["Skills", "Presets", "Projects", "Actions"]
    .map((group) => ({ group, items: matches.filter((item) => item.group === group) }))
    .filter((group) => group.items.length > 0);

  return (
    <div className="flex h-full min-h-0 flex-col bg-background font-sans text-foreground">
      {platform === "mac" ? (
        <div data-testid="drag-strip" data-tauri-drag-region className="h-7 shrink-0 bg-sidebar" />
      ) : null}
      <StatusBanner message={banner} onRetry={() => setReloadKey((value) => value + 1)} />
      <SidebarProvider open={!collapsed} onOpenChange={(open) => setCollapsed(!open)} className="relative h-full min-h-0 flex-1">
        <Sidebar
          collapsible="none"
          data-testid="sidebar"
          data-collapsed={collapsed ? "true" : "false"}
          data-band={band}
          className={cn("h-full shrink-0 border-r border-sidebar-border bg-sidebar", collapsed && "[&_[data-sidebar=group-label]]:sr-only", band === 800 && !collapsed && "absolute inset-y-0 left-0 z-40 shadow-lg")}
          style={{ width: sidebarWidth }}
        >
          <SidebarHeader className="h-11 justify-center border-b border-sidebar-border px-3">
            <div className="flex items-center gap-2 font-sans text-sm font-semibold tracking-tight">
              <HexagonIcon className="size-4 shrink-0" aria-hidden="true" />
              {collapsed ? <span className="sr-only">Skill Hive</span> : <span>Skill Hive</span>}
              {band === 1100 && !collapsed ? (
                <Button
                  type="button"
                  size="icon-xs"
                  variant={pinned ? "secondary" : "ghost"}
                  className="ml-auto"
                  aria-pressed={pinned}
                  aria-label={t("shell.pin")}
                  onClick={() => {
                    const next = !pinned;
                    setPinned(next);
                    localStorage.setItem("skill-hive-sidebar-pin", next ? "1" : "0");
                  }}
                >
                  <GripVerticalIcon aria-hidden="true" />
                </Button>
              ) : null}
            </div>
          </SidebarHeader>
          <SidebarContent>
            <SidebarGroup>
              <SidebarMenu>
                {primaryNav.map((item) => {
                  const Icon = NAV_ICONS[item.path] ?? HexagonIcon;
                  return (
                    <SidebarMenuItem key={item.path}>
                      <SidebarMenuButton asChild isActive={location.pathname === item.path} tooltip={item.nav}>
                        <NavLink to={item.path} title={item.nav}>
                          <Icon aria-hidden="true" />
                          {collapsed ? <span className="sr-only">{item.nav}</span> : <span>{item.nav}</span>}
                          {item.path === "/my-skills" && skillUpdates > 0 ? <span data-testid="update-dot" className="ml-auto size-2 rounded-full bg-warning" title={t("sidebar.updates")} /> : null}
                        </NavLink>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  );
                })}
              </SidebarMenu>
            </SidebarGroup>
            <SidebarGroup>
              <SidebarGroupLabel>{t("workspace.title")}</SidebarGroupLabel>
              <SidebarMenu>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={location.pathname === "/global-workspace"}>
                    <NavLink to="/global-workspace">
                      <LayoutGridIcon aria-hidden="true" />
                      {collapsed ? <span className="sr-only">{t("workspace.allAgents")}</span> : t("workspace.allAgents")}
                    </NavLink>
                  </SidebarMenuButton>
                </SidebarMenuItem>
                {(agentRows ?? (agents.data ?? []).filter((agent) => agent.installed && agent.enabled)).map((agent) => (
                  <SidebarMenuItem key={agent.key} draggable data-sidebar-agent={agent.key} onDragStart={() => setDragPreset(agent.key)} onDragOver={(event) => event.preventDefault()} onDrop={() => {
                    const current = agentRows ?? (agents.data ?? []).filter((item) => item.installed && item.enabled);
                    if (!dragPreset) return;
                    const next = moveById(current, dragPreset, agent.key, (item) => item.key);
                    setAgentRows(next);
                    void writeSetting("agent_order", next.map((item) => item.key).join(","));
                    void setToolOrder(next.map((item) => item.key));
                    setDragPreset(null);
                  }}>
                    <SidebarMenuButton asChild isActive={location.pathname === `/global-workspace/${agent.key}`}>
                      <NavLink to={`/global-workspace/${agent.key}`}>
                        <AgentMark agent={agent.key} name={agent.name} />
                        <span className={collapsed ? "sr-only" : undefined}>{`${agent.name}\u200b`}</span>
                      </NavLink>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroup>
            <SidebarGroup>
              <SidebarGroupLabel>Presets</SidebarGroupLabel>
              <SidebarMenu>
                {shownPresets.map((preset) => (
                  <SidebarMenuItem key={preset.id}>
                    <div
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
                      className="group/preset relative flex items-center"
                    >
                      <SidebarMenuButton asChild className="flex-1">
                        <NavLink to="/global-workspace" onClick={() => { setActivePresetId(preset.id); void switchPreset(preset.id); }}>
                          {collapsed ? <BookmarkIcon aria-hidden="true" /> : null}
                          <span className={collapsed ? "sr-only" : undefined}>{preset.name}</span>
                          {collapsed || preset.skillIds.length === 0 ? null : <span aria-hidden="true" className="ml-auto text-xs text-muted-foreground tabular-nums group-hover/preset:invisible">{preset.skillIds.length}</span>}
                        </NavLink>
                      </SidebarMenuButton>
                      <div className="absolute right-1 flex items-center rounded-md bg-sidebar opacity-0 transition-opacity group-hover/preset:opacity-100">
                          <Button type="button" size="icon-xs" variant="ghost" title={t("library.reorder")} aria-label={`${t("library.reorder")} ${preset.name}`}>
                            <GripVerticalIcon aria-hidden="true" />
                          </Button>
                          <Button type="button" size="icon-xs" variant="ghost" title={t("preset.edit")} aria-label={`${t("preset.edit")} ${preset.name}`} onClick={() => { setEditingPreset(preset.id); setPresetName(preset.name); }}>
                            <PencilIcon aria-hidden="true" />
                            <span className="sr-only">{t("preset.edit")}</span>
                          </Button>
                          <Button type="button" size="icon-xs" variant="ghost" title={t("preset.delete")} aria-label={`${t("preset.delete")} ${preset.name}`} onClick={() => { setPresetRows(shownPresets.filter((item) => item.id !== preset.id)); void deletePreset(preset.id); }}>
                            <Trash2Icon aria-hidden="true" />
                            <span className="sr-only">{t("preset.delete")}</span>
                          </Button>
                        </div>
                    </div>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
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
                  <Input aria-label={t("preset.edit")} value={presetName} onChange={(event) => setPresetName(event.target.value)} className="h-8 text-xs" />
                  <Button type="submit" size="xs" variant="secondary">{t("preset.save")}</Button>
                </form>
              ) : null}
              <Button type="button" variant="ghost" size="sm" className="justify-start text-muted-foreground" onClick={() => { const name = presetName.trim() || "Preset"; void createPreset(name).then((id) => { setPresetRows([...shownPresets, { id, name, skillIds: [], synced: 0 }]); void rebuildTrayMenu(); }); }}>
                <PlusIcon aria-hidden="true" />
                {collapsed ? <span className="sr-only">{t("preset.new")}</span> : t("preset.new")}
              </Button>
            </SidebarGroup>
            <SidebarGroup>
              <SidebarGroupLabel>{t("sidebar.projects")}</SidebarGroupLabel>
              <SidebarMenu>
                {projectRows.length === 0 ? (
                  <SidebarMenuItem>
                    <SidebarMenuButton asChild>
                      <NavLink to="/project/fixture" className="flex items-center gap-2">
                        <span data-testid="sidebar-project-dot" data-color={projectDot} className="inline-block size-2 rounded-full bg-info" />
                        <FolderIcon aria-hidden="true" />
                        {collapsed ? <span className="sr-only">{t("project.title")}</span> : t("project.title")}
                      </NavLink>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ) : projectRows.map((project) => (
                  <SidebarMenuItem key={project.id}>
                    <SidebarMenuButton asChild>
                      <NavLink to={`/project/${project.id}`} className="flex items-center gap-2">
                        <span data-testid="sidebar-project-dot" data-color={projectDot} className="inline-block size-2 rounded-full bg-info" />
                        <FolderIcon aria-hidden="true" />
                        {collapsed ? <span className="sr-only">{project.name}</span> : project.name}
                      </NavLink>
                    </SidebarMenuButton>
                    <Button type="button" size="icon-xs" variant="ghost" aria-label={`${t("sidebar.removeProject")} ${project.name}`} onClick={() => { setProjectRows((current) => current.filter((item) => item.id !== project.id)); void silent("remove_project", { id: project.id }); }}>
                      <Trash2Icon aria-hidden="true" />
                    </Button>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroup>
            <SidebarGroup>
              <SidebarGroupLabel>{collapsed ? <span className="sr-only">{t("shell.test")}</span> : t("shell.test")}</SidebarGroupLabel>
              <SidebarMenu>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={location.pathname.startsWith("/effectiveness")}>
                    <NavLink to="/effectiveness">
                      <FlaskConicalIcon aria-hidden="true" />
                      {collapsed ? <span className="sr-only">{t("effect.title")}</span> : t("effect.title")}
                    </NavLink>
                  </SidebarMenuButton>
                </SidebarMenuItem>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={location.pathname.includes("/history")}>
                    <NavLink to="/skills/skill-research/history">
                      <HistoryIcon aria-hidden="true" />
                      {collapsed ? <span className="sr-only">{t("history.title")}</span> : t("history.title")}
                    </NavLink>
                  </SidebarMenuButton>
                </SidebarMenuItem>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={location.pathname.includes("/tests")}>
                    <NavLink to="/skills/skill-research/tests">
                      <ListChecksIcon aria-hidden="true" />
                      {collapsed ? <span className="sr-only">{t("suite.title")}</span> : t("suite.title")}
                    </NavLink>
                  </SidebarMenuButton>
                </SidebarMenuItem>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={location.pathname.startsWith("/runs")}>
                    <NavLink to="/runs/run-fixture">
                      <ClipboardListIcon aria-hidden="true" />
                      {collapsed ? <span className="sr-only">{t("results.title")}</span> : t("results.title")}
                    </NavLink>
                  </SidebarMenuButton>
                </SidebarMenuItem>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={location.pathname.startsWith("/compare")}>
                    <NavLink to="/compare/fixture">
                      <GitCompareIcon aria-hidden="true" />
                      {collapsed ? <span className="sr-only">{t("compare.title")}</span> : t("compare.title")}
                    </NavLink>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </SidebarMenu>
            </SidebarGroup>
            {lobsterRows.length === 0 ? null : (
              <SidebarGroup>
                <SidebarGroupLabel>{t("lobster.title")}</SidebarGroupLabel>
                <SidebarMenu>
                  <SidebarMenuItem>
                    <SidebarMenuButton asChild>
                      <NavLink to="/lobster-workspace">
                        <BotIcon aria-hidden="true" />
                        {collapsed ? <span className="sr-only">{t("lobster.all")}</span> : t("lobster.all")}
                      </NavLink>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  {lobsterRows.map((agent) => (
                    <SidebarMenuItem key={agent.key}>
                      <SidebarMenuButton asChild>
                        <NavLink to={`/lobster-workspace/${agent.key}`}>
                          <AgentMark agent={agent.key} name={agent.name} />
                          <span className={collapsed ? "sr-only" : undefined}>{agent.name}</span>
                        </NavLink>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  ))}
                </SidebarMenu>
              </SidebarGroup>
            )}
          </SidebarContent>
          <SidebarFooter className="border-t border-sidebar-border">
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton asChild isActive={location.pathname.startsWith("/settings")}>
                  <NavLink to="/settings">
                    <SettingsIcon aria-hidden="true" />
                    {collapsed ? <span className="sr-only">Settings</span> : <span>Settings</span>}
                    {showUpdate && !collapsed ? <span className="ml-auto size-2 rounded-full bg-warning" aria-hidden="true" /> : null}
                  </NavLink>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarFooter>
        </Sidebar>
        <div className="flex min-h-0 min-w-0 flex-1 flex-col bg-background">
          <header
            data-testid="header"
            className={cn("sticky top-0 z-10 flex h-11 items-center justify-between gap-3 border-b border-border bg-background", pagePad, "py-0")}
          >
            <div className="min-w-0">
              <div data-testid="breadcrumb" className="truncate text-overline font-semibold text-muted-foreground uppercase">{crumb}</div>
            </div>
            <div data-testid="toolbar" className="flex items-center gap-1">
              <Button type="button" size="sm" variant="outline" className="text-muted-foreground" onClick={() => setPaletteOpen(true)}>
                <SearchIcon aria-hidden="true" />
                {t("shell.search")}
                <Kbd>Mod+K</Kbd>
              </Button>
              <Button type="button" size="icon-sm" variant="ghost" aria-label={t("help.title")} onClick={() => setHelpOpen(true)}>
                <CircleHelpIcon aria-hidden="true" />
              </Button>
              <Button type="button" size="icon-sm" variant="ghost" aria-label={t("close.dialog")} data-testid="request-close" onClick={() => setCloseOpen(true)}>
                <XIcon aria-hidden="true" />
              </Button>
            </div>
          </header>
          <main
            data-testid="content"
            className={cn("min-h-0 flex-1 overflow-auto", pagePad)}
            style={{ scrollPaddingTop: "44px" }}
          >
            {job && !job.done ? (
              <JobBanner job={job} onCancel={() => { void silent("cancel_job"); setJob(null); }} />
            ) : null}
            <div className="mx-auto w-full max-w-[1600px]">
              <DataPhaseContext.Provider value={dataPhase}>
              <RetryContext.Provider value={retryData}>
              <RouteErrorBoundary route={location.pathname}>
              <DevCrash level="route">
              <Outlet
                context={{
                  listing: listing.data ?? [],
                  skills: skills.data ?? [],
                  dataPhase,
                  retryData,
                  presets: shownPresets,
                  activePresetId,
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
              </DevCrash>
              </RouteErrorBoundary>
              </RetryContext.Provider>
              </DataPhaseContext.Provider>
            </div>
          </main>
        </div>
      </SidebarProvider>
      <footer
        role="status"
        data-testid="status-bar"
        className="flex h-6 shrink-0 items-center justify-between gap-3 border-t border-border bg-sidebar px-3 text-xs text-muted-foreground"
      >
        <span className="flex items-center gap-2">
          <span className="size-1.5 rounded-full bg-success" aria-hidden="true" />
          {job && !job.done ? t("job.working") : authoring.counts ? t("status.validation", { errors: String(authoring.counts.errors), warnings: String(authoring.counts.warnings) }) : t("shell.ready")}
        </span>
        <Link to="/runs/run-fixture" data-testid="run-chip" aria-label={t("results.title")} className="inline-flex text-muted-foreground">
          <ClipboardListIcon className="size-3.5" aria-hidden="true" />
        </Link>
        <span className="font-mono tabular-nums">Skill Hive 1.0.0</span>
      </footer>
      <Dialog open={paletteOpen} onOpenChange={setPaletteOpen}>
        <DialogContent data-testid="command-palette" aria-label="Command palette" className="top-[18%] translate-y-0 gap-0 overflow-hidden p-0 sm:max-w-lg" showCloseButton={false}>
          <DialogHeader className="sr-only">
            <DialogTitle>Command palette</DialogTitle>
            <DialogDescription>Jump to a skill, agent, or action.</DialogDescription>
          </DialogHeader>
          <Command shouldFilter={false} label="Command search">
            <CommandInput
              autoFocus
              aria-label="Command search"
              placeholder="Jump to skill, agent, run, action…"
              value={query}
              onValueChange={setQuery}
              onKeyDown={(event) => {
                if (event.key === "ArrowDown") {
                  event.preventDefault();
                  setPaletteIndex((index) => Math.min(index + 1, Math.max(matches.length - 1, 0)));
                  return;
                }
                if (event.key === "ArrowUp") {
                  event.preventDefault();
                  setPaletteIndex((index) => Math.max(index - 1, 0));
                  return;
                }
                if (event.key === "Enter" && matches[paletteIndex]) {
                  event.preventDefault();
                  setPaletteOpen(false);
                  runAction(matches[paletteIndex].href, navigate);
                }
              }}
            />
            <CommandList>
              {matches.length === 0 ? <CommandEmpty>{t("palette.empty", { query })}</CommandEmpty> : null}
              {grouped.map((group) => (
                <CommandGroup key={group.group} heading={group.group === "Projects" ? t("palette.group.projects") : group.group}>
                  {group.items.map((action) => {
                    const index = matches.findIndex((item) => item.group === action.group && item.id === action.id);
                    const active = index === paletteIndex;
                    return (
                      <CommandItem
                        key={`${action.group}-${action.id}`}
                        value={`${action.group}-${action.id}`}
                        data-active={active ? "true" : "false"}
                        data-testid={active ? "palette-active" : undefined}
                        onSelect={() => {
                          setPaletteOpen(false);
                          runAction(action.href, navigate);
                        }}
                      >
                        <span>{action.label}</span>
                        <Kbd>Enter</Kbd>
                      </CommandItem>
                    );
                  })}
                </CommandGroup>
              ))}
            </CommandList>
          </Command>
        </DialogContent>
      </Dialog>
      <Dialog open={shortcutsOpen} onOpenChange={setShortcutsOpen}>
        <DialogContent aria-label={t("shortcuts.title")} className="sm:max-w-md" showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>{t("shortcuts.title")}</DialogTitle>
            <DialogDescription>Remap the numbered jumps. Every shortcut keeps a modifier.</DialogDescription>
          </DialogHeader>
          <ul className="space-y-2 text-sm">
            {shortcutRows.map((item) => (
              <li key={item.id} className="flex items-center gap-2">
                <Kbd>{`Mod+${item.id}`}</Kbd>
                <Input aria-label={`Shortcut ${item.id}`} value={item.href} onChange={(event) => setShortcutRows((current) => current.map((row) => row.id === item.id ? { ...row, href: event.target.value } : row))} className="h-8" />
              </li>
            ))}
            <li className="text-muted-foreground">Mod+F</li>
            <li className="text-muted-foreground">Mod+/</li>
          </ul>
          <div className="flex justify-end gap-2">
            <Button type="button" size="sm" onClick={() => { void setSettings("shortcuts", JSON.stringify(shortcutRows)); }}>
              {t("shortcuts.save")}
            </Button>
            <Button type="button" size="sm" variant="outline" onClick={() => setShortcutsOpen(false)}>
              Close
            </Button>
          </div>
        </DialogContent>
      </Dialog>
      <Dialog open={Boolean(authoring.picker)} onOpenChange={(open) => { if (!open) closePicker(); }}>
        <DialogContent aria-label={t("editor.template.title")} className="sm:max-w-sm" showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>{t("editor.template.title")}</DialogTitle>
            <DialogDescription>Start from a template. The name becomes the directory.</DialogDescription>
          </DialogHeader>
          <form
            className="space-y-3"
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
            <div className="flex flex-col gap-1">
              {SKILL_TEMPLATES.map((item) => (
                <Button key={item.id} type="button" variant={templateId === item.id ? "default" : "outline"} size="sm" aria-pressed={templateId === item.id} className="justify-start" onClick={() => setTemplateId(item.id)}>
                  {t(item.labelKey)}
                </Button>
              ))}
            </div>
            <Input aria-label={t("editor.template.name")} value={templateName} onChange={(event) => setTemplateName(event.target.value)} className="h-8" />
            <div className="flex justify-end gap-2">
              <Button type="submit" size="sm">{t("editor.template.create")}</Button>
              <Button type="button" size="sm" variant="outline" onClick={() => closePicker()}>{t("agents.cancel")}</Button>
            </div>
          </form>
        </DialogContent>
      </Dialog>
      {showUpdate || restartNow ? (
        <div role="status" data-testid="update-toast" className="fixed right-3 bottom-8 z-[70] rounded-lg border border-border bg-popover px-3 py-2 text-sm shadow-lg">
          {restartNow ? (
            <Button type="button" size="sm" onClick={() => setRestartNow(true)}>{t("settings.about.restart")}</Button>
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
      <Toaster position="top-right" closeButton />
    </div>
  );
}

function JobBanner({
  job,
  onCancel,
}: {
  job: { job: string; current: number; total: number; message: string };
  onCancel: () => void;
}) {
  const { text, percent } = jobLabel(job.job, job.message, job.current, job.total);
  return (
    <div data-testid="job-progress" className="mb-4 rounded-lg border border-border bg-popover px-4 py-3 text-popover-foreground shadow-md">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-medium">{text}</p>
        <Button type="button" size="sm" variant="outline" onClick={onCancel}>
          {t("shell.cancel")}
        </Button>
      </div>
      <div
        className="mt-2 h-2 overflow-hidden rounded-full bg-muted"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        aria-valuetext={text}
        aria-label={text}
      >
        <div className="h-full rounded-full bg-primary" style={{ width: `${percent}%` }} />
      </div>
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
  dataPhase: "loading" | "ready" | "error";
  retryData: () => void;
  presets: PresetRow[];
  activePresetId: string | null;
  setPresets: (next: PresetRow[]) => void;
  agents: AgentRow[];
  theme: ThemeChoice;
  setTheme: (choice: ThemeChoice) => void;
}
