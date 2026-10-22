export interface ScanCopy {
  name: string;
  tool: string;
  path: string;
  alreadyManaged?: boolean;
  differs?: boolean;
  fingerprint?: string;
  /** "error", "parse", or empty. */
  reason?: string;
}

export interface GroupedScan extends ScanCopy {
  tools: string[];
  paths: string[];
  fingerprints: string[];
  differs: boolean;
}

/** A row can be imported. Read and parse failures are not new skills. */
export function isFreshScan(row: { alreadyManaged?: boolean; reason?: string }): boolean {
  return !row.alreadyManaged && row.reason !== "error" && row.reason !== "parse";
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
    if (row.reason === "error" || (row.reason === "parse" && existing.reason !== "error")) existing.reason = row.reason;
  }
  return grouped;
}
