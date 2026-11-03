import * as React from "react";
import { Star } from "lucide-react";
import type * as T from "@/lib/api/bindings";
import { cn, formatTokens, relativeTime } from "@/lib/utils";
import { Checkbox, Tooltip } from "@/components/ui";
import { EffectivenessBadge, KindIcon, SourceBadge, TagChip, ValidationBadge } from "@/components/app/common";

export function ItemCard({
  item,
  onOpen,
  selected,
  onToggleSelect,
  selectable,
  dense,
}: {
  item: T.LibraryItem;
  onOpen: () => void;
  selected?: boolean;
  onToggleSelect?: () => void;
  selectable?: boolean;
  dense?: boolean;
}) {
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onOpen();
        }
      }}
      className={cn(
        "group surface relative flex flex-col gap-2 p-3.5 text-left outline-none transition-[box-shadow,border-color,transform] duration-150",
        "hover:border-line-strong hover:shadow-md focus-visible:ring-2 focus-visible:ring-[var(--accent-ring)]",
        selected && "border-accent ring-1 ring-accent/40",
        dense && "p-3",
      )}
    >
      <div className="flex items-start gap-2.5">
        {selectable ? (
          <span
            onClick={(e) => {
              e.stopPropagation();
              onToggleSelect?.();
            }}
            className={cn("mt-0.5 transition-opacity", !selected && "opacity-0 group-hover:opacity-100")}
          >
            <Checkbox checked={!!selected} aria-label={`Select ${item.name}`} />
          </span>
        ) : null}
        <div className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-sunken text-muted">
          <KindIcon kind={item.kind} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5">
            <span className="truncate text-[13.5px] font-semibold tracking-[-0.01em]">{item.name}</span>
            {item.favorite ? <Star className="size-3 fill-accent text-accent" /> : null}
            {item.deploymentCount > 0 ? (
              <Tooltip content={`Deployed to ${item.deploymentCount} place${item.deploymentCount === 1 ? "" : "s"}`}>
                <span className="ml-auto flex items-center gap-1 text-[11px] text-ok">
                  <span className="size-1.5 rounded-full bg-ok" />
                  {item.deploymentCount}
                </span>
              </Tooltip>
            ) : null}
          </div>
          <p className={cn("mt-0.5 text-xs text-muted", dense ? "line-clamp-1" : "line-clamp-2")}>{item.description || "No description"}</p>
        </div>
      </div>
      <div className="mt-auto flex flex-wrap items-center gap-1.5">
        <EffectivenessBadge e={item.effectiveness} />
        <ValidationBadge status={item.validation} />
        {item.tags.slice(0, dense ? 2 : 4).map((t) => (
          <TagChip key={t} tag={t} />
        ))}
        <span className="ml-auto flex items-center gap-2 text-[11px] text-subtle">
          <SourceBadge source={item.source} />
          <Tooltip content="Estimated tokens of the instructions">
            <span className="num">{formatTokens(item.tokenEstimate)} tok</span>
          </Tooltip>
          <span>{relativeTime(item.updatedAt)}</span>
        </span>
      </div>
    </div>
  );
}

export function ItemRow({ item, onOpen, selected, onToggleSelect }: { item: T.LibraryItem; onOpen: () => void; selected?: boolean; onToggleSelect?: () => void }) {
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => {
        if (e.key === "Enter") onOpen();
      }}
      className={cn(
        "group grid grid-cols-[auto_minmax(0,1.2fr)_minmax(0,2fr)_auto_auto_auto] items-center gap-3 border-b px-3 py-2 text-xs outline-none transition-colors hover:bg-sunken/70 focus-visible:bg-sunken",
        selected && "bg-accent-soft/40",
      )}
    >
      <span
        onClick={(e) => {
          e.stopPropagation();
          onToggleSelect?.();
        }}
      >
        <Checkbox checked={!!selected} aria-label={`Select ${item.name}`} />
      </span>
      <span className="flex min-w-0 items-center gap-2">
        <KindIcon kind={item.kind} className="shrink-0 text-muted" />
        <span className="truncate font-medium text-[13px]">{item.name}</span>
        {item.favorite ? <Star className="size-3 shrink-0 fill-accent text-accent" /> : null}
      </span>
      <span className="truncate text-muted">{item.description}</span>
      <span className="flex items-center gap-1">
        <EffectivenessBadge e={item.effectiveness} />
        <ValidationBadge status={item.validation} />
      </span>
      <span className="flex items-center gap-1">
        {item.tags.slice(0, 3).map((t) => (
          <TagChip key={t} tag={t} />
        ))}
      </span>
      <span className="flex items-center gap-3 text-subtle">
        <SourceBadge source={item.source} />
        <span className="num w-14 text-right">{formatTokens(item.tokenEstimate)} tok</span>
        <span className="w-16 text-right">{relativeTime(item.updatedAt)}</span>
      </span>
    </div>
  );
}

export function ItemCardSkeleton() {
  return (
    <div className="surface flex flex-col gap-3 p-3.5">
      <div className="flex gap-2.5">
        <div className="size-7 animate-pulse-soft rounded-lg bg-sunken" />
        <div className="flex-1 space-y-2">
          <div className="h-3.5 w-1/2 animate-pulse-soft rounded bg-sunken" />
          <div className="h-3 w-full animate-pulse-soft rounded bg-sunken" />
          <div className="h-3 w-4/5 animate-pulse-soft rounded bg-sunken" />
        </div>
      </div>
      <div className="h-4 w-2/3 animate-pulse-soft rounded bg-sunken" />
    </div>
  );
}

export const MemoItemCard = React.memo(ItemCard);
