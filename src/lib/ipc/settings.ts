import { invoke } from "@tauri-apps/api/core";
import { demoEnabled } from "../demo";
import type { IpcCommands } from "./ipc";

function demoSettingKey(key: string) {
  return `demo-setting:${key}`;
}

export function getSettings(key: string) {
  if (demoEnabled()) return Promise.resolve(localStorage.getItem(demoSettingKey(key)));
  return invoke<IpcCommands["get_settings"]["result"]>("get_settings", { key });
}

export function setSettings(key: string, value: string) {
  if (demoEnabled()) {
    localStorage.setItem(demoSettingKey(key), value);
    return Promise.resolve(null);
  }
  return invoke<IpcCommands["set_settings"]["result"]>("set_settings", { key, value });
}

export function logStartupEvent(label: string, elapsedMs: number) {
  return invoke<IpcCommands["log_startup_event"]["result"]>("log_startup_event", {
    label,
    elapsedMs,
  });
}

export function listBaseEntries() {
  try {
    return invoke<IpcCommands["list_base_entries"]["result"]>("list_base_entries").catch(() => []);
  } catch {
    return Promise.resolve([]);
  }
}
