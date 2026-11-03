import { Page, PageBody, PageHeader } from "@/components/app/common";
import { EmptyState } from "@/components/ui";

export function DiscoverPage() {
  return (
    <Page>
      <PageHeader title="Discover" description="Collect skills from this machine or online sources." />
      <PageBody>
        <EmptyState title="Discover" description="This screen is being built." />
      </PageBody>
    </Page>
  );
}
