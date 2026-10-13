export interface IpcCommands {
  get_settings: { args: { key: string }; result: string | null };
  set_settings: { args: { key: string; value: string }; result: null };
  log_startup_event: { args: { label: string; elapsedMs: number }; result: null };
  list_base_entries: { args: Record<string, never>; result: string[] };
}
