import { cleanup, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { setLocale } from "@/lib/i18n";
import { setProjectSkills } from "@/features/project/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("lobster", () => {
  afterEach(() => {
    cleanup();
    setProjectSkills([]);
    setLocale("en");
    localStorage.clear();
  });

it("shows lobster agents only when the home marker is present", async () => {
    renderAt("/");
    expect(screen.queryByText("All Lobster Agents")).not.toBeInTheDocument();
    cleanup();
    localStorage.setItem("skill-hive-home", ".openclaw");
    renderAt("/global-workspace/openclaw");
    expect(await screen.findByTestId("lobster")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "All Lobster Agents" })).toBeInTheDocument();
  });
});
