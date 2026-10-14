import { QueryClient } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { resetPhase5 } from "../lib/phase5/store";

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

describe("phase 5 generation and analytics", () => {
  afterEach(() => {
    cleanup();
    resetPhase5();
    localStorage.clear();
  });

  it("walks the wizard, keeps save disabled until the diff opens, and asks before discard", async () => {
    const user = userEvent.setup();
    renderAt("/generate/skill");
    expect(await screen.findByTestId("generate-wizard")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Goal"), "Extract invoices");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await user.click(screen.getByRole("button", { name: "Accept all defaults" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByTestId("generate-problems")).toHaveTextContent("Problems: 0");
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Open diff" }));
    expect(screen.getByTestId("generate-diff")).toHaveTextContent("SKILL.md");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(screen.getByTestId("generate-saved")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.getByRole("dialog", { name: "Discard this draft?" })).toBeInTheDocument();
  });

  it("shows tested and regressed badges and the dashboard regressions block", async () => {
    localStorage.setItem("skill-hive-demo", "1");
    localStorage.setItem(
      "skill-hive-phase5",
      JSON.stringify({
        tested: ["skill-research"],
        regressed: ["skill-scan"],
        regressions: [{ skillId: "skill-scan", skill: "scan", rule: "R2", message: "columns failed" }],
      }),
    );
    renderAt("/my-skills");
    expect(await screen.findByTestId("tested-badge")).toHaveTextContent("Tested");
    expect(screen.getByTestId("regressed-badge")).toHaveTextContent("Regressed");
    cleanup();
    renderAt("/");
    expect(await screen.findByTestId("tested-count")).toHaveTextContent("1");
    expect(screen.getByTestId("regression-row")).toHaveTextContent("R2");
  });

  it("keeps comparison blind until the verdict and applies optimization only on click", async () => {
    const user = userEvent.setup();
    renderAt("/compare/cmp-fixture");
    expect(await screen.findByText("Output A")).toBeInTheDocument();
    expect(screen.queryByText("security-reviewer v2")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Record verdict" }));
    expect(screen.getByText("security-reviewer v2")).toBeInTheDocument();
    cleanup();
    renderAt("/skills/skill-research/tests");
    expect(screen.queryByTestId("optimize-applied")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Optimize description" }));
    expect(screen.getByTestId("optimize-preview")).toHaveTextContent("not applied");
    expect(screen.queryByTestId("optimize-applied")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Apply" }));
    expect(screen.getByTestId("optimize-applied")).toBeInTheDocument();
  });

  it("shows the history chain with one current best", async () => {
    renderAt("/skills/skill-research/history");
    expect(await screen.findByTestId("history-v1")).toBeInTheDocument();
    expect(screen.getByTestId("history-v2-optimized")).toHaveTextContent("0.97");
  });
});
