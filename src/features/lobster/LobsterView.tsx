import { t } from "@/lib/i18n";

export function LobsterView({ agentKey }: { agentKey?: string }) {
  return (
    <section data-testid="lobster">
      <h1 className="font-mono text-[22px] font-semibold">{t("lobster.title")}</h1>
      <p className="mt-2 text-sm">{agentKey ? agentKey : t("lobster.all")}</p>
    </section>
  );
}
