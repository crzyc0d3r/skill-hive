import { Page, PageBody, PageHeader } from "@/components/app/common";
import { EmptyState } from "@/components/ui";

export function RunPage() {
  return (
    <Page>
      <PageHeader title="Run" description="Evaluation run detail." />
      <PageBody>
        <EmptyState title="Run" description="This screen is being built." />
      </PageBody>
    </Page>
  );
}
