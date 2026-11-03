import { Page, PageBody, PageHeader } from "@/components/app/common";
import { EmptyState } from "@/components/ui";

export function SettingsPage() {
  return (
    <Page>
      <PageHeader title="Settings" description="Providers, paths and appearance." />
      <PageBody>
        <EmptyState title="Settings" description="This screen is being built." />
      </PageBody>
    </Page>
  );
}
