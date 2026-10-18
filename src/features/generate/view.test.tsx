import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetGenerate } from "@/features/generate/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("generate", () => {
  afterEach(() => {
    cleanup();
    resetGenerate();
    localStorage.clear();
  });

it("walks the wizard, keeps save disabled until the diff opens, and asks before discard", async () => {
    localStorage.setItem("skill-hive-demo", "1");
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
});
