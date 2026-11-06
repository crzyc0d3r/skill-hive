// Test helper: render a component inside the providers the app uses. The router is a data
// router (like the app's), so pages may use `useBlocker` and friends.
import type { ReactElement } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { render } from "@testing-library/react";
import { TooltipProvider } from "@/components/ui";

export function renderWithProviders(ui: ReactElement, { route = "/", path = "*" }: { route?: string; path?: string } = {}) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: 0 } } });
  const router = createMemoryRouter([{ path, element: ui }], { initialEntries: [route] });
  const result = render(
    <QueryClientProvider client={client}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
  return { ...result, client, router };
}
