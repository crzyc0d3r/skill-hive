import * as React from "react";
import { Select as Primitive } from "radix-ui";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

export const Select = Primitive.Root;
export const SelectGroup = Primitive.Group;
export const SelectValue = Primitive.Value;

export const SelectTrigger = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof Primitive.Trigger> & { size?: "sm" | "md" }
>(({ className, children, size = "md", ...props }, ref) => (
  <Primitive.Trigger
    ref={ref}
    className={cn(
      "flex w-full items-center justify-between gap-2 rounded-md bg-elevated px-2.5 text-[13px] text-fg hairline",
      "focus:outline-none focus:ring-2 focus:ring-[var(--accent-ring)] disabled:opacity-50 data-[placeholder]:text-subtle [&>span]:truncate",
      size === "sm" ? "h-7 text-xs" : "h-8",
      className,
    )}
    {...props}
  >
    {children}
    <Primitive.Icon asChild>
      <ChevronDown className="size-3.5 shrink-0 text-muted" />
    </Primitive.Icon>
  </Primitive.Trigger>
));
SelectTrigger.displayName = "SelectTrigger";

export const SelectContent = React.forwardRef<HTMLDivElement, React.ComponentPropsWithoutRef<typeof Primitive.Content>>(
  ({ className, children, position = "popper", ...props }, ref) => (
    <Primitive.Portal>
      <Primitive.Content
        ref={ref}
        position={position}
        sideOffset={6}
        className={cn(
          "z-[60] max-h-[320px] min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-lg bg-elevated text-[13px] text-fg shadow-md hairline animate-slide-up",
          className,
        )}
        {...props}
      >
        <Primitive.Viewport className="p-1">{children}</Primitive.Viewport>
      </Primitive.Content>
    </Primitive.Portal>
  ),
);
SelectContent.displayName = "SelectContent";

export const SelectItem = React.forwardRef<HTMLDivElement, React.ComponentPropsWithoutRef<typeof Primitive.Item>>(
  ({ className, children, ...props }, ref) => (
    <Primitive.Item
      ref={ref}
      className={cn(
        "relative flex w-full cursor-default select-none items-center rounded-md py-1.5 pl-7 pr-2 outline-none data-[highlighted]:bg-sunken data-[disabled]:opacity-50",
        className,
      )}
      {...props}
    >
      <span className="absolute left-2 flex size-3.5 items-center justify-center">
        <Primitive.ItemIndicator>
          <Check className="size-3.5" />
        </Primitive.ItemIndicator>
      </span>
      <Primitive.ItemText>{children}</Primitive.ItemText>
    </Primitive.Item>
  ),
);
SelectItem.displayName = "SelectItem";

export function SelectLabel({ className, ...props }: React.ComponentPropsWithoutRef<typeof Primitive.Label>) {
  return <Primitive.Label className={cn("px-2 py-1.5 text-[11px] font-medium uppercase tracking-wide text-subtle", className)} {...props} />;
}

/** Simple controlled select over string options. */
export function SimpleSelect({
  value,
  onChange,
  options,
  placeholder,
  size,
  className,
  disabled,
  "aria-label": ariaLabel,
}: {
  value: string | null | undefined;
  onChange: (value: string) => void;
  options: { value: string; label: React.ReactNode; disabled?: boolean }[];
  placeholder?: string;
  size?: "sm" | "md";
  className?: string;
  disabled?: boolean;
  "aria-label"?: string;
}) {
  return (
    <Select value={value ?? undefined} onValueChange={onChange} disabled={disabled}>
      <SelectTrigger size={size} className={className} aria-label={ariaLabel}>
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {options.map((o) => (
          <SelectItem key={o.value} value={o.value} disabled={o.disabled}>
            {o.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
