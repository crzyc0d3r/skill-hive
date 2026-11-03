import { Page, PageBody, PageHeader } from "@/components/app/common";
import { EmptyState } from "@/components/ui";

export function FindPage() {
  return (
    <Page>
      <PageHeader title="Find" description="Describe a job and get the right skills without conflicts." />
      <PageBody>
        <EmptyState title="Find" description="This screen is being built." />
      </PageBody>
    </Page>
  );
}
