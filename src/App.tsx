import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Route, Routes } from "react-router-dom";
import { APP_ROUTES } from "./app/routes";
import { AppShell } from "./components/shell/AppShell";
import { TooltipProvider } from "@/components/ui/tooltip";
import { bindFileWatcher } from "./lib/events/files";
import { DashboardView, PlaceholderView, SettingsView, TokensView } from "./views/Screens";

function screenFor(path: string) {
  if (path === "/") return <DashboardView />;
  if (path === "/settings") return <SettingsView />;
  if (path === "/dev/tokens") return <TokensView />;
  return <PlaceholderView />;
}

export function App({ client }: { client: QueryClient }) {
  const [platform, setPlatform] = useState("linux");

  useEffect(() => {
    const ua = navigator.userAgent;
    if (/Mac/.test(ua)) setPlatform("mac");
    else if (/Win/.test(ua)) setPlatform("windows");
    const unlisten = bindFileWatcher(client);
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [client]);

  return (
    <QueryClientProvider client={client}>
      <TooltipProvider>
      <Routes>
        <Route element={<AppShell platform={platform} />}>
          {APP_ROUTES.map((route) => (
            <Route key={route.path} path={route.path} element={screenFor(route.path)} />
          ))}
        </Route>
      </Routes>
      </TooltipProvider>
    </QueryClientProvider>
  );
}
