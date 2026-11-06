// Helpers and metadata shared by the settings tabs.

import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import type * as T from "@/lib/api/bindings";
import { api } from "@/lib/api/client";
import { keys, useApiMutation } from "@/lib/api/hooks";

export const REPO_URL = "https://github.com/crzyc0d3r/skill-hive";
export const DOCS_URL = `${REPO_URL}#readme`;
export const ISSUES_URL = `${REPO_URL}/issues`;

const EMPTY_PATCH: T.SettingsPatch = {
  defaultLinkMode: null,
  defaultDescriptionBudgetChars: null,
  defaultExecutorId: null,
  defaultGraderId: null,
  extraScanDirs: null,
  githubToken: null,
  theme: null,
  overlapThreshold: null,
  autoUpdateCheck: null,
};

/** A full patch with every untouched field set to null (= leave unchanged). */
export function settingsPatch(partial: Partial<T.SettingsPatch>): T.SettingsPatch {
  return { ...EMPTY_PATCH, ...partial };
}

/** Mutation that applies a partial settings patch, updates the cache and toasts. */
export function useSettingsPatch() {
  const qc = useQueryClient();
  return useApiMutation(
    ({ patch }: { patch: Partial<T.SettingsPatch>; message?: string }) => api.updateSettings(settingsPatch(patch)),
    [keys.dashboard, keys.targets, keys.installedTargets],
    (settings, { message }) => {
      qc.setQueryData(keys.settings, settings);
      toast.success(message ?? "Saved");
    },
  );
}

export const PROVIDER_KINDS: T.ProviderKind[] = ["claude_code", "codex", "anthropic", "openai_compatible", "scripted"];

export const PROVIDER_KIND_META: Record<
  T.ProviderKind,
  {
    label: string;
    description: string;
    family: "cli" | "http" | "script";
    defaultCommand: string | null;
    defaultBaseUrl: string | null;
    defaultModel: string | null;
    defaultRoles: T.ProviderRole[];
    envVar: string | null;
  }
> = {
  claude_code: {
    label: "Claude Code CLI",
    description: "Runs tasks through the local claude command with your existing login. Can execute, grade and recommend.",
    family: "cli",
    defaultCommand: "claude",
    defaultBaseUrl: null,
    defaultModel: null,
    defaultRoles: ["executor", "grader", "recommender"],
    envVar: null,
  },
  codex: {
    label: "Codex CLI",
    description: "Runs tasks through the local codex command. Can execute, grade and recommend.",
    family: "cli",
    defaultCommand: "codex",
    defaultBaseUrl: null,
    defaultModel: null,
    defaultRoles: ["executor", "grader", "recommender"],
    envVar: null,
  },
  anthropic: {
    label: "Anthropic API",
    description: "Direct Messages API calls. Grades results and re-ranks recommendations; does not run agent tasks.",
    family: "http",
    defaultCommand: null,
    defaultBaseUrl: "https://api.anthropic.com",
    defaultModel: "claude-sonnet-4-5",
    defaultRoles: ["grader", "recommender"],
    envVar: "ANTHROPIC_API_KEY",
  },
  openai_compatible: {
    label: "OpenAI-compatible API",
    description: "Any chat-completions endpoint: OpenAI, OpenRouter, Ollama, LM Studio, vLLM. Grades and recommends.",
    family: "http",
    defaultCommand: null,
    defaultBaseUrl: "https://api.openai.com/v1",
    defaultModel: null,
    defaultRoles: ["grader", "recommender"],
    envVar: "OPENAI_API_KEY",
  },
  scripted: {
    label: "Scripted",
    description: "Your own executable: it receives the task on stdin inside the sandbox and writes the final answer to stdout. For custom agents and CI.",
    family: "script",
    defaultCommand: null,
    defaultBaseUrl: null,
    defaultModel: null,
    defaultRoles: ["executor"],
    envVar: null,
  },
};

export const BASE_URL_PRESETS: { label: string; url: string }[] = [
  { label: "OpenAI", url: "https://api.openai.com/v1" },
  { label: "OpenRouter", url: "https://openrouter.ai/api/v1" },
  { label: "Ollama", url: "http://localhost:11434/v1" },
  { label: "LM Studio", url: "http://localhost:1234/v1" },
];

export const ROLES: T.ProviderRole[] = ["executor", "grader", "recommender"];
export const ROLE_META: Record<T.ProviderRole, { label: string; description: string }> = {
  executor: { label: "Executor", description: "Performs eval tasks in a sandbox" },
  grader: { label: "Grader", description: "Judges expectations with cited evidence" },
  recommender: { label: "Recommender", description: "Re-ranks skills for a task" },
};

/** Parse a quoted command line into arguments. */
export function splitArgs(text: string): string[] {
  const out: string[] = [];
  const re = /"([^"]*)"|'([^']*)'|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) out.push(m[1] ?? m[2] ?? m[3]);
  return out;
}

export function joinArgs(args: string[]): string {
  return args.map((a) => (/\s/.test(a) ? `"${a}"` : a)).join(" ");
}
