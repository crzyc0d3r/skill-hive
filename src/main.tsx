import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyTheme, useUi } from "./lib/store";
import "./styles/globals.css";

const root = document.getElementById("root")!;

type TauriInternals = { invoke?: (cmd: string, args?: Record<string, unknown>) => Promise<unknown> };

/** Put a boot failure in the desktop log through Tauri's own bridge, which needs no imports. */
function reportToDesktopLog(message: string) {
  const internals = (window as unknown as { __TAURI_INTERNALS__?: TauriInternals }).__TAURI_INTERNALS__;
  void internals?.invoke?.("ui_report", { level: "error", message: `boot failed: ${message}` })?.catch(() => {});
}

/**
 * A failure before the interface has rendered must never leave an empty window. The placeholder
 * from index.html stays until React has mounted; an error during that time replaces it with the
 * message, so a user can report something better than "it never loads".
 */
function showBootError(error: unknown) {
  if (!root.querySelector("[data-boot]")) return; // the app has rendered; let it handle errors
  const detail = error instanceof Error ? (error.stack ?? error.message) : String(error);
  console.error("Skill Hive failed to start", error);
  reportToDesktopLog(detail);
  root.replaceChildren();
  const pre = document.createElement("pre");
  pre.setAttribute("data-boot-error", "");
  pre.style.cssText =
    "margin:0;padding:24px;white-space:pre-wrap;font:13px/1.5 ui-monospace,Menlo,Consolas,monospace;color:#c52d31;";
  pre.textContent = `Skill Hive could not start its interface.\n\n${detail}\n\nThe log file is in the logs folder of the Skill Hive home directory (~/.skill-hive/logs).`;
  root.appendChild(pre);
}

window.addEventListener("error", (event) => showBootError(event.error ?? event.message));
window.addEventListener("unhandledrejection", (event) => showBootError(event.reason));

try {
  applyTheme(useUi.getState().theme);

  // Desktop feel: no browser context menu except on text inputs.
  document.addEventListener("contextmenu", (e) => {
    const target = e.target as HTMLElement | null;
    if (target?.closest("input, textarea, [contenteditable], .selectable, pre, code")) return;
    e.preventDefault();
  });

  ReactDOM.createRoot(root, {
    onUncaughtError: (error) => showBootError(error),
  }).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
} catch (error) {
  showBootError(error);
}
