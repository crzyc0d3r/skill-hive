import type * as T from "@/lib/api/bindings";

export const CANDIDATE_STATUS_META: Record<T.CandidateStatus, { label: string; tone: "accent" | "neutral" | "warn" | "ok" | "danger"; hint: string }> = {
  new: { label: "New", tone: "accent", hint: "Not in the library yet" },
  in_library: { label: "In library", tone: "neutral", hint: "Identical to an item already in the library" },
  modified: { label: "Modified", tone: "warn", hint: "An item with this name is in the library, but this copy differs" },
  managed: { label: "Managed", tone: "ok", hint: "Deployed here by Skill Hive from the library" },
  invalid: { label: "Invalid", tone: "danger", hint: "Fails validation and cannot be imported" },
};

/** Only valid candidates can be imported. */
export function isImportable(c: T.Candidate): boolean {
  return c.status !== "invalid";
}

/** The default selection for a fresh scan or preview: everything that is new. */
export function defaultSelection(candidates: T.Candidate[]): string[] {
  return candidates.filter((c) => c.status === "new").map((c) => c.id);
}

export function toggleId(ids: string[], id: string): string[] {
  return ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id];
}

export interface CandidateGroup {
  key: string;
  label: string;
  hint?: string;
}
