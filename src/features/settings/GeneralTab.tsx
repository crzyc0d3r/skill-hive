import * as React from "react";
import { toast } from "sonner";
import { Copy, FolderOpen, FolderPlus, Link2, Monitor, Moon, Sun, X } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { useSettings } from "@/lib/api/hooks";
import { useUi, type Theme } from "@/lib/store";
import { cn, displayPath } from "@/lib/utils";
import { Button, Input, Skeleton, Tooltip, copyText, pickDirectory, revealPath } from "@/components/ui";
import { ErrorNote, InlineCode } from "@/components/app/common";
import { Segmented, SettingsRow, SettingsSection } from "./bits";
import { useSettingsPatch } from "./shared";

const LINK_MODES: { value: T.LinkMode; label: string; description: string }[] = [
  { value: "link", label: "Link", description: "Symlink into the agent's directory. Edits in the library apply instantly; the agent must follow symlinks (Claude Code, Codex and Cursor do)." },
  { value: "copy", label: "Copy", description: "Independent copy in the agent's directory. Works everywhere; re-sync after editing the library version." },
];

export function GeneralTab() {
  const { data: settings, isLoading, error } = useSettings();
  const save = useSettingsPatch();
  const theme = useUi((s) => s.theme);
  const setTheme = useUi((s) => s.setTheme);

  if (isLoading || !settings) {
    return (
      <div className="flex max-w-3xl flex-col gap-4">
        <Skeleton className="h-40 w-full" />
        <Skeleton className="h-32 w-full" />
        <ErrorNote error={error} />
      </div>
    );
  }

  const addScanDir = async () => {
    const dir = await pickDirectory("Add a folder to scan for skills");
    if (!dir) return;
    if (settings.extraScanDirs.includes(dir)) {
      toast("Already in the list");
      return;
    }
    save.mutate({ patch: { extraScanDirs: [...settings.extraScanDirs, dir] }, message: "Scan folder added" });
  };

  return (
    <div className="flex max-w-3xl flex-col gap-5">
      <SettingsSection title="Library" description="Where Skill Hive keeps its database, the library and run workspaces.">
        <SettingsRow label="Home directory" description="Read-only. Set HIVE_HOME before launching to move it.">
          <InlineCode className="inline-block max-w-xs truncate align-middle" title={settings.home}>
            {displayPath(settings.home)}
          </InlineCode>
          <Tooltip content="Copy path">
            <Button variant="ghost" size="icon-sm" aria-label="Copy home path" onClick={() => void copyText(settings.home).then(() => toast("Path copied"))}>
              <Copy />
            </Button>
          </Tooltip>
          <Tooltip content="Reveal in file manager">
            <Button variant="ghost" size="icon-sm" aria-label="Reveal home directory" onClick={() => void revealPath(settings.home)}>
              <FolderOpen />
            </Button>
          </Tooltip>
        </SettingsRow>
        <SettingsRow label="Extra scan directories" description="Searched for SKILL.md folders on every local scan, in addition to the agent directories." align="start">
          <div className="flex w-80 flex-col gap-1.5">
            {settings.extraScanDirs.length === 0 ? <p className="text-xs text-subtle">None.</p> : null}
            {settings.extraScanDirs.map((d) => (
              <div key={d} className="flex items-center gap-2 rounded-md bg-sunken py-1 pl-2.5 pr-1 text-xs">
                <span className="mono min-w-0 flex-1 truncate selectable" title={d}>
                  {displayPath(d)}
                </span>
                <Button variant="ghost" size="icon-sm" aria-label={`Remove ${d}`} onClick={() => save.mutate({ patch: { extraScanDirs: settings.extraScanDirs.filter((x) => x !== d) }, message: "Scan folder removed" })}>
                  <X />
                </Button>
              </div>
            ))}
            <div>
              <Button size="sm" variant="secondary" onClick={() => void addScanDir()}>
                <FolderPlus /> Add folder…
              </Button>
            </div>
          </div>
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Deployment" description="Defaults used when deploying a skill to an agent.">
        <SettingsRow label="Default link mode" description="How a skill is placed into an agent's directory. Each deployment can override it." align="start">
          <div className="grid w-80 gap-1.5" role="radiogroup" aria-label="Default link mode">
            {LINK_MODES.map((m) => {
              const active = settings.defaultLinkMode === m.value;
              return (
                <button
                  key={m.value}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  onClick={() => !active && save.mutate({ patch: { defaultLinkMode: m.value }, message: `Default link mode: ${m.label}` })}
                  className={cn("flex items-start gap-2.5 rounded-lg px-3 py-2 text-left transition-colors", active ? "bg-accent-soft/70 hairline" : "bg-sunken hover:bg-line/60")}
                >
                  <span className={cn("mt-1 size-2.5 shrink-0 rounded-full border-2", active ? "border-accent bg-accent" : "border-line-strong")} />
                  <span className="min-w-0">
                    <span className="flex items-center gap-1.5 text-[13px] font-medium">
                      {m.value === "link" ? <Link2 className="size-3.5" /> : <Copy className="size-3.5" />} {m.label}
                    </span>
                    <span className="mt-0.5 block text-xs text-muted">{m.description}</span>
                  </span>
                </button>
              );
            })}
          </div>
        </SettingsRow>
        <SettingsRow label="Description budget" description="Characters of skill descriptions an agent loads before truncating the list. Used when an agent does not declare its own; Claude Code's limit is about 15 000.">
          <NumberCommit key={settings.defaultDescriptionBudgetChars} value={settings.defaultDescriptionBudgetChars} min={1000} max={200_000} step={500} suffix="chars" ariaLabel="Description budget characters" onCommit={(v) => save.mutate({ patch: { defaultDescriptionBudgetChars: v }, message: "Description budget saved" })} />
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Conflicts" description="How strictly overlapping descriptions are flagged in Find, Agents and Loadouts.">
        <SettingsRow label="Overlap threshold" description="Two descriptions at least this similar count as overlapping, which means the agent may pick the wrong one. Lower catches more; higher is stricter." align="start">
          <ThresholdSlider key={settings.overlapThreshold} value={settings.overlapThreshold} onCommit={(v) => save.mutate({ patch: { overlapThreshold: v }, message: `Overlap threshold ${v.toFixed(2)}` })} />
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Appearance">
        <SettingsRow label="Theme" description="System follows the OS setting.">
          <Segmented<Theme>
            ariaLabel="Theme"
            value={theme}
            onChange={(t) => {
              setTheme(t);
              save.mutate({ patch: { theme: t }, message: `Theme: ${t}` });
            }}
            options={[
              { value: "system", label: (<><Monitor /> System</>) },
              { value: "light", label: (<><Sun /> Light</>) },
              { value: "dark", label: (<><Moon /> Dark</>) },
            ]}
          />
        </SettingsRow>
      </SettingsSection>
      <ErrorNote error={save.error} />
    </div>
  );
}

function NumberCommit({ value, min, max, step, suffix, ariaLabel, onCommit }: { value: number; min: number; max: number; step?: number; suffix?: string; ariaLabel: string; onCommit: (v: number) => void }) {
  const [text, setText] = React.useState(String(value));
  const n = Number(text);
  const valid = text.trim() !== "" && Number.isFinite(n) && n >= min && n <= max;
  const dirty = valid && n !== value;
  const commit = () => dirty && onCommit(Math.round(n));
  return (
    <div className="flex items-center gap-2">
      <div className="relative">
        <Input
          type="number"
          min={min}
          max={max}
          step={step}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && commit()}
          onBlur={commit}
          aria-label={ariaLabel}
          aria-invalid={!valid}
          className={cn("w-36 pr-12 num", !valid && "ring-2 ring-danger/50")}
        />
        {suffix ? <span className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-[11px] text-subtle">{suffix}</span> : null}
      </div>
      {dirty ? (
        <Button size="sm" variant="primary" onClick={commit}>
          Save
        </Button>
      ) : null}
    </div>
  );
}

function ThresholdSlider({ value, onCommit }: { value: number; onCommit: (v: number) => void }) {
  const [v, setV] = React.useState(value);
  const commit = () => {
    if (Math.abs(v - value) > 1e-9) onCommit(v);
  };
  const hint = v < 0.4 ? "Aggressive: flags loosely related skills too." : v < 0.7 ? "Balanced." : "Strict: only near-duplicates are flagged.";
  return (
    <div className="flex w-80 flex-col gap-1.5">
      <div className="flex items-center gap-3">
        <input
          type="range"
          min={0.2}
          max={0.95}
          step={0.05}
          value={v}
          onChange={(e) => setV(Number(e.target.value))}
          onPointerUp={commit}
          onKeyUp={commit}
          onBlur={commit}
          aria-label="Overlap threshold"
          className="h-1.5 flex-1 cursor-default appearance-none rounded-full bg-line"
          style={{ accentColor: "var(--accent)" }}
        />
        <span className="num w-10 text-right text-[13px] font-medium">{v.toFixed(2)}</span>
      </div>
      <div className="flex justify-between text-[11px] text-subtle">
        <span>0.20 · loose</span>
        <span>{hint}</span>
        <span>0.95 · strict</span>
      </div>
    </div>
  );
}
