import { useSyncExternalStore } from "react";
import type { ProjectSkill } from "./model";
import { worstDot, type DotColor } from "./model";

let skills: ProjectSkill[] = [];
const listeners = new Set<() => void>();

function emit() {
  for (const listener of listeners) listener();
}

export function setProjectSkills(next: ProjectSkill[]) {
  skills = next;
  emit();
}

export function projectSkills(): ProjectSkill[] {
  return skills;
}

export function useProjectDot(): DotColor {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => (skills.length ? worstDot(skills) : "grey"),
  );
}
