import { Page, PageBody, PageHeader } from "@/components/app/common";
import { EmptyState } from "@/components/ui";

export function LoadoutsPage() {
  return (
    <Page>
      <PageHeader title="Loadouts" description="Curated sets to deploy together." />
      <PageBody>
        <EmptyState title="Loadouts" description="This screen is being built." />
      </PageBody>
    </Page>
  );
}
