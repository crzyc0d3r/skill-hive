import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyTheme, useUi } from "./lib/store";
import "./styles/globals.css";

applyTheme(useUi.getState().theme);

// Desktop feel: no browser context menu except on text inputs.
document.addEventListener("contextmenu", (e) => {
  const target = e.target as HTMLElement | null;
  if (target?.closest("input, textarea, [contenteditable], .selectable, pre, code")) return;
  e.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
