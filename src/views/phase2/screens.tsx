import { useState } from "react";
import { t } from "@/lib/i18n";
import {
  addSkillToAgents,
  applyRemoval,
  backupNow,
  dotColor,
  filterMarket,
  HELP_CARDS,
  MARKET_FIXTURE,
  pullCenter,
  removalPlan,
  restoreSnapshot,
  toggleSkill,
  type MarketSkill,
  type ProjectSkill,
} from "@/lib/phase2/model";
import { setProjectSkills } from "@/lib/phase2/store";

const FIXTURE: ProjectSkill = {
  id: "notes",
  name: "notes",
  agents: [],
  enabled: true,
  center: "center v2",
  local: "center v1",
};

export function ProjectView() {
  const [linked, setLinked] = useState(false);
  const [skill, setSkill] = useState<ProjectSkill>(FIXTURE);
  const update = (next: ProjectSkill) => {
    setSkill(next);
    setProjectSkills([next]);
  };
  return (
    <section data-testid="project">
      <h1 className="font-mono text-[22px] font-semibold">{t("project.title")}</h1>
      <button
        type="button"
        data-testid="link-project"
        className="mt-3 rounded-md border border-border px-2 py-1 text-sm"
        onClick={() => {
          setLinked(true);
          setProjectSkills([skill]);
        }}
      >
        {t("project.link")}
      </button>
      <button type="button" className="ml-2 rounded-md border border-border px-2 py-1 text-sm" onClick={() => setLinked(true)}>
        {t("project.linked")}
      </button>
      {linked ? (
        <div className="mt-4 space-y-2 text-sm">
          <p>
            {skill.name}
            <span data-testid="project-dot" data-color={dotColor(skill)} className="ml-2 inline-block h-2 w-2 rounded-full bg-current" />
          </p>
          <p>{t("project.agents")}: {skill.agents.join(", ") || "—"}</p>
          <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => update(addSkillToAgents(skill, ["claude_code", "codex"]))}>
            {t("project.add")}
          </button>
          <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => update(toggleSkill(skill, !skill.enabled))}>
            {t("project.disable")}
          </button>
          <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => update(pullCenter(skill))}>
            {t("project.pull")}
          </button>
          <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => update({ ...skill, enabled: true })}>
            {t("project.batch")}
          </button>
          <div className="flex gap-2">
            <span>{t("library.detail.local")}</span>
            <span>{t("library.detail.diff")}</span>
            <span>{t("workspace.detail.center")}</span>
          </div>
        </div>
      ) : null}
    </section>
  );
}

export function MarketplacePanel({ reachable = true, skills = MARKET_FIXTURE }: { reachable?: boolean; skills?: MarketSkill[] }) {
  const [query, setQuery] = useState("");
  const [owner, setOwner] = useState<string | null>(null);
  const [installed, setInstalled] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const rows = filterMarket(skills, query, owner);
  if (!reachable) return <p className="mt-4 text-sm text-muted-foreground">{t("install.leaderboard.empty")}</p>;
  return (
    <div className="mt-4 space-y-2" data-testid="marketplace">
      <input aria-label={t("install.search")} value={query} onChange={(event) => setQuery(event.target.value)} className="rounded-md border border-input px-2 py-1 text-sm" />
      <div className="flex flex-wrap gap-2">
        {["acme", "beta"].map((item) => (
          <button key={item} type="button" aria-pressed={owner === item} className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => setOwner(owner === item ? null : item)}>
            {t("install.owner")} {item}
          </button>
        ))}
      </div>
      <ul className="space-y-1 text-sm">
        {rows.map((skill) => (
          <li key={`${skill.owner}/${skill.name}`} className="flex items-center gap-2">
            <span>{skill.owner}/{skill.name}</span>
            <button type="button" className="rounded-md bg-primary px-2 py-1 text-primary-foreground" onClick={() => { setPending(skill.name); setInstalled(skill.name); }}>
              {t("install.install")}
            </button>
          </li>
        ))}
      </ul>
      {pending ? (
        <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => { setPending(null); setInstalled(null); }}>
          {t("install.cancelInstall")}
        </button>
      ) : null}
      {installed ? <p role="status">{t("install.installed")} {installed}</p> : null}
      <button type="button" className="rounded-md border border-border px-2 py-1 text-sm">{t("install.loadMore")}</button>
    </div>
  );
}

export function UpdateFlow() {
  const plan = removalPlan(["notes.txt"]);
  const [open, setOpen] = useState(false);
  const [applied, setApplied] = useState<boolean | null>(null);
  return (
    <div data-testid="updates" className="space-y-2 text-sm">
      <button type="button" data-testid="check-all" className="rounded-md border border-border px-2 py-1" onClick={() => setOpen(true)}>{t("library.checkAll")}</button>
      <button type="button" className="ml-2 rounded-md border border-border px-2 py-1" onClick={() => setOpen(true)}>{t("library.menu.update")}</button>
      {open ? (
        <div role="dialog" aria-label={t("library.removal.title")} className="rounded-md border border-border p-3">
          <p>{t("library.removal.title")}</p>
          <ul>{plan.paths.map((path) => <li key={path}>{path}</li>)}</ul>
          <button type="button" data-testid="update-anyway" className="mt-2 rounded-md bg-primary px-2 py-1 text-primary-foreground" onClick={() => setApplied(applyRemoval(plan, plan.token))}>
            {t("library.removal.anyway")}
          </button>
        </div>
      ) : null}
      {applied ? <p role="status">{t("library.updateAll")}</p> : null}
      <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => setApplied(false)}>{t("library.relink")}</button>
      <button type="button" className="ml-2 rounded-md border border-border px-2 py-1">{t("library.keepLocal")}</button>
    </div>
  );
}

export function BackupView() {
  const [body, setBody] = useState("library v1");
  const [history, setHistory] = useState<{ id: string; label: string; body: string }[]>([]);
  const [safety, setSafety] = useState<string | null>(null);
  const [device, setDevice] = useState("this-machine");
  return (
    <section data-testid="backup">
      <h1 className="font-mono text-[22px] font-semibold">{t("backup.title")}</h1>
      <p className="mt-2 text-sm" role="status">{body}</p>
      <div className="mt-3 flex flex-wrap gap-2 text-sm">
        <button type="button" data-testid="backup-now" className="rounded-md bg-primary px-2 py-1 text-primary-foreground" onClick={() => setHistory(backupNow(body, history))}>{t("backup.now")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.sync")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.github")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.disconnect")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.revoke")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.retry")}</button>
        <a className="rounded-md border border-border px-2 py-1" href="https://github.com/settings/applications" target="_blank" rel="noreferrer">{t("backup.deletion")}</a>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.setup")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.recovery")}</button>
        <button type="button" className="rounded-md border border-border px-2 py-1">{t("backup.setAside")}</button>
      </div>
      <p className="mt-2 text-sm">{t("backup.public")}</p>
      <label className="mt-2 block text-sm">
        {t("backup.pat")}
        <input aria-label={t("backup.pat")} className="ml-2 rounded-md border border-input px-2 py-1" />
      </label>
      <label className="mt-2 block text-sm">
        {t("backup.advanced")}
        <input aria-label={t("backup.advanced")} defaultValue="git@localhost:hive.git" className="ml-2 rounded-md border border-input px-2 py-1" />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("settings.engine.note")}</p>
      <button type="button" disabled className="rounded-md border border-border px-2 py-1 text-sm">{t("settings.engine.libgit2")}</button>
      <label className="mt-3 block text-sm">
        {t("backup.device")}
        <input aria-label={t("backup.device")} value={device} onChange={(event) => setDevice(event.target.value)} className="ml-2 rounded-md border border-input px-2 py-1" />
      </label>
      <p className="mt-2 text-sm text-muted-foreground">{t("backup.remote")} git@localhost:hive.git</p>
      <p className="text-sm">{t("backup.scope")}</p>
      <ul className="mt-2 space-y-1 text-sm">
        {history.map((snap) => (
          <li key={snap.id}>
            {snap.label}
            <button
              type="button"
              data-testid="backup-restore"
              className="ml-2 rounded-md border border-border px-2 py-1"
              onClick={() => {
                const restored = restoreSnapshot(history, snap.id);
                if (!restored) return;
                setBody(restored.body);
                setSafety(restored.safety);
              }}
            >
              {t("backup.restore")}
            </button>
          </li>
        ))}
      </ul>
      {safety ? <p role="status">{t("backup.history")} {safety}</p> : null}
      <p className="mt-2 text-sm">{t("backup.firstRun")}</p>
    </section>
  );
}

const HELP_ANCHORS: Record<(typeof HELP_CARDS)[number], string> = {
  "help.card.library": "library",
  "help.card.install": "install-skills",
  "help.card.workspace": "global-workspace",
  "help.card.project": "project-workspace",
  "help.card.backup": "backup",
  "help.card.updates": "updates",
  "help.card.tray": "tray",
  "help.card.cli": "cli",
};

export function HelpDialog({ onClose }: { onClose: () => void }) {
  return (
    <div role="dialog" aria-label={t("help.title")} className="fixed inset-0 z-[70] flex items-center justify-center bg-[rgb(15_23_42/0.4)]">
      <div className="max-h-[80vh] w-[32rem] max-w-[90vw] overflow-auto rounded-lg border border-border bg-popover p-4">
        <h2 className="font-medium">{t("help.title")}</h2>
        <ul className="mt-3 grid grid-cols-2 gap-2 text-sm">
          {HELP_CARDS.map((key) => (
            <li key={key} className="rounded-md border border-border p-2" data-help-card={key}>
              <a href={`https://github.com/crzyc0d3r/skill-hive/blob/main/docs/user-guide.md#${HELP_ANCHORS[key]}`}>{t(key)}</a>
            </li>
          ))}
        </ul>
        <button type="button" className="mt-3 rounded-md border border-border px-2 py-1 text-sm" onClick={onClose}>Close</button>
      </div>
    </div>
  );
}

export function CloseGuardDialog({
  onRemember,
  onClose,
}: {
  onRemember: (choice: "minimize" | "quit") => void;
  onClose: () => void;
}) {
  const [remember, setRemember] = useState(false);
  return (
    <div role="dialog" aria-label={t("close.dialog")} className="fixed inset-0 z-[70] flex items-center justify-center bg-[rgb(15_23_42/0.4)]">
      <div className="rounded-lg border border-border bg-popover p-4 text-sm">
        <h2 className="font-medium">{t("close.dialog")}</h2>
        <label className="mt-2 block">
          <input type="checkbox" checked={remember} onChange={(event) => setRemember(event.target.checked)} /> {t("close.remember")}
        </label>
        <div className="mt-3 flex gap-2">
          <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => { if (remember) onRemember("minimize"); onClose(); }}>{t("settings.close.minimize")}</button>
          <button type="button" className="rounded-md border border-border px-2 py-1" onClick={() => { if (remember) onRemember("quit"); onClose(); }}>{t("settings.close.quit")}</button>
        </div>
      </div>
    </div>
  );
}

export function LobsterView({ agentKey }: { agentKey?: string }) {
  return (
    <section data-testid="lobster">
      <h1 className="font-mono text-[22px] font-semibold">{t("lobster.title")}</h1>
      <p className="mt-2 text-sm">{agentKey ? agentKey : t("lobster.all")}</p>
    </section>
  );
}

export function GitStatusButton() {
  return (
    <button type="button" className="rounded-md border border-border px-2 py-1 text-sm">{t("library.gitStatus")}</button>
  );
}
