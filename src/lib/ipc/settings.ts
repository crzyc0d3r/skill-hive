import { invoke } from "@tauri-apps/api/core";
import type { IpcCommands } from "./ipc";

export function getSettings(key: string) {
  return invoke<IpcCommands["get_settings"]["result"]>("get_settings", { key });
}

export function setSettings(key: string, value: string) {
  return invoke<IpcCommands["set_settings"]["result"]>("set_settings", { key, value });
}

export function logStartupEvent(label: string, elapsedMs: number) {
  return invoke<IpcCommands["log_startup_event"]["result"]>("log_startup_event", {
    label,
    elapsedMs,
  });
}

export function listBaseEntries() {
  return invoke<IpcCommands["list_base_entries"]["result"]>("list_base_entries");
}
