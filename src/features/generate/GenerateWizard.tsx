import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { useEffect } from "react";
import { useParams } from "react-router-dom";
import { t } from "../../lib/i18n";
import { demoEnabled } from "@/lib/demo";
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
  const demo = demoEnabled();
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
    <section data-testid="generate-wizard" className="mx-auto flex w-full max-w-3xl flex-col gap-4">
      <header>
        <h1 className="font-mono text-[22px] font-semibold tracking-tight">{t("generate.title")}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{sessionId}</p>
      </header>
      <ol className="inline-flex w-fit flex-wrap rounded-lg bg-muted p-0.5 text-sm">
        {STEPS.map((step) => (
          <li
            key={step}
            data-testid={`generate-step-${step}`}
            aria-current={wizard.step === step ? "step" : undefined}
            className={wizard.step === step ? "rounded-md bg-background px-3 py-1 font-medium shadow-sm" : "px-3 py-1 text-muted-foreground"}
          >
            {stepLabel[step]}
          </li>
        ))}
      </ol>
      <Card className="gap-3 rounded-lg border-border px-4 py-4 shadow-none">
      {wizard.step === "goal" ? (
        <form
          className="space-y-2"
          onSubmit={(event) => {
            event.preventDefault();
            continueWizard();
          }}
        >
          <label className="block text-sm" htmlFor="generate-goal">{t("generate.goal")}</label>
          <textarea id="generate-goal" className="min-h-24 w-full rounded-md border border-input bg-background p-2 text-sm" value={wizard.goal} onChange={(event) => setGoal(event.target.value)} />
          <Button type="submit" size="sm">{t("generate.continue")}</Button>
        </form>
      ) : null}
      {wizard.step === "interview" ? (
        <div className="space-y-2">
          {wizard.questions.length === 0 ? <p>{t("generate.interview.empty")}</p> : wizard.questions.map((question) => <p key={question}>{question}</p>)}
          <Button type="button" variant="outline" size="sm" onClick={() => acceptDefaults()}>{t("generate.defaults")}</Button>
        </div>
      ) : null}
      {wizard.step === "drafting" ? (
        <div className="space-y-2">
          <p className="text-sm">{stepLabel.drafting}</p>
          <Button type="button" variant="outline" size="sm" onClick={() => continueWizard()}>{t("generate.continue")}</Button>
        </div>
      ) : null}
      {wizard.step === "review" || wizard.step === "save" ? (
        <div className="space-y-2">
          {demo ? <p data-testid="generate-problems">{t("generate.problems")}: 0</p> : wizard.questions.length === 0 ? <p data-testid="generate-empty">{t("generate.interview.empty")}</p> : null}
          {wizard.questions.map((question) => <p key={question}>{question}</p>)}
          <p className="text-sm">{t("generate.safety")}</p>
          <pre data-testid="generate-diff" className="rounded-md border border-border p-2 text-xs">{wizard.diffOpened ? "+++ SKILL.md" : t("generate.diff")}</pre>
          <Button type="button" variant="outline" size="sm" onClick={() => openDiff()}>{t("generate.openDiff")}</Button>
          <p data-testid="generate-inspector" className="text-sm">{t("generate.inspector")}</p>
          {wizard.revised ? <p data-testid="generate-evidence">{t("generate.evidence")}</p> : null}
          <label className="block text-sm" htmlFor="generate-feedback">{t("generate.feedback")}</label>
          <textarea id="generate-feedback" className="w-full rounded-md border border-input p-2 text-sm" />
          <Button type="button" variant="outline" size="sm" onClick={() => reviseDraft()}>{t("generate.revise")}</Button>
          <Button type="button" variant="default" size="sm" disabled={!wizard.diffOpened} onClick={() => saveDraft()}>{t("generate.save")}</Button>
          {wizard.saved ? <p data-testid="generate-saved">{t("generate.step.save")}</p> : null}
        </div>
      ) : null}
      </Card>
      {wizard.discardAsk ? (
        <div role="dialog" aria-label={t("generate.discard.title")} className="rounded-md border border-border p-3">
          <p>{t("generate.discard.title")}</p>
          <Button type="button" variant="outline" size="sm" className="mr-2" onClick={() => confirmDiscard()}>{t("generate.discard.confirm")}</Button>
          <Button type="button" variant="outline" size="sm" onClick={() => stayInWizard()}>{t("generate.discard.stay")}</Button>
        </div>
      ) : null}
    </section>
  );
}
