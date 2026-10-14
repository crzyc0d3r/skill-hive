import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { setLocale } from "@/lib/i18n";
import { setProjectSkills } from "@/features/project/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("close guard and help", () => {
  afterEach(() => {
    cleanup();
    setProjectSkills([]);
    setLocale("en");
    localStorage.clear();
  });

it("remembers a close choice and lists eight help cards", async () => {
    const user = userEvent.setup();
    renderAt("/");
    await user.click(await screen.findByTestId("request-close"));
    await user.click(screen.getByRole("checkbox", { name: "Remember my choice" }));
    await user.click(screen.getByRole("button", { name: "Minimize to tray" }));
    expect(localStorage.getItem("skill-hive-close")).toBe("minimize");
    await user.click(screen.getByRole("button", { name: "Help" }));
    const dialog = screen.getByRole("dialog", { name: "Help" });
    expect(dialog.querySelectorAll("[data-help-card]")).toHaveLength(8);
  });
});
