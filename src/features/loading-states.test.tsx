import { cleanup, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { installIpc, renderAt } from "@/test/render-app";

const screens = [
  ["/", "dashboard", "Good day", "dashboard"],
  ["/my-skills", "library", "Library", "library"],
  ["/agents", "agents", "Agents", "agents"],
  ["/install", "install", "Install Skills", "install"],
  ["/backup", "backup", "Backup", "backup"],
  ["/project/alpha", "project", "Project", "project"],
  ["/settings", "settings", "Settings", "settings"],
  ["/settings/endpoints", "endpoints", "Endpoints", "endpoints"],
  ["/global-workspace", "global-workspace", "Global Workspace", "workspace"],
  ["/effectiveness", "effectiveness", "Effectiveness", "effectiveness"],
  ["/compare/fixture", "compare", "Compare", "compare"],
  ["/runs/run-fixture", "results", "Run results", "results"],
  ["/skills/skill-research/tests", "suite", "Test suite", "suite"],
  ["/skills/skill-research/history", "history", "History", "history"],
] as const;

describe("loading states", () => {
  afterEach(() => {
    cleanup();
    window.history.pushState({}, "", "/");
  });

  it.each(screens)("%s renders a loading state", async (path, testId, title, skeleton) => {
    window.history.pushState({}, "", `${path}?state=loading`);
    installIpc();
    renderAt(`${path}?state=loading`);
    const region = await screen.findByTestId("loading-state");
    expect(region).toHaveAttribute("data-skeleton", skeleton);
    expect(screen.getByTestId(testId)).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: title })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Loading" })).not.toBeInTheDocument();
  });
});
