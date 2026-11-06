// State of the app's own update: shared by the startup check in the shell and the Updates
// section of Settings > About, so a check started by one is visible in the other.

import { create } from "zustand";
import { ApiError, api, onUpdateProgress, type UpdateCheck } from "@/lib/api/client";

export type UpdateStatus =
  | "idle"
  | "checking"
  /** Checked: this build is the latest release. */
  | "latest"
  | "available"
  | "downloading"
  | "installing"
  /** Installed; the app is restarting. */
  | "installed"
  | "error";

interface UpdateState {
  status: UpdateStatus;
  /** The last successful check. */
  check: UpdateCheck | null;
  progress: { downloaded: number; total: number | null } | null;
  error: string | null;
  /** When the last check finished (ms since epoch). */
  checkedAt: number | null;
  /** Ask the backend for the latest release. Resolves with the check, or null when it failed. */
  checkForUpdates: () => Promise<UpdateCheck | null>;
  /** Download and install the update the last check found. Rejects on failure. */
  install: () => Promise<void>;
  reset: () => void;
}

const message = (err: unknown) => (err instanceof ApiError || err instanceof Error ? err.message : String(err));

export const useUpdate = create<UpdateState>((set, get) => ({
  status: "idle",
  check: null,
  progress: null,
  error: null,
  checkedAt: null,

  async checkForUpdates() {
    const { status } = get();
    if (status === "checking" || status === "downloading" || status === "installing" || status === "installed") return get().check;
    set({ status: "checking", error: null });
    try {
      const check = await api.updateCheck();
      set({ status: check.available ? "available" : "latest", check, checkedAt: Date.now() });
      return check;
    } catch (err) {
      set({ status: "error", error: message(err), checkedAt: Date.now() });
      return null;
    }
  },

  async install() {
    const { status, check } = get();
    if (!check?.available || status === "downloading" || status === "installing" || status === "installed") return;
    set({ status: "downloading", progress: { downloaded: 0, total: null }, error: null });
    const stop = onUpdateProgress((p) => {
      if (p.phase === "downloading") set({ status: "downloading", progress: { downloaded: p.downloaded, total: p.total } });
      else if (p.phase === "installing") set({ status: "installing" });
      else set({ status: "installed" });
    });
    try {
      await api.updateInstall();
      // On most platforms the backend restarts the app before this resolves.
      set({ status: "installed" });
    } catch (err) {
      set({ status: "available", progress: null, error: message(err) });
      throw err;
    } finally {
      stop();
    }
  },

  reset: () => set({ status: "idle", check: null, progress: null, error: null, checkedAt: null }),
}));
