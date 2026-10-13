import type { QueryClient } from "@tanstack/react-query";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const FILES_CHANGED = "app-files-changed";
export const BASE_LISTING_KEY = ["base-listing"] as const;

export async function bindFileWatcher(queryClient: QueryClient): Promise<UnlistenFn> {
  return listen(FILES_CHANGED, () => {
    void queryClient.invalidateQueries({ queryKey: BASE_LISTING_KEY });
  });
}
