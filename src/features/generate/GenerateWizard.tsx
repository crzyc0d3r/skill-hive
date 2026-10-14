import { useEffect } from "react";
import { useParams } from "react-router-dom";
import { t } from "../../lib/i18n";
import {
  acceptDefaults,
  askDiscard,
  confirmDiscard,
  continueWizard,
  openDiff,
  reviseDraft,
  saveDraft,
  setGoal,
  stayInWizard,
  useGenerate,
} from "@/features/generate/store";

const STEPS = ["goal", "interview", "drafting", "review", "save"] as const;

export function GenerateWizard() {
  const { sessionId } = useParams();
  const { wizard } = useGenerate();
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") askDiscard();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const stepLabel = {
    goal: t("generate.step.goal"),
    interview: t("generate.step.interview"),
    drafting: t("generate.step.drafting"),
    review: t("generate.step.review"),
    save: t("generate.step.save"),
  };
  return (
    <section data-testid="generate-wizard" className="mx-auto max-w-3xl space-y-3">
      <h1 className="font-mono text-xl font-semibold">{t("generate.title")}</h1>
      <p className="text-sm text-muted-foreground">{sessionId}</p>
      <ol className="flex flex-wrap gap-2 text-sm">
        {STEPS.map((step) => (
          <li key={step} data-testid={`generate-step-${step}`} aria-current={wizard.step === step ? "step" : undefined}>
            {stepLabel[step]}
          </li>
        ))}
      </ol>
      {wizard.step === "goal" ? (
        <form
          className="space-y-2"
          onSubmit={(event) => {
            event.preventDefault();
            continueWizard();
          }}
        >
          <label className="block text-sm" htmlFor="generate-goal">{t("generate.goal")}</label>
          <textarea id="generate-goal" className="w-full rounded-md border border-input p-2 text-sm" value={wizard.goal} onChange={(event) => setGoal(event.target.value)} />
          <button type="submit" className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground">{t("generate.continue")}</button>
        </form>
      ) : null}
      {wizard.step === "interview" ? (
        <div className="space-y-2">
          <button type="button" className="rounded-md border border-border px-3 py-1 text-sm" onClick={() => acceptDefaults()}>{t("generate.defaults")}</button>
        </div>
      ) : null}
      {wizard.step === "drafting" ? (
        <div className="space-y-2">
          <p className="text-sm">{stepLabel.drafting}</p>
          <button type="button" className="rounded-md border border-border px-3 py-1 text-sm" onClick={() => continueWizard()}>{t("generate.continue")}</button>
        </div>
      ) : null}
      {wizard.step === "review" || wizard.step === "save" ? (
        <div className="space-y-2">
          <p data-testid="generate-problems">{t("generate.problems")}: 0</p>
          <p className="text-sm">{t("generate.safety")}</p>
          <pre data-testid="generate-diff" className="rounded-md border border-border p-2 text-xs">{wizard.diffOpened ? "+++ SKILL.md" : t("generate.diff")}</pre>
          <button type="button" className="rounded-md border border-border px-3 py-1 text-sm" onClick={() => openDiff()}>{t("generate.openDiff")}</button>
          <p data-testid="generate-inspector" className="text-sm">{t("generate.inspector")}</p>
          {wizard.revised ? <p data-testid="generate-evidence">{t("generate.evidence")}</p> : null}
          <label className="block text-sm" htmlFor="generate-feedback">{t("generate.feedback")}</label>
          <textarea id="generate-feedback" className="w-full rounded-md border border-input p-2 text-sm" />
          <button type="button" className="rounded-md border border-border px-3 py-1 text-sm" onClick={() => reviseDraft()}>{t("generate.revise")}</button>
          <button type="button" className="rounded-md bg-primary px-3 py-1 text-sm text-primary-foreground disabled:opacity-40" disabled={!wizard.diffOpened} onClick={() => saveDraft()}>{t("generate.save")}</button>
          {wizard.saved ? <p data-testid="generate-saved">{t("generate.step.save")}</p> : null}
        </div>
      ) : null}
      {wizard.discardAsk ? (
        <div role="dialog" aria-label={t("generate.discard.title")} className="rounded-md border border-border p-3">
          <p>{t("generate.discard.title")}</p>
          <button type="button" className="mr-2 rounded-md border border-border px-2 py-1 text-sm" onClick={() => confirmDiscard()}>{t("generate.discard.confirm")}</button>
          <button type="button" className="rounded-md border border-border px-2 py-1 text-sm" onClick={() => stayInWizard()}>{t("generate.discard.stay")}</button>
        </div>
      ) : null}
    </section>
  );
}
