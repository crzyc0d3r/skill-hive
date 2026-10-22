/** Keep the start and the file name visible when a path is too long for one row. */
export function middleEllipsis(value: string, max = 56): string {
  const text = value.replace(/\s+/g, " ").trim();
  if (text.length <= max) return text;
  const keep = Math.max(8, Math.floor((max - 1) / 2));
  return `${text.slice(0, keep)}…${text.slice(text.length - keep)}`;
}

export function scanReasonIsError(reason: string | undefined): boolean {
  return reason === "error" || reason === "parse";
}
