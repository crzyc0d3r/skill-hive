import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { api } from "@/lib/api/client";
import { ProviderDialog } from "./ProviderDialog";
import { renderWithProviders } from "@/features/evals/test-utils";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("ProviderDialog", () => {
  it("switches the visible fields and defaults by provider kind", async () => {
    const user = userEvent.setup();
    renderWithProviders(<ProviderDialog open onOpenChange={() => {}} />);

    // CLI kind: command + args, no HTTP fields, all three roles.
    expect(await screen.findByLabelText("Command")).toHaveValue("claude");
    expect(screen.getByLabelText("Arguments")).toBeInTheDocument();
    expect(screen.queryByLabelText("Base URL")).toBeNull();
    expect(screen.queryByLabelText("API key")).toBeNull();
    expect(screen.getByRole("checkbox", { name: "Executor" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("checkbox", { name: "Grader" })).toHaveAttribute("aria-checked", "true");

    // HTTP kind: base URL with presets, key source toggle, grader + recommender only.
    await user.click(screen.getByRole("radio", { name: /OpenAI-compatible API/ }));
    const baseUrl = screen.getByLabelText("Base URL");
    expect(baseUrl).toHaveValue("https://api.openai.com/v1");
    expect(screen.queryByLabelText("Command")).toBeNull();
    expect(screen.getByLabelText("API key")).toHaveAttribute("type", "password");
    expect(screen.getByRole("checkbox", { name: "Executor" })).toHaveAttribute("aria-checked", "false");
    expect(screen.getByRole("checkbox", { name: "Grader" })).toHaveAttribute("aria-checked", "true");
    await user.click(screen.getByRole("button", { name: "Ollama" }));
    expect(baseUrl).toHaveValue("http://localhost:11434/v1");
    await user.click(screen.getByRole("radio", { name: "Read from env var" }));
    expect(screen.getByLabelText("API key environment variable")).toHaveValue("OPENAI_API_KEY");
    expect(screen.queryByLabelText("API key")).toBeNull();

    // Anthropic keeps its own base URL and model.
    await user.click(screen.getByRole("radio", { name: /Anthropic API/ }));
    expect(screen.getByLabelText("Base URL")).toHaveValue("https://api.anthropic.com");
    expect(screen.getByLabelText("Model")).toHaveValue("claude-sonnet-4-5");

    // Scripted: script path, executor only.
    await user.click(screen.getByRole("radio", { name: /Scripted/ }));
    expect(screen.getByLabelText("Script path")).toHaveValue("");
    expect(screen.queryByLabelText("Base URL")).toBeNull();
    expect(screen.getByRole("checkbox", { name: "Executor" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("checkbox", { name: "Grader" })).toHaveAttribute("aria-checked", "false");
  });

  it("validates and saves a new provider through the API", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    const spy = vi.spyOn(api, "saveProvider");
    renderWithProviders(<ProviderDialog open onOpenChange={onOpenChange} />);
    await user.click(await screen.findByRole("radio", { name: /Scripted/ }));
    const submit = screen.getByRole("button", { name: /add provider/i });
    expect(submit).toBeDisabled();
    expect(screen.getByText(/script path is required/i)).toBeInTheDocument();

    await user.type(screen.getByLabelText("Script path"), "/usr/local/bin/my-agent");
    await user.type(screen.getByLabelText("Arguments"), '--mode fast "two words"');
    await user.clear(screen.getByLabelText("Provider name"));
    await user.type(screen.getByLabelText("Provider name"), "My agent");
    expect(submit).toBeEnabled();
    await user.click(submit);

    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    const [id, input] = spy.mock.calls[0];
    expect(id).toBeNull();
    expect(input).toMatchObject({ kind: "scripted", name: "My agent", command: "/usr/local/bin/my-agent", args: ["--mode", "fast", "two words"], roles: ["executor"], baseUrl: null, apiKey: null });
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });

  it("edits an existing provider without resending the stored key", async () => {
    const user = userEvent.setup();
    const spy = vi.spyOn(api, "saveProvider");
    const providers = await api.providers();
    const anthropic = providers.find((p) => p.kind === "anthropic")!;
    renderWithProviders(<ProviderDialog open onOpenChange={() => {}} provider={anthropic} />);
    expect(await screen.findByText(/a key is stored in the OS keychain/i)).toBeInTheDocument();
    expect(screen.queryByRole("radiogroup", { name: "Provider kind" })).toBeNull();
    await user.clear(screen.getByLabelText("Model"));
    await user.type(screen.getByLabelText("Model"), "claude-opus-4-1");
    await user.click(screen.getByRole("button", { name: /save changes/i }));
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    expect(spy.mock.calls[0][0]).toBe(anthropic.id);
    expect(spy.mock.calls[0][1]).toMatchObject({ model: "claude-opus-4-1", apiKey: null, roles: ["grader", "recommender"] });
  });
});
