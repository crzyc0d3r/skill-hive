import { t } from "@/lib/i18n";

/** Plain words for a skill source. Unknown values are title-cased and never keep underscores. */
export function sourceLabel(source: string): string {
  const key = source.trim().toLowerCase();
  if (key === "local") return t("source.local");
  if (key === "git") return t("source.git");
  if (key === "scan") return t("source.scan");
  if (key === "marketplace" || key === "market" || key.includes("skills.sh")) return t("source.market");
  return source
    .split(/[_-]+/)
    .filter(Boolean)
    .map((word) => word.slice(0, 1).toUpperCase() + word.slice(1))
    .join(" ");
}
