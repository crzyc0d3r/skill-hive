import { afterEach, describe, expect, it } from "vitest";
import { ApiError, api, call, onHiveEvent, setBackend, type Backend } from "./client";
import type { HiveEvent } from "./bindings";

afterEach(() => setBackend(null));

describe("api client", () => {
  it("routes to the mock backend outside Tauri", async () => {
    const dashboard = await api.dashboard();
    expect(dashboard.stats.total).toBeGreaterThan(0);
    expect(dashboard.targets.length).toBeGreaterThan(0);
    const items = await api.listItems({ text: "pdf", kinds: [], tags: [], sourceKinds: [], favoritesOnly: false, deployedTo: null, sort: "relevance", limit: null });
    expect(items[0].name).toBe("pdf");
  });

  it("normalises backend errors into ApiError", async () => {
    const backend: Backend = {
      call: async () => {
        throw { code: "not_found", message: "item x" };
      },
      onEvent: () => () => {},
    };
    setBackend(backend);
    await expect(call("library_get", { id: "x" })).rejects.toMatchObject({ name: "ApiError", code: "not_found", message: "item x" });
    await expect(api.itemDetail("missing")).rejects.toBeInstanceOf(ApiError);
  });

  it("delivers backend events to subscribers and supports disposal", async () => {
    const listeners = new Set<(e: HiveEvent) => void>();
    const backend: Backend = {
      call: async <R,>() => undefined as unknown as R,
      onEvent: (handler) => {
        listeners.add(handler);
        return () => listeners.delete(handler);
      },
    };
    setBackend(backend);
    const received: HiveEvent[] = [];
    const dispose = onHiveEvent((e) => received.push(e));
    await new Promise((r) => setTimeout(r, 0));
    listeners.forEach((l) => l({ type: "library_changed" }));
    expect(received).toEqual([{ type: "library_changed" }]);
    dispose();
    expect(listeners.size).toBe(0);
  });

  it("mock mutations emit events that keep caches honest", async () => {
    const events: HiveEvent[] = [];
    const dispose = onHiveEvent((e) => events.push(e));
    await new Promise((r) => setTimeout(r, 0));
    const created = await api.createSkill("notes-tidy", "Tidy meeting notes into actions. Use when asked to summarise a meeting.", "# Notes", ["meetings"]);
    expect(created.name).toBe("notes-tidy");
    expect(events.some((e) => e.type === "library_changed")).toBe(true);
    await expect(api.createSkill("Bad Name", "d", "b", [])).rejects.toBeInstanceOf(ApiError);
    const removed = await api.removeItem(created.id);
    expect(removed.id).toBe(created.id);
    dispose();
  });
});
