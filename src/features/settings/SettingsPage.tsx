import { useNavigate, useParams } from "react-router-dom";
import { Cpu, FolderGit2, Info, Monitor, SlidersHorizontal, type LucideIcon } from "lucide-react";
import { LineTabsList, LineTabsTrigger, Tabs, TabsContent } from "@/components/ui";
import { Page, PageBody, PageHeader } from "@/components/app/common";
import { GeneralTab } from "./GeneralTab";
import { ProvidersTab } from "./ProvidersTab";
import { AgentsTab } from "./AgentsTab";
import { ProjectsTab } from "./ProjectsTab";
import { AboutTab } from "./AboutTab";

type TabId = "general" | "providers" | "agents" | "projects" | "about";

const TABS: { value: TabId; label: string; icon: LucideIcon }[] = [
  { value: "general", label: "General", icon: SlidersHorizontal },
  { value: "providers", label: "Providers", icon: Cpu },
  { value: "agents", label: "Agents", icon: Monitor },
  { value: "projects", label: "Projects", icon: FolderGit2 },
  { value: "about", label: "About", icon: Info },
];

export function SettingsPage() {
  const { tab } = useParams<{ tab: string }>();
  const navigate = useNavigate();
  const current: TabId = TABS.some((t) => t.value === tab) ? (tab as TabId) : "general";

  return (
    <Page>
      <Tabs value={current} onValueChange={(v) => navigate(v === "general" ? "/settings" : `/settings/${v}`)} className="flex min-h-0 flex-1 flex-col">
        <PageHeader title="Settings" description="Providers, paths and appearance." className="pb-0">
          <LineTabsList>
            {TABS.map((t) => (
              <LineTabsTrigger key={t.value} value={t.value}>
                <t.icon /> {t.label}
              </LineTabsTrigger>
            ))}
          </LineTabsList>
        </PageHeader>
        <PageBody className="pt-4">
          <TabsContent value="general" className="mt-0">
            <GeneralTab />
          </TabsContent>
          <TabsContent value="providers" className="mt-0">
            <ProvidersTab />
          </TabsContent>
          <TabsContent value="agents" className="mt-0">
            <AgentsTab />
          </TabsContent>
          <TabsContent value="projects" className="mt-0">
            <ProjectsTab />
          </TabsContent>
          <TabsContent value="about" className="mt-0">
            <AboutTab />
          </TabsContent>
        </PageBody>
      </Tabs>
    </Page>
  );
}
