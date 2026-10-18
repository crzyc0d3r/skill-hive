import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetGenerate } from "@/features/generate/store";
import { ipcLog, renderShell as renderAt, setEffectivenessPayload } from "@/test/render-app";

const MEASURED = {
  cells: [{ id: "with-live", label: "with_skill 1", passRate: 1 }],
  steps: [{ id: 4, tool: "Write", useTs: 1736000007, resultTs: 1736000008 }],
  with_rate: 1,
  without_rate: 0,
  exportCsv: "configuration,pass_rate\nwith_skill,1\n",
  history: {
    iterations: [
      {
        version: "with_skill",
        parent: "without_skill",
        expectation_pass_rate: 1,
        grading_result: "won",
        is_current_best: true,
      },
    ],
  },
  outputA: "PINECONE",
  outputB: "plain",
  sealed: "with_skill",
};

describe("effectiveness screens", () => {
  afterEach(() => {
    cleanup();
    resetGenerate();
    localStorage.clear();
    setEffectivenessPayload(null);
  });

  it("measures against the typed endpoint and shows the stored run", async () => {
    const user = userEvent.setup();
    renderAt("/skills/skill-research/tests");
    const endpoint = await screen.findByLabelText("Endpoint");
    await user.clear(endpoint);
    await user.type(endpoint, "http://127.0.0.1:4010");
    await user.click(screen.getByRole("button", { name: "Measure" }));
    expect(await screen.findByTestId("suite-measured")).toBeInTheDocument();
    expect(
      ipcLog.some((entry) => entry.cmd === "run_effectiveness_suite" && entry.args.baseUrl === "http://127.0.0.1:4010"),
    ).toBe(true);

    cleanup();
    setEffectivenessPayload(JSON.stringify(MEASURED));
    renderAt("/runs/measured");
    expect(await screen.findByText(/with_skill 1/)).toBeInTheDocument();
    expect(screen.getByTestId("results-summary")).toHaveTextContent("1 / 0");
    await user.click(screen.getByRole("button", { name: "Export" }));
    expect(screen.getByTestId("results-export")).toHaveTextContent("with_skill,1");

    cleanup();
    setEffectivenessPayload(JSON.stringify(MEASURED));
    renderAt("/compare/measured");
    expect(await screen.findByText("Output A")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "with_skill" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Record verdict" }));
    expect(screen.getByRole("heading", { name: "with_skill" })).toBeInTheDocument();

    cleanup();
    setEffectivenessPayload(JSON.stringify(MEASURED));
    renderAt("/skills/skill-research/history");
    expect(await screen.findByTestId("history-with_skill")).toHaveTextContent("1");

    cleanup();
    renderAt("/generate/measured");
    await user.type(await screen.findByLabelText("Goal"), "classify the dock sample");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByText("No interview questions yet.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Ask the model" }));
    expect(await screen.findByText("What output format?")).toBeInTheDocument();
    expect(
      ipcLog.some((entry) => entry.cmd === "interview_generation" && String(entry.args.baseUrl).startsWith("http://127.0.0.1")),
    ).toBe(true);
  });
});
