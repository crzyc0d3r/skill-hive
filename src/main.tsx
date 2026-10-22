import { QueryClient } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { App } from "./App";
import { logStartupEvent } from "./lib/ipc/settings";
import { applyTheme, readStoredTheme } from "./lib/theme";
import "./styles/tokens.css";

const storedTheme = readStoredTheme();
if (storedTheme) applyTheme(storedTheme);

const queryClient = new QueryClient();

const started = performance.now();
try {
  void logStartupEvent("first-paint", Math.round(performance.now() - started)).catch(() => {
    /* The webview is not present under Vite alone. */
  });
} catch {
  /* invoke can throw before it returns a promise outside the webview. */
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <App client={queryClient} />
    </BrowserRouter>
  </StrictMode>,
);
