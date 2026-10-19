export interface AppRoute {
  path: string;
  title: string;
  nav?: string;
}

export const APP_ROUTES: AppRoute[] = [
  { path: "/", title: "Dashboard", nav: "Dashboard" },
  { path: "/my-skills", title: "Library", nav: "Library" },
  { path: "/agents", title: "Agents", nav: "Agents" },
  { path: "/global-workspace", title: "Global Workspace" },
  { path: "/global-workspace/:agentKey", title: "Agent workspace" },
  { path: "/lobster-workspace", title: "Lobster Workspace" },
  { path: "/lobster-workspace/:agentKey", title: "Lobster agent" },
  { path: "/install", title: "Install", nav: "Install" },
  { path: "/backup", title: "Backup", nav: "Backup" },
  { path: "/project/:id", title: "Project" },
  { path: "/settings", title: "Settings", nav: "Settings" },
  { path: "/settings/endpoints", title: "Endpoints" },
  { path: "/agents/:id/edit", title: "Agent editor" },
  { path: "/agents/:id/tests", title: "Agent tests" },
  { path: "/agents/:id/history", title: "Agent history" },
  { path: "/skills/:id/edit", title: "Skill editor" },
  { path: "/skills/:id/tests", title: "Skill tests" },
  { path: "/skills/:id/history", title: "Skill history" },
  { path: "/testing", title: "Testing" },
  { path: "/testing/suites/:id", title: "Test suite" },
  { path: "/testing/runs/:id", title: "Test run" },
  { path: "/runs/:runId", title: "Run" },
  { path: "/providers", title: "Providers" },
  { path: "/generate/:sessionId", title: "Generate" },
  { path: "/compare/:comparisonId", title: "Compare" },
  { path: "/effectiveness", title: "Effectiveness" },
  { path: "/dev/tokens", title: "Tokens" },
];

export const NAV_ITEMS = APP_ROUTES.filter((route) => route.nav);
