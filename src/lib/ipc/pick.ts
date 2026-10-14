export async function pickPath(directory: boolean): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({ directory, multiple: false });
  return typeof selected === "string" ? selected : null;
}
