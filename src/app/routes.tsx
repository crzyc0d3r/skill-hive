import { createHashRouter, createMemoryRouter, Navigate } from "react-router-dom";
import { AppShell } from "./shell/AppShell";
import { HomePage } from "@/features/home/HomePage";
import { LibraryPage } from "@/features/library/LibraryPage";
import { DiscoverPage } from "@/features/discover/DiscoverPage";
import { FindPage } from "@/features/find/FindPage";
import { AgentsPage } from "@/features/agents/AgentsPage";
import { LoadoutsPage } from "@/features/loadouts/LoadoutsPage";
import { EvalsPage } from "@/features/evals/EvalsPage";
import { RunPage } from "@/features/evals/RunPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { RouteError } from "./shell/RouteError";

// Hash routing in the desktop app; an in-memory router for hosted previews that forbid hash paths.
const createRouter = import.meta.env.VITE_ROUTER === "memory" ? createMemoryRouter : createHashRouter;

export const router = createRouter([
  {
    path: "/",
    element: <AppShell />,
    errorElement: <RouteError />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "library", element: <LibraryPage /> },
      { path: "discover", element: <DiscoverPage /> },
      { path: "find", element: <FindPage /> },
      { path: "agents", element: <AgentsPage /> },
      { path: "agents/:targetId", element: <AgentsPage /> },
      { path: "loadouts", element: <LoadoutsPage /> },
      { path: "loadouts/:loadoutId", element: <LoadoutsPage /> },
      { path: "evals", element: <EvalsPage /> },
      { path: "evals/:itemId", element: <EvalsPage /> },
      { path: "runs/:runId", element: <RunPage /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "settings/:tab", element: <SettingsPage /> },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);
