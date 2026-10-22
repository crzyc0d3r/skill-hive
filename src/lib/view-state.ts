import { createContext, useContext, useEffect, useState } from "react";

export type ViewPhase = "loading" | "empty" | "error" | "offline" | "ready";
export type PreviewPhase = Exclude<ViewPhase, "ready">;

export function previewPhase(): PreviewPhase | null {
  if (!import.meta.env.DEV || typeof window === "undefined") return null;
  const value = new URLSearchParams(window.location.search).get("state");
  if (value === "loading" || value === "empty" || value === "error" || value === "offline") return value;
  return null;
}

export const DataPhaseContext = createContext<"loading" | "ready" | "error">("ready");
export const RetryContext = createContext<() => void>(() => undefined);

export function useShellPhase(): "loading" | "error" | "ready" {
  const forced = previewPhase();
  const dataPhase = useContext(DataPhaseContext);
  if (forced === "loading") return "loading";
  if (forced === "error") return "error";
  if (forced) return "ready";
  return dataPhase;
}

/** Includes the screenshot phases `empty` and `offline`, which the shell phase folds away. */
export function useViewPhase(): ViewPhase {
  const forced = previewPhase();
  const dataPhase = useContext(DataPhaseContext);
  if (forced) return forced;
  if (dataPhase === "loading" || dataPhase === "error") return dataPhase;
  return "ready";
}

export function useOnline(): boolean {
  const [online, setOnline] = useState(() => typeof navigator === "undefined" || navigator.onLine !== false);
  useEffect(() => {
    const sync = () => setOnline(navigator.onLine);
    window.addEventListener("online", sync);
    window.addEventListener("offline", sync);
    return () => {
      window.removeEventListener("online", sync);
      window.removeEventListener("offline", sync);
    };
  }, []);
  return online;
}
