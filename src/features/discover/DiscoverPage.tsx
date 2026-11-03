import { useSearchParams } from "react-router-dom";
import { Globe, HardDrive, Sparkles } from "lucide-react";
import { LineTabsList, LineTabsTrigger, Tabs, TabsContent } from "@/components/ui";
import { Page, PageBody, PageHeader } from "@/components/app/common";
import { LocalTab } from "./LocalTab";
import { OnlineTab } from "./OnlineTab";
import { CatalogTab } from "./CatalogTab";

type Tab = "local" | "online" | "catalog";

const TABS: { value: Tab; label: string; icon: typeof Globe }[] = [
  { value: "local", label: "On this machine", icon: HardDrive },
  { value: "online", label: "Online", icon: Globe },
  { value: "catalog", label: "Catalog", icon: Sparkles },
];

/**
 * Discover: collect skills from the machine, a remote source or the catalog.
 * The active tab, the source being previewed and the auto-scan flag live in the URL.
 */
export function DiscoverPage() {
  const [params, setParams] = useSearchParams();
  const tabParam = params.get("tab");
  const tab: Tab = tabParam === "online" || tabParam === "catalog" ? tabParam : "local";
  const source = params.get("source");
  const scanParam = params.get("scan");

  const update = (fn: (p: URLSearchParams) => void) =>
    setParams((prev) => {
      const next = new URLSearchParams(prev);
      fn(next);
      return next;
    });
  const setTab = (t: string) => update((p) => p.set("tab", t));
  const previewSource = (s: string) =>
    update((p) => {
      p.set("tab", "online");
      p.set("source", s);
    });

  return (
    <Page>
      <PageHeader title="Discover" description="Collect skills, subagents and slash commands from this machine, a repository or the curated catalog. Nothing changes until you import." />
      <Tabs value={tab} onValueChange={setTab} className="flex min-h-0 flex-1 flex-col">
        <LineTabsList className="mx-6">
          {TABS.map((t) => (
            <LineTabsTrigger key={t.value} value={t.value}>
              <t.icon /> {t.label}
            </LineTabsTrigger>
          ))}
        </LineTabsList>
        <PageBody className="pt-4">
          <TabsContent value="local" forceMount className="mt-0 data-[state=inactive]:hidden">
            <LocalTab key={scanParam ?? ""} autoScan={scanParam === "1"} />
          </TabsContent>
          <TabsContent value="online" forceMount className="mt-0 data-[state=inactive]:hidden">
            <OnlineTab source={source} onPreview={previewSource} />
          </TabsContent>
          <TabsContent value="catalog" forceMount className="mt-0 data-[state=inactive]:hidden">
            <CatalogTab onPreview={previewSource} />
          </TabsContent>
        </PageBody>
      </Tabs>
    </Page>
  );
}
