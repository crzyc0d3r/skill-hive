import { describe, expect, it } from "vitest";
import { backupNow, restoreSnapshot } from "./model";
import { proxyOk } from "@/features/settings/proxy";
import { effectiveClose } from "@/lib/shell/close";

describe("backup", () => {
  it("restores a snapshot behind a safety tag and checks proxy URLs", () => {
    const history = backupNow("library v1", []);
    expect(restoreSnapshot(history, "snap-1")?.safety).toBe("safety-snap-1");
    expect(proxyOk("")).toBe(true);
    expect(proxyOk("http://127.0.0.1:8888")).toBe(true);
    expect(proxyOk("ftp://example")).toBe(false);
    expect(effectiveClose("minimize", false)).toBe("ask");
  });
});
