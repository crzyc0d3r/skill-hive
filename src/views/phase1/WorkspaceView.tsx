import { useState } from "react";
import { presetFill, presetLabel } from "@/lib/phase1/model";
import type { PresetRow } from "@/lib/phase1/client";
import { t } from "@/lib/i18n";

export interface WorkspaceCard {
  name: string;
  status: "local_only" | "in_sync" | "center_changed";
}

export function WorkspaceView({
  presets,
  cards,
  onApply,
  onCreate,
}: {
  presets: PresetRow[];
  cards: WorkspaceCard[];
  onApply?: (id: string) => void;
  onCreate?: (name: string) => void;
}) {
  const [applied, setApplied] = useState<string | null>(null);
  const [name, setName] = useState("Research");
  return (
    <section data-testid="global-workspace">
      <h1 className="font-mono text-[22px] font-semibold">{t("workspace.title")}</h1>
      <form
        className="mt-3 flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          onCreate?.(name);
        }}
      >
        <input aria-label={t("preset.create")} value={name} onChange={(event) => setName(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
        <button type="submit" className="rounded-md border border-border px-2 py-1 text-sm">{t("preset.new")}</button>
      </form>
      <div className="mt-3 flex flex-wrap gap-2" data-testid="preset-bar">
        {presets.map((preset) => {
          const total = preset.skillIds.length;
          const synced = applied === preset.id ? total : 0;
          const fill = presetFill(synced, total);
          return (
            <button
              key={preset.id}
              type="button"
              data-fill={fill}
              className="rounded-full border border-border px-3 py-1 text-sm"
              onClick={() => {
                setApplied(preset.id);
                onApply?.(preset.id);
              }}
            >
              {presetLabel(preset.name, synced, total)}
            </button>
          );
        })}
      </div>
      <ul className="mt-4 space-y-2">
        {cards.map((card) => (
          <li key={card.name} className="rounded-md border border-border p-3 text-sm">
            <span className="font-medium">{card.name}</span>
            <span className="ml-2 text-muted-foreground">
              {card.status === "local_only" ? t("workspace.localOnly") : card.status === "center_changed" ? t("workspace.centerChanged") : t("workspace.inSync")}
            </span>
            <div className="mt-2 flex gap-2">
              <button type="button" className="rounded-md border border-border px-2 py-1">{t("workspace.upload")}</button>
              <button type="button" className="rounded-md border border-border px-2 py-1">{t("workspace.pull")}</button>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
