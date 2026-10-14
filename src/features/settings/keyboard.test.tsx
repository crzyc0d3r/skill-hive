import { cleanup, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { resetAuthoring } from "@/lib/authoring/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("keyboard settings", () => {
  afterEach(() => {
    cleanup();
    resetAuthoring();
    localStorage.clear();
  });

it("lists editor shortcuts on the keyboard settings tab", async () => {
    renderAt("/settings?tab=keyboard");
    expect(await screen.findByText("Mod+S")).toBeInTheDocument();
    expect(screen.getByText("Mod+J")).toBeInTheDocument();
    expect(screen.getByText("F2")).toBeInTheDocument();
    expect(screen.getByText("Mod+N")).toBeInTheDocument();
  });
});
