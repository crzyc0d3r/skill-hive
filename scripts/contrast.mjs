// WCAG 2.x relative luminance. Pairs are the text-on-surface rows from PLAN.md Part 4 §2.1.

function channel(value) {
  const c = value / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(hex) {
  const h = hex.replace("#", "");
  const r = channel(parseInt(h.slice(0, 2), 16));
  const g = channel(parseInt(h.slice(2, 4), 16));
  const b = channel(parseInt(h.slice(4, 6), 16));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function ratio(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const pairs = [
  ["light foreground/background", "#0F172A", "#FFFFFF", 4.5],
  ["light muted-foreground/background", "#475569", "#FFFFFF", 4.5],
  ["light primary/primary-foreground", "#1E293B", "#F8FAFC", 4.5],
  ["light success text", "#15803D", "#FFFFFF", 4.5],
  ["light warning text", "#B45309", "#FFFFFF", 4.5],
  ["light destructive text", "#B91C1C", "#FFFFFF", 4.5],
  ["light info text", "#1D4ED8", "#FFFFFF", 4.5],
  ["dark foreground/background", "#F8FAFC", "#0F172A", 4.5],
  ["dark muted-foreground/background", "#94A3B8", "#0F172A", 4.5],
  ["dark success text", "#4ADE80", "#0F172A", 4.5],
  ["dark warning text", "#FBBF24", "#0F172A", 4.5],
  ["dark destructive text", "#F87171", "#0F172A", 4.5],
  ["dark info text", "#60A5FA", "#0F172A", 4.5],
];

let failed = 0;
for (const [name, fg, bg, min] of pairs) {
  const value = ratio(fg, bg);
  const ok = value + 1e-6 >= min;
  console.log(`${ok ? "pass" : "fail"} ${name}: ${value.toFixed(2)}:1 (min ${min}:1)`);
  if (!ok) failed += 1;
}
if (failed) {
  console.error(`${failed} text pair(s) below 4.5:1`);
  process.exit(1);
}
