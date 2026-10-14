import { useSyncExternalStore } from "react";
import { persistEffect } from "@/lib/ipc/persist";
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
  verdict: string;
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
  verdict: "",
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

export function acceptDefaults(): void {
  wizard = { ...wizard, answers: "accepted defaults", step: "drafting" };
  publish();
}

export function continueWizard(): void {
  const order: WizardStep[] = ["goal", "interview", "drafting", "review", "save"];
  const index = order.indexOf(wizard.step);
  const next = order[Math.min(index + 1, order.length - 1)];
  wizard = { ...wizard, step: wizard.step === "drafting" ? "review" : next };
  publish();
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
  persistEffect("generate", "draft", wizard.goal);
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

export function optimizeDescription(): void {
  wizard = { ...wizard, optimized: true };
  publish();
}

export function applyOptimized(): void {
  if (!wizard.optimized) return;
  wizard = { ...wizard, applied: true };
  publish();
  persistEffect("suite", "optimized", wizard.goal);
}

export function recordVerdict(verdict: string): void {
  wizard = { ...wizard, verdict };
  publish();
  persistEffect("compare", "verdict", verdict);
}

export function resetGenerate(): void {
  wizard = { ...wizardEmpty };
  seedCache = "";
  publish();
}
