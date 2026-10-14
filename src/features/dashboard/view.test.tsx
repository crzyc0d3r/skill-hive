import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { installIpc, renderAt } from "@/test/render-app";

describe("dashboard", () => {
  it("shows an empty dashboard and the scan action", async () => {
    installIpc([]);
    renderAt("/");
    expect(await screen.findByText("0 skills")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Scan & Import Local Skills" })).toHaveAttribute("href", "/install?tab=scan");
  });

  it("opens a skill from the command palette", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/");
    await user.keyboard("{Control>}k{/Control}");
    await user.type(screen.getByLabelText("Command search"), "research-notes{Enter}");
    expect(await screen.findByTestId("detail-panel")).toBeInTheDocument();
    expect(screen.getByText("Local body")).toBeInTheDocument();
  });

  it("shows the load banner with Retry when settings fail", async () => {
    installIpc([], true);
    const user = userEvent.setup();
    renderAt("/");
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not load settings.");
    installIpc([], false);
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(await screen.findByText("0 skills")).toBeInTheDocument();
  });
});
