export type ThemeChoice = "light" | "dark" | "system";

export function resolveDark(choice: ThemeChoice, prefersDark: boolean): boolean {
  if (choice === "dark") return true;
  if (choice === "light") return false;
  return prefersDark;
}

export function applyTheme(choice: ThemeChoice, prefersDark?: boolean): void {
  const dark =
    prefersDark ??
    (typeof window !== "undefined" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", resolveDark(choice, dark));
  document.documentElement.dataset.theme = choice;
}

export function isThemeChoice(value: string | null | undefined): value is ThemeChoice {
  return value === "light" || value === "dark" || value === "system";
}
