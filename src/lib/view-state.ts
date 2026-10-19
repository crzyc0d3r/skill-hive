import { useEffect, useState } from "react";

export type ViewPhase = "loading" | "empty" | "error" | "offline" | "ready";
export type PreviewPhase = Exclude<ViewPhase, "ready">;

export function previewPhase(): PreviewPhase | null {
  if (!import.meta.env.DEV || typeof window === "undefined") return null;
  const value = new URLSearchParams(window.location.search).get("state");
  if (value === "loading" || value === "empty" || value === "error" || value === "offline") return value;
  return null;
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
