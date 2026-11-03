import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { api } from "@/lib/api/client";
import { RunPage } from "./RunPage";
import { renderWithProviders } from "./test-utils";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("RunPage", () => {
  it("renders a finished benchmark: verdict, arm stats, expectation matrix, trigger table and notes", async () => {
    renderWithProviders(<RunPage />, { route: "/runs/run-1", path: "/runs/:runId" });

    // Verdict card
    expect(await screen.findByText("Helps a lot", {}, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.getByText(/raised the pass rate from 50% to 92%/)).toBeInTheDocument();
    expect(screen.getByText(/p = 0\.012/)).toBeInTheDocument();
    expect(screen.getByText("Done")).toBeInTheDocument();

    // Arm comparison table: both arms, baseline marked, mean ± sd pass rates
    expect(screen.getAllByText("With skill").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Without skill").length).toBeGreaterThan(0);
    expect(screen.getByText("baseline")).toBeInTheDocument();
    expect(screen.getByText("92% ± 8")).toBeInTheDocument();
    expect(screen.getByText("50% ± 14")).toBeInTheDocument();

    // Expectation matrix: one row per expectation, grouped by case, with pattern badges.
    const matrix = screen.getByRole("table", { name: "Expectation matrix" });
    expect(within(matrix).getByText("Totals match the PDF (1,284.50)")).toBeInTheDocument();
    expect(within(matrix).getByText("Page order is preserved")).toBeInTheDocument();
    expect(within(matrix).getAllByText("Only with skill")).toHaveLength(3);
    expect(within(matrix).getAllByText("Always passes")).toHaveLength(2);
    expect(within(matrix).getByText("Flaky")).toBeInTheDocument();
    expect(within(matrix).getAllByText("merge-two-pdfs")).toHaveLength(1);

    // Trigger results
    const trigger = screen.getByRole("table", { name: "Trigger results" });
    expect(within(trigger).getByText("Write a haiku about autumn")).toBeInTheDocument();
    expect(within(trigger).getAllByText("correct")).toHaveLength(5);
    expect(within(trigger).getByText("missed")).toBeInTheDocument();
    expect(screen.getByText("Precision")).toBeInTheDocument();

    // Notes
    expect(screen.getByText(/do not measure the skill/)).toBeInTheDocument();

    // Case results: 3 cases × 2 arms × 2 repeats
    expect(screen.getByText("Case results (12)")).toBeInTheDocument();
  });

  it("expands a case result to show graded expectations and opens the transcript", async () => {
    const user = userEvent.setup();
    renderWithProviders(<RunPage />, { route: "/runs/run-1", path: "/runs/:runId" });
    await screen.findByText("Case results (12)", {}, { timeout: 3000 });
    const rows = screen.getAllByRole("button", { name: /^extract-invoice-tables/ });
    expect(rows).toHaveLength(4);
    await user.click(rows[0]);
    expect(rows[0]).toHaveAttribute("aria-expanded", "true");
    expect(screen.getAllByText("Found in transcript step 3: output verified").length).toBeGreaterThan(0);
    expect(screen.getByText("Final answer (excerpt)")).toBeInTheDocument();

    const spy = vi.spyOn(api, "caseTranscript");
    await user.click(screen.getByRole("button", { name: /transcript/i }));
    await waitFor(() => expect(spy).toHaveBeenCalledWith("run-1", expect.stringMatching(/^run-1-/)));
    expect(await screen.findByText(/I'll read the skill instructions first/)).toBeInTheDocument();
  });

  it("shows a cancel control and live progress for an active run", async () => {
    const config = await api.defaultRunConfig((await api.runs()).find((r) => r.skillName === "pdf")!.itemId!);
    const run = await api.startRun(config);
    renderWithProviders(<RunPage />, { route: `/runs/${run.id}`, path: "/runs/:runId" });
    expect(await screen.findByRole("button", { name: /cancel/i }, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.getByText(/Case results stream in below/)).toBeInTheDocument();
    await api.cancelRun(run.id);
  });
});
