import { cleanup, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { installIpc, renderAt } from "@/test/render-app";

const screens = [
  ["/", "dashboard"],
  ["/my-skills", "library"],
  ["/agents", "agents"],
  ["/install", "install"],
  ["/backup", "backup"],
  ["/project/alpha", "project"],
  ["/settings", "settings"],
  ["/settings/endpoints", "endpoints"],
  ["/global-workspace", "global-workspace"],
  ["/effectiveness", "effectiveness"],
  ["/compare/fixture", "compare"],
  ["/runs/run-fixture", "results"],
  ["/skills/skill-research/tests", "suite"],
  ["/skills/skill-research/history", "history"],
];

describe("loading states", () => {
  afterEach(() => {
    cleanup();
    window.history.pushState({}, "", "/");
  });

  it.each(screens)("%s renders a loading state", async (path, testId) => {
    window.history.pushState({}, "", `${path}?state=loading`);
    installIpc();
    renderAt(`${path}?state=loading`);
    expect(await screen.findByTestId("loading-state")).toBeInTheDocument();
    expect(screen.getByTestId(testId)).toBeInTheDocument();
  });
});
