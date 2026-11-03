import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { api } from "@/lib/api/client";
import type * as T from "@/lib/api/bindings";
import { EvalsPage } from "./EvalsPage";
import { renderWithProviders } from "./test-utils";

const SKILLS: T.LibraryQuery = { text: null, kinds: ["skill"], tags: [], sourceKinds: [], favoritesOnly: false, deployedTo: null, sort: "name", limit: null };

async function openSkill(name: string) {
  const items = await api.listItems(SKILLS);
  const item = items.find((i) => i.name === name)!;
  renderWithProviders(<EvalsPage />, { route: `/evals/${item.id}`, path: "/evals/:itemId" });
  return item;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("SuiteEditor", () => {
  it("tracks dirtiness, discards and saves edits through the API", async () => {
    const user = userEvent.setup();
    await openSkill("pdf");
    expect(await screen.findByDisplayValue("extract-invoice-tables", {}, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.getByText("All changes saved")).toBeInTheDocument();
    const save = screen.getByRole("button", { name: /save suite/i });
    const discard = screen.getByRole("button", { name: /discard/i });
    expect(save).toBeDisabled();
    expect(discard).toBeDisabled();

    // Edit the first (expanded) case's prompt → dirty.
    const prompt = screen.getByLabelText("Prompt");
    await user.clear(prompt);
    await user.paste("Pull every table out of invoice.pdf");
    expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
    expect(save).toBeEnabled();

    // Discard restores the stored suite.
    await user.click(discard);
    expect(screen.getByText("All changes saved")).toBeInTheDocument();
    expect(screen.getByLabelText("Prompt")).toHaveValue("Extract the line-item table from invoice.pdf into a CSV named invoice.csv with the same columns.");

    // Edit again, add a trigger query, save.
    await user.clear(screen.getByLabelText("Prompt"));
    await user.paste("Pull every table out of invoice.pdf");
    await user.click(screen.getByRole("button", { name: /add query/i }));
    const queries = screen.getAllByLabelText(/^Trigger query \d+$/);
    await user.click(queries[queries.length - 1]);
    await user.paste("OCR this scanned contract");
    const spy = vi.spyOn(api, "saveEvalSuite");
    await user.click(save);
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    const sent = spy.mock.calls[0][0];
    expect(sent.cases[0].prompt).toBe("Pull every table out of invoice.pdf");
    expect(sent.triggerCases.at(-1)).toEqual({ query: "OCR this scanned contract", shouldTrigger: true });
    expect(await screen.findByText("All changes saved")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /save suite/i })).toBeDisabled();
  }, 20_000);

  it("blocks saving an invalid case and reports why", async () => {
    const user = userEvent.setup();
    await openSkill("pdf");
    const nameInputs = await screen.findAllByLabelText("Case name", {}, { timeout: 3000 });
    await user.clear(nameInputs[0]);
    await user.type(nameInputs[0], "Bad Name!");
    expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /save suite/i })).toBeDisabled();
    expect(screen.getByText(/name must be a slug/i)).toBeInTheDocument();
  });

  it("offers to draft a suite for a skill without one and loads it into the editor", async () => {
    const user = userEvent.setup();
    await openSkill("docx");
    const draft = await screen.findByRole("button", { name: /draft a suite/i }, { timeout: 3000 });
    await user.click(draft);
    expect(await screen.findByDisplayValue("basic-docx", {}, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.getByText("Unsaved changes")).toBeInTheDocument();
    const section = screen.getByText(/^Trigger queries \(4\)$/);
    expect(within(section.parentElement!).getByText(/^Trigger queries/)).toBeInTheDocument();
  });
});
