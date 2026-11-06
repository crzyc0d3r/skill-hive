// Small client-side UI state (theme, palette, selection). Server state lives in React Query.

import { create } from "zustand";

export type Theme = "system" | "light" | "dark";

interface UiState {
  theme: Theme;
  setTheme: (t: Theme) => void;
  /** What `theme` resolves to on <html> right now ("system" follows the OS). Kept by applyTheme. */
  resolvedTheme: "light" | "dark";
  paletteOpen: boolean;
  setPaletteOpen: (open: boolean) => void;
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  /** Item currently open in the detail panel. */
  detailItemId: string | null;
  openItem: (id: string | null) => void;
  /** Skills the merge dialog is open for (null = closed). */
  mergeIds: string[] | null;
  openMerge: (ids: string[] | null) => void;
  /** Library selection for batch actions. */
  selectedIds: string[];
  toggleSelected: (id: string) => void;
  clearSelection: () => void;
  setSelection: (ids: string[]) => void;
}

const storedTheme = (): Theme => {
  try {
    const t = localStorage.getItem("hive.theme");
    return t === "light" || t === "dark" || t === "system" ? t : "system";
  } catch {
    return "system";
  }
};

export const useUi = create<UiState>((set) => ({
  theme: storedTheme(),
  setTheme: (theme) => {
    try {
      localStorage.setItem("hive.theme", theme);
    } catch {
      // ignore
    }
    set({ theme });
  },
  resolvedTheme: "light",
  paletteOpen: false,
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  detailItemId: null,
  openItem: (detailItemId) => set({ detailItemId }),
  mergeIds: null,
  openMerge: (mergeIds) => set({ mergeIds }),
  selectedIds: [],
  toggleSelected: (id) =>
    set((s) => ({ selectedIds: s.selectedIds.includes(id) ? s.selectedIds.filter((x) => x !== id) : [...s.selectedIds, id] })),
  clearSelection: () => set({ selectedIds: [] }),
  setSelection: (selectedIds) => set({ selectedIds }),
}));

/** Apply the theme class to <html>. */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  const prefersDark = window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
  const dark = theme === "dark" || (theme === "system" && prefersDark);
  root.classList.toggle("dark", dark);
  useUi.setState({ resolvedTheme: dark ? "dark" : "light" });
}
