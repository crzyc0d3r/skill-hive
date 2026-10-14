import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { setLocale } from "@/lib/i18n";
import { setProjectSkills } from "@/features/project/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("project", () => {
  afterEach(() => {
    cleanup();
    setProjectSkills([]);
    setLocale("en");
    localStorage.clear();
  });

it("links a project, shares it, and changes the sidebar dot", async () => {
    const user = userEvent.setup();
    renderAt("/project/fixture");
    expect(screen.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "grey");
    await user.click(await screen.findByRole("button", { name: "Link project" }));
    expect(screen.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "red");
    await user.click(screen.getByRole("button", { name: "Add to agents" }));
    expect(screen.getByText(/claude_code, codex/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Toggle skill" }));
    expect(screen.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "grey");
    await user.click(screen.getByRole("button", { name: "Toggle skill" }));
    await user.click(screen.getByRole("button", { name: "Pull center" }));
    expect(screen.getByTestId("project-dot")).toHaveAttribute("data-color", "green");
    expect(screen.getByTestId("sidebar-project-dot")).toHaveAttribute("data-color", "green");
  });
});
