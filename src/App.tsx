import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Route, Routes, useOutletContext, useParams, useSearchParams } from "react-router-dom";
import { TooltipProvider } from "@/components/ui/tooltip";
import { APP_ROUTES } from "./app/routes";
import { AppShell, type ShellOutletContext } from "./components/shell/AppShell";
import { bindFileWatcher } from "./lib/events/files";
import { listAgents, listScanned, type AgentRow } from "./lib/phase1/client";
import type { SkillCard } from "./lib/phase1/model";
import type { WorkspaceCard } from "./views/phase1/WorkspaceView";
import { PlaceholderView, TokensView } from "./views/Screens";
import { DashboardView } from "./views/phase1/DashboardView";
import { InstallView } from "./views/phase1/InstallView";
import { LibraryView } from "./views/phase1/LibraryView";
import { SettingsView } from "./views/phase1/SettingsView";
import { WorkspaceView } from "./views/phase1/WorkspaceView";

function DashboardPage() {
  const { skills } = useOutletContext<ShellOutletContext>();
  return <DashboardView skills={skills} />;
}

function LibraryPage() {
  const { skills, presets, setPresets } = useOutletContext<ShellOutletContext>();
  const [params] = useSearchParams();
  const [rows, setRows] = useState(skills);
  useEffect(() => {
    setRows(skills);
  }, [skills]);
  const memberIds = presets.find((preset) => preset.id === "research")?.skillIds ?? presets[0]?.skillIds ?? [];
  return (
    <LibraryView
      skills={rows}
      initialId={params.get("skill")}
      memberIds={memberIds}
      onAddTag={(skillId, tag) =>
        setRows((current) =>
          current.map((skill) => (skill.id === skillId ? { ...skill, tags: [...skill.tags, tag] } : skill)),
        )
      }
      onRenameTag={(from, to) =>
        setRows((current) =>
          current.map((skill) => ({ ...skill, tags: skill.tags.map((tag) => (tag === from ? to : tag)) })),
        )
      }
      onDeleteTag={(tag) => setRows((current) => current.map((skill) => ({ ...skill, tags: skill.tags.filter((item) => item !== tag) })))}
      onRemoveTag={(skillId, tag) =>
        setRows((current) => current.map((skill) => (skill.id === skillId ? { ...skill, tags: skill.tags.filter((item) => item !== tag) } : skill)))
      }
      onSync={(ids, agents) =>
        setRows((current) => current.map((skill) => (ids.includes(skill.id) ? { ...skill, agents } : skill)))
      }
      onToggleMember={(skillId) =>
        setPresets(
          presets.map((preset) =>
            preset.id === (presets.find((item) => item.id === "research")?.id ?? presets[0]?.id)
              ? {
                  ...preset,
                  skillIds: preset.skillIds.includes(skillId)
                    ? preset.skillIds.filter((id) => id !== skillId)
                    : [...preset.skillIds, skillId],
                }
              : preset,
          ),
        )
      }
      onReorderMembers={(ids) => {
        const presetId = presets.find((preset) => preset.id === "research")?.id ?? presets[0]?.id;
        setPresets(presets.map((preset) => (preset.id === presetId ? { ...preset, skillIds: ids } : preset)));
      }}
      onDeleteSkills={(ids) => setRows((current) => current.filter((skill) => !ids.includes(skill.id)))}
      onSetEnabled={(ids, enabled) => setRows((current) => current.map((skill) => (ids.includes(skill.id) ? { ...skill, enabled } : skill)))}
      onBatchTag={(ids, tag) =>
        setRows((current) => current.map((skill) => (ids.includes(skill.id) ? { ...skill, tags: [...skill.tags, tag] } : skill)))
      }
      onToggleAgent={(skillId, agent) =>
        setRows((current) =>
          current.map((skill) => {
            if (skill.id !== skillId) return skill;
            const agents = skill.agents.includes(agent) ? skill.agents.filter((item) => item !== agent) : [...skill.agents, agent];
            return { ...skill, agents };
          }),
        )
      }
    />
  );
}

function InstallPage() {
  const scanned = useQuery({ queryKey: ["scan"], queryFn: listScanned });
  return <InstallView scanned={scanned.data ?? []} />;
}

function workspaceCards(skills: SkillCard[], agentKey: string): WorkspaceCard[] {
  return skills.map((skill) => ({
    id: skill.id,
    name: skill.name,
    status: skill.name.includes("diff") ? "center_changed" : skill.agents.includes(agentKey) ? "in_sync" : "local_only",
    tags: skill.tags,
    body: skill.body,
    center: skill.sourceBody,
    managed: skill.agents.includes(agentKey),
  }));
}

function WorkspacePage() {
  const { presets, skills, agents } = useOutletContext<ShellOutletContext>();
  const { agentKey } = useParams();
  return (
    <WorkspaceView
      presets={presets}
      agents={agents}
      library={skills}
      agentKey={agentKey}
      cards={agentKey ? workspaceCards(skills, agentKey) : []}
    />
  );
}

function SettingsPage() {
  const { theme, setTheme } = useOutletContext<ShellOutletContext>();
  const agents = useQuery({ queryKey: ["agents"], queryFn: listAgents });
  const [extra, setExtra] = useState(agents.data ?? []);
  useEffect(() => {
    setExtra(agents.data ?? []);
  }, [agents.data]);
  return (
    <SettingsView
      theme={theme}
      setTheme={setTheme}
      agents={extra}
      onAddAgent={(name, path) =>
        setExtra((current) => [
          ...current,
          {
            key: name.toLowerCase().replace(/\s+/g, "_"),
            name,
            installed: true,
            enabled: true,
            path,
            projectPath: "",
            custom: true,
          },
        ])
      }
      onAgents={(next: AgentRow[]) => setExtra(next)}
    />
  );
}

function screenFor(path: string) {
  if (path === "/") return <DashboardPage />;
  if (path === "/settings") return <SettingsPage />;
  if (path === "/my-skills") return <LibraryPage />;
  if (path === "/install") return <InstallPage />;
  if (path === "/global-workspace" || path === "/global-workspace/:agentKey") return <WorkspacePage />;
  if (path === "/dev/tokens") return <TokensView />;
  return <PlaceholderView />;
}

export function App({ client }: { client: QueryClient }) {
  const [platform, setPlatform] = useState("linux");

  useEffect(() => {
    const ua = navigator.userAgent;
    if (/Mac/.test(ua)) setPlatform("mac");
    else if (/Win/.test(ua)) setPlatform("windows");
    const unlisten = bindFileWatcher(client);
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [client]);

  return (
    <QueryClientProvider client={client}>
      <TooltipProvider>
        <Routes>
          <Route element={<AppShell platform={platform} />}>
            {APP_ROUTES.map((route) => (
              <Route key={route.path} path={route.path} element={screenFor(route.path)} />
            ))}
          </Route>
        </Routes>
      </TooltipProvider>
    </QueryClientProvider>
  );
}
