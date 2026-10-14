import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { resetEndpoints } from "@/features/endpoints/store";
import { renderShell as renderAt } from "@/test/render-app";

describe("endpoints", () => {
  afterEach(() => {
    cleanup();
    resetEndpoints();
    localStorage.clear();
  });

it("does not put a saved key in the export payload", async () => {
    const user = userEvent.setup();
    const secret = "sk-ant-endpoint-fixture-secret";
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
});
