import { Page, PageBody, PageHeader } from "@/components/app/common";
import { EmptyState } from "@/components/ui";

export function AgentsPage() {
  return (
    <Page>
      <PageHeader title="Agents" description="What each agent tool has installed." />
      <PageBody>
        <EmptyState title="Agents" description="This screen is being built." />
      </PageBody>
    </Page>
  );
}
