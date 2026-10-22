import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RouteErrorBoundary } from "./RouteError";

function Boom(): never {
  throw new Error("Top-level render failed");
}

afterEach(() => {
  localStorage.clear();
  document.documentElement.classList.remove("dark");
  delete document.documentElement.dataset.theme;
});

describe("top-level crash theme", () => {
  it("uses the stored dark theme", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    localStorage.setItem("demo-setting:theme", "dark");
    render(
      <RouteErrorBoundary>
        <Boom />
      </RouteErrorBoundary>,
    );
    expect(screen.getByTestId("route-error")).toBeInTheDocument();
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("uses the stored light theme", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    localStorage.setItem("demo-setting:theme", "light");
    document.documentElement.classList.add("dark");
    render(
      <RouteErrorBoundary>
        <Boom />
      </RouteErrorBoundary>,
    );
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("defaults to dark when nothing is stored", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    render(
      <RouteErrorBoundary>
        <Boom />
      </RouteErrorBoundary>,
    );
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("does not restyle a route crash", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    render(
      <RouteErrorBoundary route="/my-skills">
        <Boom />
      </RouteErrorBoundary>,
    );
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });
});
