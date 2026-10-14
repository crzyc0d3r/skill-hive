import { QueryClient } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { App } from "@/App";

describe("help guide links", () => {
  it("links the library card to the user guide", async () => {
    const user = userEvent.setup();
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <MemoryRouter>
        <App client={client} />
      </MemoryRouter>,
    );
    await user.click(await screen.findByRole("button", { name: "Help" }));
    const library = screen.getByRole("link", { name: "Library keeps every skill in one place." });
    expect(library).toHaveAttribute("href", "https://github.com/crzyc0d3r/skill-hive/blob/main/docs/user-guide.md#library");
  });
});
