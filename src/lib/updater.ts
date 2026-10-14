export type DesktopPlatform = "macos" | "windows" | "linux";

export type UpdateDecision =
  | { state: "current"; messageKey: "settings.about.latest" }
  | {
      state: "available";
      version: string;
      action: "install" | "download";
      messageKey: "settings.about.available";
    }
  | { state: "blocked"; messageKey: "settings.about.blocked" };

export function desktopPlatform(userAgent: string, override?: string | null): DesktopPlatform {
  if (override === "macos" || override === "windows" || override === "linux") return override;
  if (/Mac|iPhone|iPad/i.test(userAgent)) return "macos";
  if (/Win/i.test(userAgent)) return "windows";
  return "linux";
}

export function decideUpdate(options: {
  remoteVersion: string | null;
  platform: DesktopPlatform;
  activeRuns: number;
}): UpdateDecision {
  if (options.activeRuns > 0) return { state: "blocked", messageKey: "settings.about.blocked" };
  if (!options.remoteVersion) return { state: "current", messageKey: "settings.about.latest" };
  return {
    state: "available",
    version: options.remoteVersion,
    action: options.platform === "linux" ? "download" : "install",
    messageKey: "settings.about.available",
  };
}

export const LINUX_DOWNLOAD_URL = "https://github.com/crzyc0d3r/skill-hive/releases/latest";
