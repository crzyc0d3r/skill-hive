import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { SAMPLE_REPORT } from "@/features/effectiveness/model";
import { setEffectivenessReport } from "@/test/render-app";
import { renderShell as renderAt } from "@/test/render-app";

describe("effectiveness", () => {
  afterEach(() => {
    cleanup();
    localStorage.clear();
    setEffectivenessReport("");
  });

  it("shows the letter first and the stolen query after one step", async () => {
    localStorage.setItem("skill-hive-demo", "1");
    const user = userEvent.setup();
    renderAt("/effectiveness");
    expect(await screen.findByTestId("effect-grade")).toHaveTextContent("B");
    expect(screen.getByTestId("effect-verdict")).toHaveTextContent("Other skills are answering");
    expect(screen.queryByTestId("effect-map")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Show the charts" }));
    expect(screen.getByText("What this means: the weakest axis is Holds up.")).toBeInTheDocument();
    expect(screen.getByRole("table", { name: "Effectiveness radar" })).toBeInTheDocument();
    const map = screen.getByTestId("effect-map");
    map.focus();
    await user.keyboard("{ArrowRight}{Enter}");
    expect(screen.getByTestId("effect-stolen")).toHaveTextContent("hijack-target the pine question");
    expect(screen.getByTestId("effect-stolen")).toHaveTextContent("triggers/library-q2.jsonl");
    expect(screen.getByTestId("board-pine")).toHaveTextContent("0.5");
    expect(screen.getByRole("table", { name: "Conflict matrix" })).toBeInTheDocument();
    expect(screen.getByTestId("effect-bisect")).toHaveTextContent("culprit");
    expect(screen.getByText("What this means: installing culprit changed an existing answer.")).toBeInTheDocument();
  });

  it("reads a measured report from the command", async () => {
    setEffectivenessReport(JSON.stringify(SAMPLE_REPORT));
    const user = userEvent.setup();
    renderAt("/effectiveness");
    expect(await screen.findByTestId("effect-grade")).toHaveTextContent("B");
    await user.click(screen.getByRole("button", { name: "Show the charts" }));
    expect(screen.getByText("What this means: the thickest arrow is culprit -> pine.")).toBeInTheDocument();
  });
});
