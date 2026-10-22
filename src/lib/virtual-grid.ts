import { useLayoutEffect, useRef, useState } from "react";
import { designColumns } from "@/lib/shell-width";

const MIN_CARD = 280;
const GAP = 12;
const UNMEASURED_CAP = 24;
const ROW_HEIGHT = { grid: 180, list: 76 } as const;

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

function scrollParent(node: HTMLElement): HTMLElement {
  let current = node.parentElement;
  while (current) {
    const overflow = getComputedStyle(current).overflowY;
    if (overflow === "auto" || overflow === "scroll") return current;
    current = current.parentElement;
  }
  return node;
}

/** One window into `items`. The page scroller owns scrolling; this hook does not. */
export function useVirtualGrid<T>(items: T[], layout: "grid" | "list") {
  const ref = useRef<HTMLDivElement>(null);
  const rowHeight = layout === "grid" ? ROW_HEIGHT.grid : ROW_HEIGHT.list;
  const [frame, setFrame] = useState({ width: 0, height: 0, scrollTop: 0 });
  const [viewport, setViewport] = useState(() => (typeof window === "undefined" ? 0 : window.innerWidth));
  useLayoutEffect(() => {
    const node = ref.current;
    if (!node) return;
    const scroller = scrollParent(node);
    let raf = 0;
    const measure = () => {
      const origin = node.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
      const next = {
        width: node.clientWidth,
        height: scroller.clientHeight,
        scrollTop: Math.max(0, scroller.scrollTop - origin),
      };
      setViewport((current) => (current === window.innerWidth ? current : window.innerWidth));
      setFrame((current) =>
        current.width === next.width && current.height === next.height && current.scrollTop === next.scrollTop ? current : next,
      );
    };
    const schedule = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(measure);
    };
    measure();
    const observer = new ResizeObserver(schedule);
    observer.observe(scroller);
    scroller.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("resize", schedule);
    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      scroller.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
    };
  }, [layout, items.length]);
  const columns = layout === "list" ? 1 : designColumns(viewport || frame.width);
  const perRow = Math.max(1, columns);
  const startRow = frame.height < 40 ? 0 : Math.max(0, Math.floor(frame.scrollTop / rowHeight) - 1);
  const rows = items.length === 0 ? 0 : Math.ceil(items.length / perRow);
  return {
    ref,
    columns,
    rowHeight,
    offset: startRow * rowHeight,
    totalHeight: rows * rowHeight,
    rendered: windowSlice(items, frame.scrollTop, frame.height, rowHeight, columns),
  };
}
