import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { applyRemoval, removalPlan } from "@/features/library/updates";
import { setLocale } from "@/lib/i18n";
import { setProjectSkills } from "@/features/project/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("library updates", () => {
  afterEach(() => {
    cleanup();
    setProjectSkills([]);
    setLocale("en");
    localStorage.clear();
  });

it("holds a listed file until the approval token matches", () => {
    const plan = removalPlan(["notes.txt"]);
    expect(plan.paths).toEqual(["notes.txt"]);
    expect(applyRemoval(plan, "no")).toBe(false);
    expect(applyRemoval(plan, plan.token)).toBe(true);
  });

it("holds a file removal until Update anyway", async () => {
    const user = userEvent.setup();
    renderAt("/my-skills");
    await user.click(await screen.findByRole("button", { name: "Check All" }));
    expect(screen.getByRole("dialog", { name: "This update removes files" })).toBeInTheDocument();
    expect(screen.queryByText("notes.txt")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Update anyway" }));
    expect(screen.getByText("Update All")).toBeInTheDocument();
  });
});
