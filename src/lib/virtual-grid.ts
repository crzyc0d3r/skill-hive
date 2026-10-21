import { useEffect, useRef, useState } from "react";
import { designColumns } from "@/lib/shell-width";

const MIN_CARD = 280;
const GAP = 12;
const UNMEASURED_CAP = 24;

export function columnCount(width: number, minCard = MIN_CARD, gap = GAP): number {
  if (!Number.isFinite(width) || width <= 0) return 1;
  return Math.max(1, Math.floor((width + gap) / (minCard + gap)));
}

export function windowSlice<T>(items: T[], scrollTop: number, height: number, rowHeight: number, columns: number): T[] {
  const perRow = Math.max(1, columns);
  const row = Math.max(48, rowHeight);
  if (height < 40) return items.slice(0, Math.min(items.length, UNMEASURED_CAP * perRow));
  const start = Math.max(0, Math.floor(scrollTop / row) - 1);
  const count = Math.ceil(height / row) + 3;
  return items.slice(start * perRow, (start + count) * perRow);
}

export function useVirtualGrid<T>(items: T[], layout: "grid" | "list") {
  const ref = useRef<HTMLDivElement>(null);
  const [frame, setFrame] = useState({ width: 0, height: 0, scrollTop: 0, rowHeight: layout === "grid" ? 196 : 92 });
  const [viewport, setViewport] = useState(() => (typeof window === "undefined" ? 0 : window.innerWidth));
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    const measure = () => {
      const card = node.querySelector<HTMLElement>("[data-virtual-card]");
      const next = {
        width: node.clientWidth,
        height: node.clientHeight,
        scrollTop: node.scrollTop,
        rowHeight: card && card.offsetHeight > 0 ? card.offsetHeight : layout === "grid" ? 196 : 92,
      };
      setViewport(window.innerWidth);
      setFrame((current) =>
        current.width === next.width && current.height === next.height && current.scrollTop === next.scrollTop && current.rowHeight === next.rowHeight
          ? current
          : next,
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    node.addEventListener("scroll", measure, { passive: true });
    window.addEventListener("resize", measure);
    return () => {
      observer.disconnect();
      node.removeEventListener("scroll", measure);
      window.removeEventListener("resize", measure);
    };
  }, [layout, items.length]);
  const columns = layout === "list" ? 1 : designColumns(viewport || frame.width);
  return {
    ref,
    columns,
    rowHeight: frame.rowHeight,
    rendered: windowSlice(items, frame.scrollTop, frame.height, frame.rowHeight, columns),
  };
}
