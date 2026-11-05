import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type * as T from "@/lib/api/bindings";
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

    // Activity: metric rows × arm columns, with the skill-opened row only filled for arms that have a skill.
    const activity = screen.getByRole("table", { name: "Activity" });
    expect(within(activity).getByText("With skill")).toBeInTheDocument();
    expect(within(activity).getByText("Without skill")).toBeInTheDocument();
    expect(within(activity).getAllByText("6 traced")).toHaveLength(2);
    for (const row of ["Model turns", "Tool calls", "File reads", "Searches", "Writes", "Commands", "Tool errors", "Distinct files read", "Opened the skill", "Thinking tokens", "IO share", "Cache reads", "Model time", "Tool time"]) {
      expect(within(activity).getByText(row)).toBeInTheDocument();
    }
    const opened = within(activity).getByText("Opened the skill").closest("tr")!;
    const cells = within(opened).getAllByRole("cell");
    expect(cells[1]).toHaveTextContent("83%");
    expect(cells[2]).toHaveTextContent("—");
    const reads = within(within(activity).getByText("File reads").closest("tr")!).getAllByRole("cell");
    expect(reads[1]).toHaveTextContent("2.8 ± 0.4");
    expect(reads[2]).toHaveTextContent("2.0");
    expect(screen.getByRole("img", { name: "Output tokens per arm by use" })).toBeInTheDocument();
    expect(screen.getByText("Tool arguments")).toBeInTheDocument();

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

    // Activity chips, files and the collapsed timeline for the with_skill run.
    const detail = within(rows[0].parentElement!);
    expect(detail.getByText("Grader spent 2.3k tokens")).toBeInTheDocument();
    const chip = (label: string) => detail.getByText(label).previousElementSibling!;
    expect(chip("tool calls")).toHaveTextContent("7");
    expect(chip("reads")).toHaveTextContent("3");
    expect(chip("searches")).toHaveTextContent("1");
    expect(chip("writes")).toHaveTextContent("1");
    expect(chip("commands")).toHaveTextContent("2");
    expect(chip("opened the skill")).toHaveTextContent("yes");
    expect(chip("IO share")).toHaveTextContent("73%");
    expect(screen.getByText(".claude/skills/pdf/SKILL.md")).toBeInTheDocument();
    expect(screen.getByText("extract.py")).toBeInTheDocument();
    expect(screen.queryByText(/read the skill instructions in SKILL\.md/)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Show timeline \(18 steps\)/ }));
    const thinking = screen.getByText(/read the skill instructions in SKILL\.md/);
    expect(thinking).toHaveClass("italic");
    expect(screen.getAllByText("thinking").length).toBeGreaterThan(1);
    expect(screen.getAllByText("read").length).toBeGreaterThan(1);
    expect(screen.getByText("python3 extract.py")).toBeInTheDocument();
    expect(screen.getAllByText("Bash").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: /Hide timeline/ })).toBeInTheDocument();

    const spy = vi.spyOn(api, "caseTranscript");
    await user.click(screen.getByRole("button", { name: /transcript/i }));
    await waitFor(() => expect(spy).toHaveBeenCalledWith("run-1", expect.stringMatching(/^run-1-/)));
    expect(await screen.findByText(/I'll read the skill instructions first/)).toBeInTheDocument();
  });

  it("marks a tool error in the timeline and says when a skill run never opened the skill", async () => {
    const user = userEvent.setup();
    renderWithProviders(<RunPage />, { route: "/runs/run-1", path: "/runs/:runId" });
    await screen.findByText("Case results (12)", {}, { timeout: 3000 });

    // merge-two-pdfs, with_skill, repeat 1 hit a ModuleNotFoundError and retried.
    const merge = screen.getAllByRole("button", { name: /^merge-two-pdfs/ });
    await user.click(merge[0]);
    expect(screen.getByText("tool errors").previousElementSibling).toHaveTextContent("1");
    await user.click(screen.getByRole("button", { name: /Show timeline/ }));
    expect(screen.getByText("error")).toBeInTheDocument();
    expect(screen.getByText(/ModuleNotFoundError/)).toHaveClass("text-danger");

    // fill-form, with_skill, repeat 2 never read SKILL.md.
    const fill = screen.getAllByRole("button", { name: /^fill-form/ });
    await user.click(fill[1]);
    const chips = screen.getAllByText("opened the skill").map((el) => el.previousElementSibling!.textContent);
    expect(chips).toEqual(["yes", "no"]);
    // The baseline arm has no skill to open, so its detail shows no such chip.
    await user.click(fill[2]);
    expect(screen.getAllByText("opened the skill")).toHaveLength(2);
  });

  it("handles a run recorded before traces existed and a sparse trace", async () => {
    const user = userEvent.setup();
    const original = await api.runDetail("run-1");
    const detail = structuredClone(original);
    // An old run: no activity on any arm, no trace and no grader tokens on any case.
    for (const a of detail.run.summary!.arms) a.activity = null;
    for (const r of detail.results) {
      r.trace = null;
      r.graderInputTokens = 0;
      r.graderOutputTokens = 0;
    }
    // One sparse trace: untimed steps, an unknown category, no tool name, a capped timeline.
    const sparse = detail.results[1];
    sparse.trace = {
      ...original.results[1].trace!,
      steps: [
        { atMs: null, kind: "tool_use", tool: null, category: "mystery" as T.ToolCategory, summary: "", chars: 0, isError: false },
        { atMs: null, kind: "tool_result", tool: null, category: null, summary: "ok", chars: 2, isError: false },
      ],
      stepsTruncated: 7,
      filesRead: ["a.txt"],
      filesReadCount: 9,
    };
    vi.spyOn(api, "runDetail").mockResolvedValue(detail);
    renderWithProviders(<RunPage />, { route: "/runs/run-1", path: "/runs/:runId" });
    await screen.findByText("Case results (12)", {}, { timeout: 3000 });
    expect(screen.queryByRole("table", { name: "Activity" })).not.toBeInTheDocument();
    expect(screen.queryByRole("img", { name: "Output tokens per arm by use" })).not.toBeInTheDocument();

    const rows = screen.getAllByRole("button", { name: /^extract-invoice-tables/ });
    await user.click(rows[0]);
    expect(screen.getByText(/No activity trace for this run/)).toBeInTheDocument();
    expect(screen.queryByText(/Grader spent/)).not.toBeInTheDocument();

    await user.click(rows[1]);
    expect(screen.getByText("8 more not listed")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Show timeline \(2 steps\)/ }));
    expect(screen.getByText("other")).toBeInTheDocument();
    expect(screen.getByText("(empty)")).toBeInTheDocument();
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
    expect(screen.getByText(/7 more steps were not recorded/)).toBeInTheDocument();
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
