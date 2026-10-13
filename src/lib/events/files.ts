import type { QueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const FILES_CHANGED = "app-files-changed";
export const SKILLS_AUTO_UPDATED = "skills-auto-updated";
export const BACKUP_AUTO_COMPLETED = "backup-auto-completed";
export const BASE_LISTING_KEY = ["base-listing"] as const;
export const SKILLS_KEY = ["skills"] as const;

export async function bindFileWatcher(queryClient: QueryClient): Promise<UnlistenFn> {
  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: BASE_LISTING_KEY });
    void queryClient.invalidateQueries({ queryKey: SKILLS_KEY });
  };
  try {
    const stops = await Promise.all([
      listen(FILES_CHANGED, refresh),
      listen(SKILLS_AUTO_UPDATED, refresh),
      listen(BACKUP_AUTO_COMPLETED, refresh),
    ]);
    return () => {
      for (const stop of stops) stop();
    };
  } catch {
    return () => {};
  }
}
