import { agentDisplayName, agentHue } from "@/lib/agents";
import { cn } from "cn";

export function AgentMark({ agent, name }: { agent: string; name: string }) {
  const letter = name.trim().slice(0, 1).toUpperCase() || "?";
  return (
    <span
      aria-hidden="true"
      data-agent-mark={agent}
      className="inline-flex size-4 shrink-0 items-center justify-center rounded-[4px] text-[10px] font-semibold leading-none text-white"
      style={{ backgroundColor: `oklch(0.58 0.14 ${agentHue(agent)})` }}
    >
      {letter}
    </span>
  );
}

export function AgentBadge({
  agent,
  displayName,
  path,
  filled,
  className,
}: {
  agent: string;
  displayName?: string;
  path?: string;
  filled?: boolean;
  className?: string;
}) {
  const name = displayName && displayName !== agent ? displayName : agentDisplayName(agent);
  const tip = path ? `${name} · ${path}` : name;
  return (
    <span
      title={tip}
      data-filled={filled ? "true" : "false"}
      className={cn(
        "inline-flex max-w-full items-center gap-1.5 rounded-full bg-muted px-1.5 py-0.5 text-xs text-foreground",
        className,
      )}
    >
      <AgentMark agent={agent} name={name} />
      <span className="truncate">{name}</span>
    </span>
  );
}

export function AgentName({ agent, displayName }: { agent: string; displayName?: string }) {
  const name = displayName && displayName !== agent ? displayName : agentDisplayName(agent);
  return (
    <span className="inline-flex min-w-0 items-center gap-1.5" title={name}>
      <AgentMark agent={agent} name={name} />
      <span className="truncate">{name}</span>
    </span>
  );
}
