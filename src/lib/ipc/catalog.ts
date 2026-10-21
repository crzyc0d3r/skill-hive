import { invoke } from "@tauri-apps/api/core";
import { DEMO, demoEnabled } from "../demo";
import type { SkillCard } from "@/features/library/model";

export interface PresetRow {
  id: string;
  name: string;
  skillIds: string[];
  synced: number;
}

export interface AgentRow {
  key: string;
  name: string;
  installed: boolean;
  enabled: boolean;
  path: string;
  projectPath: string;
  custom: boolean;
}

export interface ScanRow {
  name: string;
  tool: string;
  path: string;
}

const inflight = new Map<string, Promise<unknown>>();

async function call<T>(command: string, args: Record<string, unknown>, fallback: T): Promise<T> {
  if (demoEnabled()) return fallback;
  const key = `${command}:${JSON.stringify(args)}`;
  const pending = inflight.get(key);
  if (pending) return pending as Promise<T>;
  const request = invoke<T | null>(command, args)
    .then((value) => value ?? fallback)
    .catch(() => fallback)
    .finally(() => {
      if (inflight.get(key) === request) inflight.delete(key);
    });
  inflight.set(key, request);
  return request;
}

export function listSkills(): Promise<SkillCard[]> {
  return call("get_managed_skills", {}, demoEnabled() ? DEMO.skills : []);
}

export function listPresets(): Promise<PresetRow[]> {
  const fallback = demoEnabled()
    ? DEMO.presets.map((preset) => ({ ...preset, synced: preset.skillIds.length }))
    : [];
  return call("list_presets", {}, fallback);
}

export function listAgents(): Promise<AgentRow[]> {
  const fallback = demoEnabled()
    ? DEMO.agents.map((agent) => ({ ...agent, projectPath: ".agents/skills", custom: false }))
    : [];
  return call("get_tool_status", {}, fallback);
}

export function listScanned(): Promise<ScanRow[]> {
  const fallback = demoEnabled()
    ? DEMO.scanned.map((row) => ({ ...row, path: `~/.${row.tool}/skills/${row.name}` }))
    : [];
  return call("scan_local_skills", {}, fallback);
}

export function readSetting(key: string): Promise<string | null> {
  if (demoEnabled()) return Promise.resolve(localStorage.getItem(`demo-setting:${key}`));
  return invoke<string | null>("get_settings", { key }).catch(() => {
    throw new Error("settings-unavailable");
  });
}

export function writeSetting(key: string, value: string): Promise<void> {
  if (demoEnabled()) {
    localStorage.setItem(`demo-setting:${key}`, value);
    return Promise.resolve();
  }
  return invoke("set_settings", { key, value });
}
