import dictionary from "../locales/en.json";

const table = dictionary as Record<string, string>;

export function t(key: string, vars?: Record<string, string>): string {
  const template = table[key] ?? key;
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
