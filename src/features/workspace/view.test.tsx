import { fireEvent, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { installIpc, renderAt, SKILLS } from "@/test/render-app";

describe("workspace", () => {
  it("applies a preset from the global workspace", async () => {
    installIpc();
    const user = userEvent.setup();
    renderAt("/global-workspace");
    const pill = await screen.findByRole("button", { name: "Research" });
    await user.click(pill);
    expect(screen.getByRole("button", { name: "Research check" })).toHaveAttribute("data-fill", "active");
  });

  it("opens the agent page and confirms upload, pull, remove, and a refused delete", async () => {
    installIpc([{ ...SKILLS[0], agents: ["claude_code"] }, SKILLS[1], SKILLS[2]]);
    const user = userEvent.setup();
    const overview = renderAt("/global-workspace");
    expect(await within(await screen.findByTestId("agent-catalog")).findByRole("link", { name: "Claude Code" })).toHaveAttribute("href", "/global-workspace/claude_code");
    const tools = document.querySelector("[data-preset='tools']");
    const research = document.querySelector("[data-preset='research']");
    fireEvent.dragStart(tools!);
    fireEvent.drop(research!);
    const order = [...document.querySelectorAll("[data-preset]")].map((node) => node.getAttribute("data-preset"));
    expect(order).toEqual(["tools", "research"]);
    await user.click(screen.getByRole("button", { name: "Edit preset Research" }));
    const editor = screen.getByRole("textbox", { name: "Edit preset" });
    await user.clear(editor);
    await user.type(editor, "Sources");
    await user.click(screen.getByRole("button", { name: "Save preset" }));
    expect(screen.getByRole("link", { name: "Sources" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Delete preset Tools" }));
    expect(screen.queryByRole("link", { name: "Tools" })).not.toBeInTheDocument();
    overview.unmount();

    renderAt("/global-workspace/claude_code");
    expect(await screen.findByTestId("agent-page")).toBeInTheDocument();
    await user.type(screen.getByRole("textbox", { name: "Search agent skills" }), "review");
    expect(screen.queryByText("research-notes")).not.toBeInTheDocument();
    await user.clear(screen.getByRole("textbox", { name: "Search agent skills" }));
    await user.click(screen.getByRole("button", { name: "research" }));
    expect(screen.queryByText("plain")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "research" }));
    await user.click(screen.getByRole("button", { name: "List" }));
    expect(document.querySelector("[data-layout='list']")).toBeTruthy();
    await user.click(screen.getAllByRole("button", { name: "Delete local skill" })[0]);
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.getByText("refusing to delete managed skill")).toBeInTheDocument();
    expect(screen.getByText("research-notes")).toBeInTheDocument();
    await user.click(screen.getAllByRole("button", { name: "Pull" })[1]);
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    await user.click(screen.getAllByRole("button", { name: "Remove from agent" })[2]);
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.queryByText("plain")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Add from Library" }));
    await user.click(within(screen.getByTestId("add-sheet")).getByRole("button", { name: "plain" }));
    expect(screen.getByText("plain")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "review-diff" }));
    await user.click(screen.getByRole("button", { name: "Center" }));
    expect(screen.getByTestId("workspace-detail")).toHaveTextContent("right");
    await user.click(screen.getByRole("checkbox", { name: "Select review-diff" }));
    await user.click(screen.getByRole("button", { name: "Batch remove" }));
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(screen.queryByText("review-diff")).not.toBeInTheDocument();
  });
});
