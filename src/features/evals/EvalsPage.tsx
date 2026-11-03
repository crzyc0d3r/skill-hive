import { Page, PageBody, PageHeader } from "@/components/app/common";
import { EmptyState } from "@/components/ui";

export function EvalsPage() {
  return (
    <Page>
      <PageHeader title="Evals" description="Measure whether a skill helps." />
      <PageBody>
        <EmptyState title="Evals" description="This screen is being built." />
      </PageBody>
    </Page>
  );
}
