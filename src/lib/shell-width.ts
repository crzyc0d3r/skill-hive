import { useEffect, useState } from "react";

export type ShellBand = 800 | 1100 | 1440 | 1920;

/** Viewport bands from the design spec: 800, 1100, 1440, 1920. */
export function shellBand(width: number): ShellBand {
  if (!Number.isFinite(width) || width < 1100) return 800;
  if (width < 1440) return 1100;
  if (width < 1920) return 1440;
  return 1920;
}

/** Card columns at the four design widths: 1 / 2 / 3 / 4. */
export function designColumns(width: number): number {
  if (!Number.isFinite(width) || width < 1100) return 1;
  if (width < 1440) return 2;
  if (width < 1920) return 3;
  return 4;
}

export function useShellBand(): ShellBand {
  const [band, setBand] = useState<ShellBand>(() =>
    shellBand(typeof window === "undefined" ? 1440 : window.innerWidth),
  );
  useEffect(() => {
    const apply = () => setBand(shellBand(window.innerWidth));
    apply();
    window.addEventListener("resize", apply);
    return () => window.removeEventListener("resize", apply);
  }, []);
  return band;
}
