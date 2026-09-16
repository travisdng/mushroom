import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/index.css";

/**
 * Strip the remaining web-view tells that CSS cannot reach (R1.7). The
 * context menu is suppressed globally here; the note tree and note list
 * re-enable their own classic context menus in spec 02.
 */
function suppressWebViewDefaults() {
  if (import.meta.env.DEV) return;

  document.addEventListener("contextmenu", (e) => e.preventDefault());
  // Dropping a file onto the window must not navigate away from the app.
  document.addEventListener("dragover", (e) => e.preventDefault());
  document.addEventListener("drop", (e) => e.preventDefault());
}

suppressWebViewDefaults();

const root = document.getElementById("root");
if (!root) {
  throw new Error("Mushroom could not find its root element.");
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
