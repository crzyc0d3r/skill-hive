import en from "../locales/en.json";

export const LOCALES = ["en"] as const;
export type Locale = (typeof LOCALES)[number];

const ENGLISH = en as Record<string, string>;

let locale: Locale = "en";

export function currentLocale(): Locale {
  return locale;
}

export function setLocale(next: string): Locale {
  locale = "en";
  if (typeof document !== "undefined") document.documentElement.lang = locale;
  void next;
  return locale;
}

export function t(key: string, vars?: Record<string, string>): string {
  const template = ENGLISH[key] ?? key;
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (_, name: string) => vars[name] ?? "");
}

export const TEXT_SIZES = {
  small: "14px",
  medium: "16px",
  large: "18px",
  xl: "20px",
} as const;

export type TextSize = keyof typeof TEXT_SIZES;

export function applyTextSize(size: TextSize): void {
  document.documentElement.style.fontSize = TEXT_SIZES[size];
  document.documentElement.dataset.textSize = size;
}

export function isTextSize(value: string | null | undefined): value is TextSize {
  return value === "small" || value === "medium" || value === "large" || value === "xl";
}
