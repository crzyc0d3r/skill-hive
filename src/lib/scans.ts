export interface ScanCopy {
  name: string;
  tool: string;
  path: string;
  alreadyManaged?: boolean;
  differs?: boolean;
  fingerprint?: string;
  /** Empty when the folder read cleanly. Otherwise the scanner's own cause. */
  reason?: string;
}

export interface GroupedScan extends ScanCopy {
  tools: string[];
  paths: string[];
  fingerprints: string[];
  differs: boolean;
}

/** A row can be imported. Any read or parse cause keeps it out of the new count. */
export function isFreshScan(row: { alreadyManaged?: boolean; reason?: string }): boolean {
  return !row.alreadyManaged && !row.reason?.trim();
}

/** Parse causes rank under IO causes so the grouped row keeps the stronger failure. */
function reasonRank(reason: string | undefined): number {
  const text = reason?.trim() ?? "";
  if (!text) return 0;
  if (text === "parse" || text.startsWith("parse error")) return 1;
  return 2;
}

/** One row per skill name. Copies differ when their content fingerprints do. */
export function groupScanRows(rows: ScanCopy[]): GroupedScan[] {
  const grouped: GroupedScan[] = [];
  for (const row of rows) {
    const existing = grouped.find((item) => item.name.trim().toLowerCase() === row.name.trim().toLowerCase());
    const fingerprint = row.fingerprint ?? "";
    if (!existing) {
      grouped.push({
        ...row,
        tools: [row.tool],
        paths: [row.path],
        fingerprints: fingerprint ? [fingerprint] : [],
        differs: Boolean(row.differs),
        reason: row.reason ?? "",
      });
      continue;
    }
    if (!existing.tools.includes(row.tool)) existing.tools.push(row.tool);
    if (!existing.paths.includes(row.path)) existing.paths.push(row.path);
    if (fingerprint && !existing.fingerprints.includes(fingerprint)) existing.fingerprints.push(fingerprint);
    if (row.differs || existing.fingerprints.length > 1) existing.differs = true;
    if (row.alreadyManaged) existing.alreadyManaged = true;
    if (reasonRank(row.reason) > reasonRank(existing.reason)) existing.reason = row.reason;
  }
  return grouped;
}
