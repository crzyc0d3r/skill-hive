import { QueryClient } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { App } from "../App";

function renderApp(path = "/") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App client={client} />
    </MemoryRouter>,
  );
}

describe("application shell", () => {
  it("renders sidebar, header, status bar, and the command palette", async () => {
    const user = userEvent.setup();
    renderApp();
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(screen.getByTestId("header")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Skill Hive 1.0.0");
    expect(screen.getByTestId("status-banner")).toBeInTheDocument();

    await user.keyboard("{Control>}k{/Control}");
    expect(screen.getByTestId("command-palette")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Command search"), "zzz");
    expect(screen.getByText(/No matches for 'zzz'/)).toBeInTheDocument();

    await user.keyboard("{Escape}");
    await user.keyboard("{Control>}b{/Control}");
    expect(screen.getByTestId("sidebar")).toHaveAttribute("data-collapsed", "false");
  });

  it("persists the theme choice through set_settings", async () => {
    const user = userEvent.setup();
    renderApp("/settings");
    const dark = await screen.findByRole("button", { name: "Dark" });
    await user.click(dark);
    expect(dark).toHaveAttribute("aria-pressed", "true");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("routes numbered shortcuts, focuses search, and stores a remap", async () => {
    const user = userEvent.setup();
    renderApp("/my-skills");
    await user.keyboard("{Control>}f{/Control}");
    expect(screen.getByLabelText("Search skills")).toHaveFocus();
    await user.keyboard("{Control>}/{/Control}");
    expect(screen.getByRole("dialog", { name: "Shortcuts" })).toBeInTheDocument();
    const first = screen.getByRole("textbox", { name: "Shortcut 1" });
    await user.clear(first);
    await user.type(first, "/settings");
    await user.click(screen.getByRole("button", { name: "Save shortcuts" }));
    await user.click(screen.getByRole("button", { name: "Close" }));
    await user.keyboard("{Control>}1{/Control}");
    expect(await screen.findByTestId("settings")).toBeInTheDocument();
  });
});
