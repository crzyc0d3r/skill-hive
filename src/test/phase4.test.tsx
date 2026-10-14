import { QueryClient } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { resetPhase4 } from "../lib/phase4/store";

function renderAt(path: string) {
  mockIPC((cmd, args) => {
    const record = (args ?? {}) as Record<string, string>;
    if (cmd === "get_settings") return record.key === "theme" ? "light" : null;
    if (cmd === "set_settings") return null;
    if (cmd === "list_base_entries") return [];
    if (cmd === "get_managed_skills") return [];
    if (cmd === "list_presets") return [];
    if (cmd === "get_tool_status") return [];
    return null;
  });
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App client={client} />
    </MemoryRouter>,
  );
}

describe("phase 4 providers and results", () => {
  afterEach(() => {
    cleanup();
    resetPhase4();
    localStorage.clear();
  });

  it("does not put a saved key in the export payload", async () => {
    const user = userEvent.setup();
    const secret = "sk-ant-phase4-fixture-secret";
    renderAt("/settings/endpoints");
    await user.type(await screen.findByLabelText("API key"), secret);
    await user.click(screen.getByRole("button", { name: "Save key" }));
    expect(screen.queryByDisplayValue(secret)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Export" }));
    const payload = screen.getByTestId("endpoint-export").textContent ?? "";
    expect(payload).not.toContain(secret);
    expect(payload).toContain("hasSecret");
    expect(document.body.textContent).not.toContain(secret);
  });

  it("opens a heatmap cell with the keyboard and jumps to evidence", async () => {
    const user = userEvent.setup();
    renderAt("/runs/run-fixture");
    const grid = await screen.findByTestId("heatmap");
    grid.focus();
    await user.keyboard("{ArrowRight}{Enter}");
    expect(screen.getByTestId("heatmap-open")).toHaveTextContent("without-1");
    await user.type(screen.getByLabelText("Jump to evidence"), "2");
    await user.click(screen.getAllByRole("button", { name: "Jump to evidence" })[0]);
    expect(screen.getByTestId("evidence-anchor")).toHaveTextContent("step-2");
    await user.clear(screen.getByLabelText("Jump to evidence"));
    await user.type(screen.getByLabelText("Jump to evidence"), "missing");
    await user.click(screen.getAllByRole("button", { name: "Jump to evidence" })[0]);
    expect(screen.getByTestId("evidence-anchor")).toHaveTextContent("No anchor");
  });

  it("confirms a budget pause and shows the run chip", async () => {
    const user = userEvent.setup();
    renderAt("/skills/skill-research/tests");
    expect(await screen.findByTestId("suite")).toBeInTheDocument();
    expect(screen.getByTestId("run-chip")).toHaveAttribute("href", "/runs/run-fixture");
    await user.click(screen.getByRole("link", { name: "Run suite" }));
    await user.click(await screen.findByRole("button", { name: "Budget" }));
    expect(screen.getByTestId("budget-dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.getByText("Paused")).toBeInTheDocument();
  });
});
