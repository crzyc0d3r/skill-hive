import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetGenerate } from "@/features/generate/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("compare", () => {
  afterEach(() => {
    cleanup();
    resetGenerate();
    localStorage.clear();
  });

it("keeps comparison blind until the verdict and applies optimization only on click", async () => {
    localStorage.setItem("skill-hive-demo", "1");
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
});
