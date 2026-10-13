import { listen } from "@tauri-apps/api/event";
import { QueryClient } from "@tanstack/react-query";
import { BASE_LISTING_KEY, bindFileWatcher } from "../lib/events/files";

it("invalidates the library query when app-files-changed arrives", async () => {
  let handler: (() => void) | undefined;
  vi.mocked(listen).mockImplementationOnce(async (_event, callback) => {
    handler = () => {
      callback({ event: "app-files-changed", id: 1, payload: "" });
    };
    return () => {};
  });
  const client = new QueryClient();
  const spy = vi.spyOn(client, "invalidateQueries");
  await bindFileWatcher(client);
  handler?.();
  expect(spy).toHaveBeenCalledWith({ queryKey: BASE_LISTING_KEY });
});
