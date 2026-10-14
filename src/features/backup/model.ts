export interface BackupSnapshot {
  id: string;
  label: string;
  body: string;
}

export function backupNow(body: string, history: BackupSnapshot[]): BackupSnapshot[] {
  return [{ id: `snap-${history.length + 1}`, label: `Snapshot ${history.length + 1}`, body }, ...history];
}
export function restoreSnapshot(history: BackupSnapshot[], id: string): { body: string; safety: string } | null {
  const snap = history.find((item) => item.id === id);
  if (!snap) return null;
  return { body: snap.body, safety: `safety-${id}` };
}
