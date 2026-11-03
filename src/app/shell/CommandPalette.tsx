import * as React from "react";
import { useNavigate } from "react-router-dom";
import { Moon, Plus, RefreshCw, ScanSearch, Sun, Monitor } from "lucide-react";
import { useUi } from "@/lib/store";
import { useItems } from "@/lib/api/hooks";
import { CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandSeparator, CommandShortcut } from "@/components/ui";
import { KindIcon } from "@/components/app/common";
import { NAV } from "./AppShell";

export function CommandPalette() {
  const open = useUi((s) => s.paletteOpen);
  const setOpen = useUi((s) => s.setPaletteOpen);
  const openItem = useUi((s) => s.openItem);
  const setTheme = useUi((s) => s.setTheme);
  const navigate = useNavigate();
  const [query, setQuery] = React.useState("");
  const { data: items } = useItems({ text: query || null, kinds: [], tags: [], sourceKinds: [], favoritesOnly: false, deployedTo: null, sort: "relevance", limit: 8 });

  const handleOpenChange = (o: boolean) => {
    setOpen(o);
    if (!o) setQuery("");
  };

  const go = (fn: () => void) => {
    handleOpenChange(false);
    fn();
  };

  return (
    <CommandDialog open={open} onOpenChange={handleOpenChange}>
      <CommandInput placeholder="Search skills, jump to a page, run an action…" value={query} onValueChange={setQuery} />
      <CommandList>
        <CommandEmpty>Nothing matches.</CommandEmpty>
        {items && items.length > 0 ? (
          <CommandGroup heading="Library">
            {items.map((item) => (
              <CommandItem key={item.id} value={`${item.name} ${item.description}`} onSelect={() => go(() => openItem(item.id))}>
                <KindIcon kind={item.kind} />
                <span className="font-medium">{item.name}</span>
                <span className="truncate text-muted">{item.description}</span>
              </CommandItem>
            ))}
          </CommandGroup>
        ) : null}
        <CommandGroup heading="Go to">
          {NAV.map((n) => (
            <CommandItem key={n.to} value={`go ${n.label}`} onSelect={() => go(() => navigate(n.to))}>
              <n.icon />
              {n.label}
              <CommandShortcut>⌘{n.shortcut}</CommandShortcut>
            </CommandItem>
          ))}
          <CommandItem value="go settings" onSelect={() => go(() => navigate("/settings"))}>
            <Monitor />
            Settings
            <CommandShortcut>⌘,</CommandShortcut>
          </CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading="Actions">
          <CommandItem value="scan this machine" onSelect={() => go(() => navigate("/discover?tab=local&scan=1"))}>
            <ScanSearch />
            Scan this machine for skills
          </CommandItem>
          <CommandItem value="add from github url" onSelect={() => go(() => navigate("/discover?tab=online"))}>
            <Plus />
            Add from GitHub or a URL
          </CommandItem>
          <CommandItem value="check for updates" onSelect={() => go(() => navigate("/library?updates=1"))}>
            <RefreshCw />
            Check sources for updates
          </CommandItem>
          <CommandItem value="theme light" onSelect={() => go(() => setTheme("light"))}>
            <Sun />
            Light theme
          </CommandItem>
          <CommandItem value="theme dark" onSelect={() => go(() => setTheme("dark"))}>
            <Moon />
            Dark theme
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
