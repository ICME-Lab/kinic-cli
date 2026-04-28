// Where: browser entrypoint for the Kinic Memory desktop app.
// What: mounts the React shell into the Tauri WebView.
// Why: keep frontend bootstrap independent from the public portal app.

import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./ui/app";
import "./globals.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Root element not found");
}

createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
