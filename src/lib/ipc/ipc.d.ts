export interface IpcCommands {
  get_settings: { args: { key: string }; result: string | null };
  set_settings: { args: { key: string; value: string }; result: null };
  log_startup_event: { args: { label: string; elapsedMs: number }; result: null };
  check_app_update: { args: Record<string, never>; result: string };
  update_install_blocker: { args: { activeRuns: number }; result: string | null };
  updater_proxy: { args: { url: string }; result: string | null };
  list_base_entries: { args: Record<string, never>; result: string[] };
}
