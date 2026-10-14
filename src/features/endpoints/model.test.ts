import { describe, expect, it } from "vitest";
import { publicEndpoint } from "./model";
import { exportEndpoints, resetEndpoints, setEndpointSecret } from "./store";

describe("endpoint export", () => {
  it("keeps the saved secret out of the export", () => {
    resetEndpoints();
    setEndpointSecret("sk-ant-endpoint-fixture-secret");
    const body = JSON.stringify(exportEndpoints());
    expect(body).not.toContain("sk-ant-endpoint-fixture-secret");
    expect(body).toContain("hasSecret");
    expect(publicEndpoint(exportEndpoints()[0]).hasSecret).toBe(true);
    resetEndpoints();
  });
});
