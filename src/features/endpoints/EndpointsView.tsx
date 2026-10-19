import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import { t } from "../../lib/i18n";
import { exportEndpoints, setEndpointSecret, useEndpoints } from "@/features/endpoints/store";
import { probeEndpoint, silent } from "@/lib/ipc/actions";

const STAGES = ["credential", "proxy", "models", "completion", "tools", "cli", "persist"];

export function EndpointsView() {
  const endpoints = useEndpoints();
  const [draft, setDraft] = useState("");
  const [exported, setExported] = useState("");
  const [probed, setProbed] = useState("");
  const [prices, setPrices] = useState<{ model: string; input_per_mtok: number; output_per_mtok: number }[]>([]);
  useEffect(() => {
    void silent<string>("endpoint_prices").then((value) => {
      if (!value) return;
      try {
        const parsed = JSON.parse(value) as { model: string; input_per_mtok: number; output_per_mtok: number }[];
        if (Array.isArray(parsed) && parsed.length) setPrices(parsed);
      } catch {
        /* the catalog stays empty until the pricing table answers */
      }
    });
  }, []);
  return (
    <section data-testid="endpoints" className="min-w-0 space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("endpoints.title")}</h1>
      <ol className="flex flex-wrap gap-2 text-xs" data-testid="probe-stepper">
        {STAGES.map((stage, index) => (
          <li key={stage} className="rounded border border-border px-2 py-1">
            {index + 1}. {stage}
          </li>
        ))}
      </ol>
      <ul className="space-y-1 text-sm">
        <li>{t("endpoints.kind.anthropic")}</li>
        <li>{t("endpoints.kind.openai")}</li>
        <li>{t("endpoints.kind.claude")}</li>
      </ul>
      <ul className="space-y-1">
        {endpoints.map((row) => (
          <li key={row.id} className="flex items-center justify-between rounded border border-border px-2 py-1 text-sm">
            <span>{row.name}</span>
            <span>{row.hasSecret ? t("endpoints.saved") : t("endpoints.missing")}</span>
          </li>
        ))}
      </ul>
      <form
        className="flex flex-wrap gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          setEndpointSecret(draft);
          setDraft("");
        }}
      >
        <label className="text-sm">
          {t("endpoints.secret")}
          <Input className="h-8 ml-2" type="password" value={draft} autoComplete="off" onChange={(event) => setDraft(event.target.value)} />
        </label>
        <Button type="submit" variant="outline" size="sm">
          {t("endpoints.save")}
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => setExported(JSON.stringify(exportEndpoints()))}>
          {t("endpoints.export")}
        </Button>
        <Button type="button" variant="outline" size="sm" onClick={() => { void probeEndpoint(endpoints[0]?.id ?? "endpoint").then((value) => { if (typeof value === "string") setProbed(value); }); }}>
          {t("endpoints.runProbe")}
        </Button>
      </form>
      {probed ? <p data-testid="probe-result">{probed}</p> : null}
      <p className="text-sm">{t("endpoints.probe")}</p>
      <section data-testid="endpoint-prices">
        <h2 className="text-sm font-semibold">{t("endpoints.prices")}</h2>
        <ul className="text-sm tabular-nums">
          {prices.map((row) => (
            <li key={row.model}>{row.model} {row.input_per_mtok} / {row.output_per_mtok}</li>
          ))}
        </ul>
      </section>
      {exported ? (
        <pre data-testid="endpoint-export" className="max-w-full overflow-auto text-xs">
          {exported}
        </pre>
      ) : null}
    </section>
  );
}
