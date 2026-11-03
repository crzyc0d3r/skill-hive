import { cn } from "@/lib/utils";
import { LOADOUT_COLORS, LOADOUT_COLOR_NAMES } from "./colors";

/** Radio-style colour picker; clicking the active swatch clears the colour. */
export function ColorSwatches({ value, onChange, className }: { value: string | null; onChange: (c: string | null) => void; className?: string }) {
  return (
    <div role="radiogroup" aria-label="Colour" className={cn("flex items-center gap-1.5", className)}>
      {LOADOUT_COLORS.map((c) => {
        const active = value?.toLowerCase() === c;
        return (
          <button
            key={c}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={LOADOUT_COLOR_NAMES[c] ?? c}
            onClick={() => onChange(active ? null : c)}
            className={cn("size-5 rounded-full transition-[box-shadow] hover:ring-2 hover:ring-line-strong hover:ring-offset-2 hover:ring-offset-elevated", active && "ring-2 ring-fg ring-offset-2 ring-offset-elevated")}
            style={{ background: c }}
          />
        );
      })}
    </div>
  );
}

export function LoadoutDot({ color, className }: { color?: string | null; className?: string }) {
  return <span aria-hidden className={cn("inline-block size-2.5 shrink-0 rounded-full", !color && "bg-accent", className)} style={color ? { background: color } : undefined} />;
}
