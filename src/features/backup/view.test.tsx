import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { setLocale } from "@/lib/i18n";
import { setProjectSkills } from "@/features/project/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("backup", () => {
  afterEach(() => {
    cleanup();
    setProjectSkills([]);
    setLocale("en");
    localStorage.clear();
  });

it("backs up and restores a snapshot", async () => {
    const user = userEvent.setup();
    renderAt("/backup");
    expect(await screen.findByText("First-run restore is ready when a remote exists.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Back Up Now" }));
    await user.click(screen.getByRole("button", { name: /Snapshot actions/ }));
    await user.click(screen.getByRole("menuitem", { name: "Restore" }));
    expect(screen.getByText(/safety-snap-1/)).toBeInTheDocument();
  });
});
