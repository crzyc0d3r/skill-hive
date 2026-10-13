import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Route, Routes, useOutletContext, useSearchParams } from "react-router-dom";
import { TooltipProvider } from "@/components/ui/tooltip";
import { APP_ROUTES } from "./app/routes";
import { AppShell, type ShellOutletContext } from "./components/shell/AppShell";
import { bindFileWatcher } from "./lib/events/files";
import { listAgents, listScanned } from "./lib/phase1/client";
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
  const { skills } = useOutletContext<ShellOutletContext>();
  const [params] = useSearchParams();
  const [rows, setRows] = useState(skills);
  useEffect(() => {
    setRows(skills);
  }, [skills]);
  return (
    <LibraryView
      skills={rows}
      initialId={params.get("skill")}
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
      onSync={(ids, agents) =>
        setRows((current) => current.map((skill) => (ids.includes(skill.id) ? { ...skill, agents } : skill)))
      }
    />
  );
}

function InstallPage() {
  const scanned = useQuery({ queryKey: ["scan"], queryFn: listScanned });
  return <InstallView scanned={scanned.data ?? []} />;
}

function WorkspacePage() {
  const { presets } = useOutletContext<ShellOutletContext>();
  return (
    <WorkspaceView
      presets={presets}
      cards={[
        { name: "notes", status: "local_only" },
        { name: "research-notes", status: "in_sync" },
      ]}
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
