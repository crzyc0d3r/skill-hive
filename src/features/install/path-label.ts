/** Keep the start and the file name visible when a path is too long for one row. */
export function middleEllipsis(value: string, max = 56): string {
  const text = value.replace(/\s+/g, " ").trim();
  if (text.length <= max) return text;
  const keep = Math.max(8, Math.floor((max - 1) / 2));
  return `${text.slice(0, keep)}…${text.slice(text.length - keep)}`;
}

/** Any reason string is a real read or parse failure. An empty reason scanned cleanly. */
export function scanReasonIsError(reason: string | undefined): boolean {
  return Boolean(reason?.trim());
}
