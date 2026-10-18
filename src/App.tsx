import { QueryClient, QueryClientProvider, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Navigate, Route, Routes, useOutletContext, useParams, useSearchParams } from "react-router-dom";
import { TooltipProvider } from "@/components/ui/tooltip";
import { APP_ROUTES } from "./app/routes";
import { AppShell, type ShellOutletContext } from "./components/shell/AppShell";
import { bindFileWatcher } from "./lib/events/files";
import { listAgents, listScanned, type AgentRow } from "@/lib/ipc/catalog";
import {
  addCustomTool,
  addSkillToPreset,
  applyPreset,
  cancelInstall,
  confirmGitInstall,
  createPreset,
  deleteManagedSkills,
  deleteTag,
  importAllDiscovered,
  importExistingSkill,
  installArchiveSkill,
  installLocalSkill,
  previewGitInstall,
  removeSkillFromPreset,
  renameTag,
  reorderPresetSkills,
  resetCustomToolPath,
  setCentralRepoPath,
  setSkillTags,
  setSkillsEnabled,
  syncSkillToTool,
  unsyncSkillFromTool,
} from "@/lib/ipc/actions";
import type { SkillCard } from "@/features/library/model";
import type { WorkspaceCard } from "./features/workspace/WorkspaceView";
import { PlaceholderView, TokensView } from "./views/Screens";
import { DashboardView } from "./features/dashboard/DashboardView";
import { InstallView } from "./features/install/InstallView";
import { LibraryView } from "./features/library/LibraryView";
import { SettingsView } from "./features/settings/SettingsView";
import { WorkspaceView } from "./features/workspace/WorkspaceView";
import { centerChangedNames, draftSkillCards, useAuthoring } from "@/lib/authoring/store";
import { BackupView } from "./features/backup/BackupView";
import { ProjectView } from "./features/project/ProjectView";
import { EndpointsView } from "./features/endpoints/EndpointsView";
import { ResultsView } from "./features/results/ResultsView";
import { SuiteView } from "./features/suites/SuiteView";
import { CompareView } from "./features/compare/CompareView";
import { GenerateWizard } from "./features/generate/GenerateWizard";
import { HistoryView } from "./features/history/HistoryView";
import { AgentEditor } from "./features/agents/AgentEditor";
import { AgentsView } from "./features/agents/AgentsView";
import { SkillEditor } from "./features/skills/SkillEditor";

function DashboardPage() {
  const { skills } = useOutletContext<ShellOutletContext>();
  return <DashboardView skills={skills} />;
}

function LibraryPage() {
  const client = useQueryClient();
  const { skills, presets, setPresets, activePresetId } = useOutletContext<ShellOutletContext>();
  useAuthoring();
  const drafts = draftSkillCards();
  const [params] = useSearchParams();
  const [rows, setRows] = useState(skills);
  useEffect(() => {
    setRows(skills);
  }, [skills]);
  const preset = presets.find((item) => item.id === activePresetId) ?? presets.find((item) => item.id === "research") ?? presets[0];
  const memberIds = preset?.skillIds ?? [];
  const refresh = () => client.invalidateQueries({ queryKey: ["skills"] });
  return (
    <LibraryView
      skills={[...drafts.filter((draft) => !rows.some((skill) => skill.id === draft.id)), ...rows.map((skill) => (drafts.some((draft) => draft.id === skill.id) ? { ...skill, draft: true } : skill))]}
      initialId={params.get("skill")}
      memberIds={memberIds}
      onAddTag={(skillId, tag) => {
        const next = [...(rows.find((skill) => skill.id === skillId)?.tags ?? []), tag];
        setRows((current) => current.map((skill) => (skill.id === skillId ? { ...skill, tags: next } : skill)));
        return setSkillTags(skillId, next).then(refresh);
      }}
      onRenameTag={(from, to) => {
        setRows((current) => current.map((skill) => ({ ...skill, tags: skill.tags.map((tag) => (tag === from ? to : tag)) })));
        return renameTag(from, to).then(refresh);
      }}
      onDeleteTag={(tag) => {
        setRows((current) => current.map((skill) => ({ ...skill, tags: skill.tags.filter((item) => item !== tag) })));
        return deleteTag(tag).then(refresh);
      }}
      onRemoveTag={(skillId, tag) => {
        const next = (rows.find((skill) => skill.id === skillId)?.tags ?? []).filter((item) => item !== tag);
        setRows((current) => current.map((skill) => (skill.id === skillId ? { ...skill, tags: next } : skill)));
        return setSkillTags(skillId, next).then(refresh);
      }}
      onSync={(ids, agents) => {
        setRows((current) => current.map((skill) => (ids.includes(skill.id) ? { ...skill, agents } : skill)));
        return Promise.all(ids.flatMap((id) => agents.map((agent) => syncSkillToTool(id, agent)))).then(refresh);
      }}
      onToggleMember={(skillId) => {
        if (!preset) return;
        const has = preset.skillIds.includes(skillId);
        const skillIds = has ? preset.skillIds.filter((id) => id !== skillId) : [...preset.skillIds, skillId];
        setPresets(presets.map((item) => (item.id === preset.id ? { ...item, skillIds } : item)));
        return (has ? removeSkillFromPreset(preset.id, skillId) : addSkillToPreset(preset.id, skillId)).then(() =>
          client.invalidateQueries({ queryKey: ["presets"] }),
        );
      }}
      onReorderMembers={(ids) => {
        if (!preset) return;
        setPresets(presets.map((item) => (item.id === preset.id ? { ...item, skillIds: ids } : item)));
        return reorderPresetSkills(preset.id, ids).then(() => undefined);
      }}
      onDeleteSkills={(ids) => {
        setRows((current) => current.filter((skill) => !ids.includes(skill.id)));
        return deleteManagedSkills(ids).then(refresh);
      }}
      onSetEnabled={(ids, enabled) => {
        setRows((current) => current.map((skill) => (ids.includes(skill.id) ? { ...skill, enabled } : skill)));
        return setSkillsEnabled(ids, enabled).then(refresh);
      }}
      onBatchTag={(ids, tag) => {
        setRows((current) => current.map((skill) => (ids.includes(skill.id) ? { ...skill, tags: [...skill.tags, tag] } : skill)));
        return Promise.all(ids.map((id) => setSkillTags(id, [...(rows.find((skill) => skill.id === id)?.tags ?? []), tag]))).then(refresh);
      }}
      onToggleAgent={(skillId, agent) => {
        const current = rows.find((skill) => skill.id === skillId);
        const present = current?.agents.includes(agent) ?? false;
        setRows((list) =>
          list.map((skill) => {
            if (skill.id !== skillId) return skill;
            const agents = present ? skill.agents.filter((item) => item !== agent) : [...skill.agents, agent];
            return { ...skill, agents };
          }),
        );
        return (present ? unsyncSkillFromTool(skillId, agent) : syncSkillToTool(skillId, agent)).then(refresh);
      }}
    />
  );
}

function InstallPage() {
  const client = useQueryClient();
  const scanned = useQuery({ queryKey: ["scan"], queryFn: listScanned });
  const refresh = () => Promise.all([client.invalidateQueries({ queryKey: ["skills"] }), client.invalidateQueries({ queryKey: ["scan"] })]).then(() => undefined);
  return (
    <InstallView
      scanned={scanned.data ?? []}
      onInstallLocal={(path) => installLocalSkill(path).then(refresh)}
      onInstallArchive={(path) => installArchiveSkill(path).then(refresh)}
      onImportAll={() => importAllDiscovered().then(refresh)}
      onImportOne={(row, name) => importExistingSkill(row.path, name).then(refresh)}
      onPreview={(url) => previewGitInstall(url)}
      onImportGit={(_url, rename, previewId, relative) => confirmGitInstall(previewId, relative, rename).then(refresh)}
      onCancel={(previewId) => cancelInstall(previewId).then(() => undefined)}
    />
  );
}

function workspaceCards(skills: SkillCard[], agentKey: string): WorkspaceCard[] {
  const changed = new Set(centerChangedNames());
  const merged = [...skills];
  for (const draft of draftSkillCards()) {
    if (!merged.some((skill) => skill.id === draft.id)) merged.push(draft);
  }
  return merged.map((skill) => ({
    id: skill.id,
    name: skill.name,
    status: changed.has(skill.name) || skill.name.includes("diff") ? "center_changed" : skill.agents.includes(agentKey) ? "in_sync" : "local_only",
    tags: skill.tags,
    body: skill.body,
    center: skill.sourceBody,
    managed: skill.agents.includes(agentKey),
  }));
}

function WorkspacePage() {
  const { presets, skills, agents } = useOutletContext<ShellOutletContext>();
  useAuthoring();
  const { agentKey } = useParams();
  if (agentKey === "openclaw") return <Navigate to="/lobster-workspace/openclaw" replace />;
  return (
    <WorkspaceView
      presets={presets}
      agents={agents}
      library={skills}
      agentKey={agentKey}
      cards={agentKey ? workspaceCards(skills, agentKey) : []}
      onApply={(id) => applyPreset(id, agentKey).then(() => undefined)}
      onCreate={(name) => createPreset(name).then(() => undefined)}
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
      onAddAgent={async (name, path) => {
        const added = await addCustomTool(name, path);
        setExtra((current) => [...current.filter((agent) => agent.key !== added.key), added]);
      }}
      onRepoPath={(path) => setCentralRepoPath(path).then(() => undefined)}
      onResetPath={(key) => resetCustomToolPath(key)}
      onAgents={(next: AgentRow[]) => {
        setExtra(next);
      }}
    />
  );
}

function screenFor(path: string) {
  if (path === "/") return <DashboardPage />;
  if (path === "/settings") return <SettingsPage />;
  if (path === "/my-skills") return <LibraryPage />;
  if (path === "/install") return <InstallPage />;
  if (path === "/global-workspace" || path === "/global-workspace/:agentKey") return <WorkspacePage />;
  if (path === "/backup") return <BackupView />;
  if (path === "/project/:id") return <ProjectView />;
  if (path === "/lobster-workspace" || path === "/lobster-workspace/:agentKey") return <LobsterPage />;
  if (path === "/agents") return <AgentsView />;
  if (path === "/agents/:id/edit") return <AgentEditor />;
  if (path === "/skills/:id/edit") return <SkillEditor />;
  if (path === "/settings/endpoints" || path === "/providers") return <EndpointsView />;
  if (path === "/skills/:id/tests" || path === "/testing" || path === "/testing/suites/:id") return <SuiteView />;
  if (path === "/runs/:runId" || path === "/testing/runs/:id") return <ResultsView />;
  if (path === "/generate/:sessionId") return <GenerateWizard />;
  if (path === "/compare/:comparisonId") return <CompareView />;
  if (path === "/skills/:id/history" || path === "/agents/:id/history") return <HistoryView />;
  if (path === "/dev/tokens") return <TokensView />;
  return <PlaceholderView />;
}

function LobsterPage() {
  const { agentKey } = useParams();
  const { presets, skills, agents } = useOutletContext<ShellOutletContext>();
  useAuthoring();
  return (
    <div data-testid="lobster">
      <WorkspaceView
        presets={presets}
        agents={agents}
        library={skills}
        agentKey={agentKey}
        cards={agentKey ? workspaceCards(skills, agentKey) : []}
        onApply={(id) => applyPreset(id, agentKey).then(() => undefined)}
        onCreate={(name) => createPreset(name).then(() => undefined)}
      />
    </div>
  );
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
