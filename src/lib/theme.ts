export type ThemeChoice = "light" | "dark" | "system";

export function resolveDark(choice: ThemeChoice, prefersDark: boolean): boolean {
  if (choice === "dark") return true;
  if (choice === "light") return false;
  return prefersDark;
}

export function applyTheme(choice: ThemeChoice, prefersDark?: boolean, options?: { persist?: boolean }): void {
  const dark =
    prefersDark ??
    (typeof window !== "undefined" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", resolveDark(choice, dark));
  document.documentElement.dataset.theme = choice;
  if (options?.persist !== false && typeof localStorage !== "undefined") {
    try {
      localStorage.setItem("skill-hive-theme", choice);
    } catch {
      /* Private mode can reject storage. The class is already applied. */
    }
  }
}

const THEME_KEYS = ["demo-setting:theme", "skill-hive-theme"];

/** Theme saved by the demo harness or the last applied choice. */
export function readStoredTheme(): ThemeChoice | null {
  if (typeof localStorage === "undefined") return null;
  for (const key of THEME_KEYS) {
    const value = localStorage.getItem(key);
    if (isThemeChoice(value)) return value;
  }
  return null;
}

/** Top-level crashes render outside the shell, so they never inherit its theme class. */
export function applyCrashTheme(): void {
  const stored = readStoredTheme();
  if (stored) applyTheme(stored);
  else applyTheme("dark", true, { persist: false });
}

export function isThemeChoice(value: string | null | undefined): value is ThemeChoice {
  return value === "light" || value === "dark" || value === "system";
}
