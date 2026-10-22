import { agentCode, agentDisplayName, agentHue } from "@/lib/agents";
import { cn } from "cn";

export function AgentMark({ agent, name }: { agent: string; name: string }) {
  const code = agentCode(agent);
  return (
    <span
      aria-hidden="true"
      data-agent-mark={agent}
      data-code={code}
      title={name}
      className="inline-flex h-4 min-w-4 shrink-0 items-center justify-center rounded-[4px] px-0.5 text-[9px] font-semibold leading-none tracking-tight text-white"
      style={{ backgroundColor: `oklch(0.52 0.16 ${agentHue(agent)})` }}
    >
      {code}
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
