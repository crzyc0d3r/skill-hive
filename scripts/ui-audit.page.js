// In-page readability and layout audit, evaluated by scripts/ui-audit.mjs in every screen state.
// Returns a JSON string: low-contrast text (WCAG 2.1 AA: 4.5:1, 3:1 for large text), text set
// in a fallback font, text under 11px, horizontal overflow and clipped text.
(() => {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 1;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  const parse = (css) => {
    if (!css || css === "transparent") return [0, 0, 0, 0];
    ctx.clearRect(0, 0, 1, 1);
    ctx.fillStyle = "#000";
    ctx.fillStyle = css;
    ctx.fillRect(0, 0, 1, 1);
    const d = ctx.getImageData(0, 0, 1, 1).data;
    return [d[0], d[1], d[2], d[3] / 255];
  };
  const over = (top, below) => {
    const a = top[3] + below[3] * (1 - top[3]);
    if (a === 0) return [0, 0, 0, 0];
    const mix = (i) => (top[i] * top[3] + below[i] * below[3] * (1 - top[3])) / a;
    return [mix(0), mix(1), mix(2), a];
  };
  const lum = (c) => {
    const f = (v) => {
      v /= 255;
      return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2]);
  };
  const ratio = (a, b) => {
    const la = lum(a);
    const lb = lum(b);
    return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
  };
  const visible = (el) => {
    const cs = getComputedStyle(el);
    if (cs.display === "none" || cs.visibility === "hidden") return false;
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) return false;
    if (r.bottom < 0 || r.right < 0 || r.top > innerHeight || r.left > innerWidth) return false;
    return true;
  };
  const ariaHidden = (el) => {
    for (let e = el; e; e = e.parentElement) if (e.getAttribute("aria-hidden") === "true") return true;
    return false;
  };
  const effectiveOpacity = (el) => {
    let o = 1;
    for (let e = el; e; e = e.parentElement) o *= parseFloat(getComputedStyle(e).opacity || "1");
    return o;
  };
  // Composite ancestor backgrounds until opaque. Overlays and popovers paint their own
  // opaque surfaces, so walking the DOM is enough for this app.
  const backgroundBehind = (el) => {
    let acc = [0, 0, 0, 0];
    for (let e = el; e; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (cs.backgroundImage && cs.backgroundImage !== "none") {
        const m = cs.backgroundImage.match(/(rgb|lab|oklch|oklab|color|hsl)\([^)]*\)/);
        if (m) acc = over(acc, parse(m[0]));
      }
      acc = over(acc, parse(cs.backgroundColor));
      if (acc[3] >= 0.999) return acc;
    }
    const scheme = getComputedStyle(document.documentElement).colorScheme;
    return over(acc, scheme && scheme.includes("dark") ? [0, 0, 0, 1] : [255, 255, 255, 1]);
  };

  const text = [];
  const seen = new Set();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  let node;
  while ((node = walker.nextNode())) {
    const content = node.textContent.replace(/\s+/g, " ").trim();
    if (!content) continue;
    const el = node.parentElement;
    if (!el || seen.has(el)) continue;
    if (["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE"].includes(el.tagName)) continue;
    seen.add(el);
    if (!visible(el) || ariaHidden(el)) continue;
    const cs = getComputedStyle(el);
    const opacity = effectiveOpacity(el);
    const fg = parse(cs.color);
    fg[3] *= opacity;
    const behind = backgroundBehind(el);
    const fgOnBg = over(fg, behind);
    const r = ratio(fgOnBg, behind);
    const size = parseFloat(cs.fontSize);
    const weight = parseInt(cs.fontWeight, 10) || 400;
    const large = size >= 24 || (size >= 18.66 && weight >= 700);
    const disabled = !!el.closest("[disabled], [aria-disabled='true'], [data-disabled]");
    const family = cs.fontFamily || "";
    const problems = [];
    if (r < (large ? 3 : 4.5)) problems.push(disabled ? "low-contrast-disabled" : "low-contrast");
    if (!/^"?(Inter Variable|JetBrains Mono Variable)/.test(family)) problems.push("fallback-font");
    if (size < 11) problems.push("tiny-text");
    if (problems.length) {
      text.push({
        problems,
        text: content.slice(0, 60),
        tag: el.tagName.toLowerCase(),
        cls: (el.getAttribute("class") || "").slice(0, 90),
        color: cs.color,
        bg: behind.slice(0, 3).map((v) => Math.round(v)),
        ratio: Math.round(r * 100) / 100,
        size,
        fontFamily: family.slice(0, 40),
      });
    }
  }

  const overflow = [];
  if (document.documentElement.scrollWidth > innerWidth + 1) {
    overflow.push({ what: "document", scrollWidth: document.documentElement.scrollWidth, innerWidth });
  }
  const all = document.body.querySelectorAll("*");
  for (const el of all) {
    if (overflow.length > 40) break;
    if (!visible(el) || ariaHidden(el)) continue;
    const r = el.getBoundingClientRect();
    if (r.right <= innerWidth + 1 || r.width <= 4) continue;
    const cs = getComputedStyle(el);
    if (cs.position === "fixed") continue;
    let inScroller = false;
    for (let p = el.parentElement; p; p = p.parentElement) {
      const pcs = getComputedStyle(p);
      if (/(auto|scroll|hidden)/.test(pcs.overflowX)) {
        inScroller = p.scrollWidth > p.clientWidth;
        break;
      }
    }
    if (inScroller) continue;
    overflow.push({ what: "offscreen-right", tag: el.tagName.toLowerCase(), cls: (el.getAttribute("class") || "").slice(0, 80), right: Math.round(r.right), text: (el.textContent || "").trim().slice(0, 40) });
  }

  const clipped = [];
  for (const el of all) {
    if (clipped.length > 30) break;
    if (el.children.length || !visible(el) || ariaHidden(el)) continue;
    if (el.scrollWidth <= el.clientWidth + 2 || el.clientWidth === 0) continue;
    const cs = getComputedStyle(el);
    if (cs.overflowX === "visible" || cs.textOverflow === "ellipsis") continue;
    if (cs.webkitLineClamp && cs.webkitLineClamp !== "none") continue;
    if (["INPUT", "TEXTAREA", "PRE", "CODE"].includes(el.tagName) || el.classList.contains("sr-only")) continue;
    clipped.push({ tag: el.tagName.toLowerCase(), cls: (el.getAttribute("class") || "").slice(0, 80), text: (el.textContent || "").trim().slice(0, 50), scrollWidth: el.scrollWidth, clientWidth: el.clientWidth });
  }

  return JSON.stringify({
    url: location.href,
    viewport: [innerWidth, innerHeight],
    theme: document.documentElement.className,
    fontsReady: document.fonts ? document.fonts.status : "n/a",
    lowContrast: text.filter((t) => t.problems.includes("low-contrast")),
    lowContrastDisabled: text.filter((t) => t.problems.includes("low-contrast-disabled")).length,
    fallbackFonts: text.filter((t) => t.problems.includes("fallback-font")).slice(0, 10),
    tinyText: text.filter((t) => t.problems.includes("tiny-text")).slice(0, 20),
    overflow,
    clipped,
  });
})();
