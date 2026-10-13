import { invoke } from "@tauri-apps/api/core";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

it("drives a command through mockIPC", async () => {
  mockWindows("main");
  mockIPC((cmd, args) => {
    if (cmd === "get_settings") {
      return (args as { key: string }).key === "theme" ? "dark" : null;
    }
    return null;
  });
  await expect(invoke("get_settings", { key: "theme" })).resolves.toBe("dark");
});
