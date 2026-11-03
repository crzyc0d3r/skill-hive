import * as React from "react";
import { Tabs as TabsPrimitive } from "radix-ui";
import { cn } from "@/lib/utils";

export const Tabs = TabsPrimitive.Root;

export const TabsList = React.forwardRef<HTMLDivElement, React.ComponentPropsWithoutRef<typeof TabsPrimitive.List>>(
  ({ className, ...props }, ref) => (
    <TabsPrimitive.List
      ref={ref}
      className={cn("inline-flex h-8 items-center gap-0.5 rounded-lg bg-sunken p-0.5 text-muted", className)}
      {...props}
    />
  ),
);
TabsList.displayName = "TabsList";

export const TabsTrigger = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Trigger
    ref={ref}
    className={cn(
      "inline-flex h-7 items-center justify-center gap-1.5 whitespace-nowrap rounded-md px-2.5 text-xs font-medium transition-colors",
      "data-[state=active]:bg-elevated data-[state=active]:text-fg data-[state=active]:shadow-sm hover:text-fg [&_svg]:size-3.5",
      className,
    )}
    {...props}
  />
));
TabsTrigger.displayName = "TabsTrigger";

export const TabsContent = React.forwardRef<
  HTMLDivElement,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Content>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Content ref={ref} className={cn("mt-3 outline-none", className)} {...props} />
));
TabsContent.displayName = "TabsContent";

/** Underline-style tabs for page headers. */
export const LineTabsList = React.forwardRef<HTMLDivElement, React.ComponentPropsWithoutRef<typeof TabsPrimitive.List>>(
  ({ className, ...props }, ref) => (
    <TabsPrimitive.List ref={ref} className={cn("flex items-center gap-4 border-b", className)} {...props} />
  ),
);
LineTabsList.displayName = "LineTabsList";

export const LineTabsTrigger = React.forwardRef<
  HTMLButtonElement,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.Trigger
    ref={ref}
    className={cn(
      "-mb-px flex h-9 items-center gap-1.5 border-b-2 border-transparent px-0.5 text-[13px] text-muted transition-colors hover:text-fg",
      "data-[state=active]:border-accent data-[state=active]:text-fg [&_svg]:size-3.5",
      className,
    )}
    {...props}
  />
));
LineTabsTrigger.displayName = "LineTabsTrigger";
