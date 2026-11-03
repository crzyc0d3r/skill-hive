import * as React from "react";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";

/** Chip-style tag editor: Enter or comma adds, Backspace on an empty input removes the last tag. */
export function TagsInput({
  tags,
  onChange,
  placeholder = "Add tag…",
  className,
  "aria-label": ariaLabel = "Tags",
}: {
  tags: string[];
  onChange: (tags: string[]) => void;
  placeholder?: string;
  className?: string;
  "aria-label"?: string;
}) {
  const [value, setValue] = React.useState("");
  const commit = () => {
    const t = value.trim().toLowerCase().replace(/,+$/, "");
    if (t && !tags.includes(t)) onChange([...tags, t]);
    setValue("");
  };
  return (
    <div
      className={cn(
        "flex min-h-8 flex-wrap items-center gap-1.5 rounded-md bg-elevated px-1.5 py-1 hairline focus-within:ring-2 focus-within:ring-[var(--accent-ring)]",
        className,
      )}
    >
      {tags.map((t) => (
        <span key={t} className="inline-flex items-center gap-1 rounded-full bg-sunken pl-2 pr-1 text-[11px] text-muted">
          {t}
          <button type="button" className="rounded-full p-0.5 hover:text-fg" onClick={() => onChange(tags.filter((x) => x !== t))} aria-label={`Remove tag ${t}`}>
            <X className="size-3" />
          </button>
        </span>
      ))}
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === ",") {
            e.preventDefault();
            commit();
          } else if (e.key === "Backspace" && !value && tags.length) {
            onChange(tags.slice(0, -1));
          }
        }}
        onBlur={commit}
        placeholder={tags.length ? "" : placeholder}
        aria-label={ariaLabel}
        className="h-6 min-w-24 flex-1 bg-transparent px-1 text-xs outline-none placeholder:text-subtle"
      />
    </div>
  );
}
