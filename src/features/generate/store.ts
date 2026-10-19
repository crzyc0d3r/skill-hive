import { useSyncExternalStore } from "react";
import { applyOptimizedDescription, blindCompareLast, interviewForGoal, optimizeSkillDescription, recordCompareVerdict, saveGeneratedDraft } from "@/lib/ipc/actions";
import { demoEnabled } from "@/lib/demo";
import { EMPTY_SEED, parseSeed, type GenerateSeed } from "./model";

export type WizardStep = "goal" | "interview" | "drafting" | "review" | "save";

interface WizardState {
  step: WizardStep;
  goal: string;
  answers: string;
  diffOpened: boolean;
  revised: boolean;
  saved: boolean;
  discardAsk: boolean;
  optimized: boolean;
  applied: boolean;
  optimizePath: string;
  questions: string[];
  verdict: string;
  compareText: string;
}

const wizardEmpty: WizardState = {
  step: "goal",
  goal: "",
  answers: "",
  diffOpened: false,
  revised: false,
  saved: false,
  discardAsk: false,
  optimized: false,
  applied: false,
  optimizePath: "",
  questions: [],
  verdict: "",
  compareText: "",
};

let wizard = wizardEmpty;
let seedCache = "";
let seed = EMPTY_SEED;
const listeners = new Set<() => void>();

function emit(): void {
  listeners.forEach((listener) => listener());
}

function readSeed(): GenerateSeed {
  const raw = typeof localStorage === "undefined" ? "" : localStorage.getItem("skill-hive-generate") ?? "";
  if (raw !== seedCache) {
    seedCache = raw;
    seed = parseSeed(raw);
  }
  return seed;
}

export interface GenerateSnapshot {
  seed: GenerateSeed;
  wizard: WizardState;
}

let snapshot: GenerateSnapshot = { seed: EMPTY_SEED, wizard };

function publish(): void {
  snapshot = { seed: readSeed(), wizard };
  emit();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function getSnapshot(): GenerateSnapshot {
  const nextSeed = readSeed();
  if (snapshot.seed !== nextSeed || snapshot.wizard !== wizard) {
    snapshot = { seed: nextSeed, wizard };
  }
  return snapshot;
}

export function useGenerate(): GenerateSnapshot {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export function refreshGenerate(): void {
  seedCache = "";
  publish();
}

export function setGoal(goal: string): void {
  wizard = { ...wizard, goal };
  publish();
}

function takeQuestions(raw: unknown) {
  if (typeof raw !== "string") return;
  try {
    const parsed = JSON.parse(raw) as unknown;
    const list = Array.isArray(parsed)
      ? parsed
      : parsed && typeof parsed === "object" && "questions" in parsed && Array.isArray((parsed as { questions: unknown }).questions)
        ? (parsed as { questions: unknown[] }).questions
        : null;
    if (list && list.every((item) => typeof item === "string")) {
      wizard = { ...wizard, questions: list as string[] };
      publish();
    }
  } catch {
    /* a saved path is not an interview list */
  }
}

export function acceptDefaults(): void {
  const goal = wizard.goal || "interview";
  wizard = { ...wizard, answers: "accepted defaults", step: "drafting" };
  publish();
  void interviewForGoal(goal).then(takeQuestions);
  void saveGeneratedDraft(goal).then(takeQuestions);
}

export function continueWizard(): void {
  const order: WizardStep[] = ["goal", "interview", "drafting", "review", "save"];
  const index = order.indexOf(wizard.step);
  const leavingGoal = wizard.step === "goal";
  const goal = wizard.goal;
  const next = order[Math.min(index + 1, order.length - 1)];
  wizard = { ...wizard, step: wizard.step === "drafting" ? "review" : next };
  publish();
  if (leavingGoal) void interviewForGoal(goal).then(takeQuestions);
}

export function openDiff(): void {
  wizard = { ...wizard, diffOpened: true };
  publish();
}

export function reviseDraft(): void {
  wizard = { ...wizard, revised: true };
  publish();
}

export function saveDraft(): void {
  if (!wizard.diffOpened) return;
  wizard = { ...wizard, saved: true, step: "save" };
  publish();
  void saveGeneratedDraft(wizard.goal);
}

export function askDiscard(): void {
  if (wizard.answers || wizard.goal) wizard = { ...wizard, discardAsk: true };
  publish();
}

export function stayInWizard(): void {
  wizard = { ...wizard, discardAsk: false };
  publish();
}

export function confirmDiscard(): void {
  wizard = { ...wizardEmpty };
  publish();
}

export function optimizeDescription(skillPath = ""): void {
  wizard = { ...wizard, optimized: true };
  publish();
  void optimizeSkillDescription(skillPath || wizard.goal || "skill");
}

export function applyOptimized(): void {
  if (!wizard.optimized) return;
  wizard = { ...wizard, applied: true };
  publish();
  void applyOptimizedDescription(wizard.goal).then((value) => {
    if (typeof value === "string" && value) {
      wizard = { ...wizard, optimizePath: value };
      publish();
    }
  });
}

export function recordVerdict(verdict: string): void {
  wizard = { ...wizard, verdict };
  publish();
  const record = () => {
    void recordCompareVerdict("comparison", verdict).then((value) => {
      if (typeof value === "string" && value) {
        wizard = { ...wizard, compareText: value };
        publish();
      }
    });
  };
  if (demoEnabled()) {
    record();
    return;
  }
  void blindCompareLast(wizard.goal || "comparison", "Output A", "Output B").finally(record);
}

export function resetGenerate(): void {
  wizard = { ...wizardEmpty };
  seedCache = "";
  publish();
}
