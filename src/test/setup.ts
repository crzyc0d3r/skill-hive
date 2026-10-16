import "@testing-library/jest-dom/vitest";

class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver = ResizeObserverMock;
Element.prototype.scrollIntoView = () => {};
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { vi } from "vitest";

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

mockWindows("main");

const settings = new Map<string, string>();

mockIPC((cmd, args) => {
  const record = (args ?? {}) as Record<string, string>;
  if (cmd === "get_settings") return settings.get(record.key ?? "") ?? null;
  if (cmd === "set_settings") {
    settings.set(record.key, record.value);
    return null;
  }
  if (cmd === "log_startup_event") return null;
  if (cmd === "list_base_entries") return ["alpha-skill"];
  return null;
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onThemeChanged: async () => () => {},
  }),
}));
