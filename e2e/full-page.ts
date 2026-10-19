import { expect, type Page } from "@playwright/test";

/** Unlock the scrolling main so a full-page shot includes cards below the fold.
 *  The live shell is viewport height: the sidebar is h-full beside a scrolling main.
 *  Setting ancestor height to auto makes that percentage collapse, so the capture
 *  stretches the sidebar to the grown column. That stretch is capture-only.
 */
export async function expandForFullPage(page: Page) {
  await page.evaluate(() => {
    const main = document.querySelector('[data-testid="content"]');
    if (!(main instanceof HTMLElement)) return;
    const extra = Math.max(0, main.scrollHeight - main.clientHeight);
    let node: HTMLElement | null = main;
    while (node) {
      node.style.setProperty("height", "auto", "important");
      node.style.setProperty("max-height", "none", "important");
      node.style.setProperty("overflow", "visible", "important");
      node = node.parentElement;
    }
    document.documentElement.style.setProperty("height", "auto", "important");
    document.body.style.setProperty("height", "auto", "important");
    document.body.style.setProperty("min-height", `${window.innerHeight + extra + 32}px`, "important");
    const sidebar = document.querySelector('[data-testid="sidebar"]');
    const column = main.parentElement;
    if (sidebar instanceof HTMLElement && column instanceof HTMLElement) {
      const height = Math.max(column.getBoundingClientRect().height, window.innerHeight);
      sidebar.style.setProperty("height", `${height}px`, "important");
      sidebar.style.setProperty("min-height", `${height}px`, "important");
      sidebar.style.setProperty("align-self", "stretch", "important");
    }
  });
}

export async function expectRadarLabelsInside(page: Page) {
  const labels = await page.evaluate(() => {
    const overlay = document.querySelector('[data-testid="effect-overlay"]');
    const svg = overlay?.ownerSVGElement;
    if (!(svg instanceof SVGSVGElement)) return [];
    const box = svg.getBoundingClientRect();
    return [...svg.querySelectorAll("text")].map((node) => {
      const rect = node.getBoundingClientRect();
      return {
        text: (node.textContent ?? "").replace(/\s+/g, " ").trim(),
        left: rect.left - box.left,
        top: rect.top - box.top,
        right: box.right - rect.right,
        bottom: box.bottom - rect.bottom,
      };
    });
  });
  expect(labels.length).toBeGreaterThanOrEqual(8);
  for (const label of labels) {
    expect(label.left, label.text).toBeGreaterThanOrEqual(2);
    expect(label.top, label.text).toBeGreaterThanOrEqual(2);
    expect(label.right, label.text).toBeGreaterThanOrEqual(2);
    expect(label.bottom, label.text).toBeGreaterThanOrEqual(2);
  }
}
