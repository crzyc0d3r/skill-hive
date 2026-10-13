import en from "../locales/en.json";
import zhCN from "../locales/zh-CN.json";
import zhTW from "../locales/zh-TW.json";

export const LOCALES = ["en", "zh-CN", "zh-TW"] as const;
export type Locale = (typeof LOCALES)[number];

const TABLES: Record<Locale, Record<string, string>> = {
  en: en as Record<string, string>,
  "zh-CN": zhCN as Record<string, string>,
  "zh-TW": zhTW as Record<string, string>,
};

let locale: Locale = "en";

export function currentLocale(): Locale {
  return locale;
}

export function setLocale(next: string): Locale {
  locale = next === "zh-CN" || next === "zh-TW" ? next : "en";
  document.documentElement.lang = locale;
  return locale;
}

export function t(key: string, vars?: Record<string, string>): string {
  const template = TABLES[locale][key] ?? TABLES.en[key] ?? key;
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
