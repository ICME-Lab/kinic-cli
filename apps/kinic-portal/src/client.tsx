// Where: client bootstrap for the Kinic portal shell.
// What: hydrates the Worker-rendered HTML using the same route tree as the server.
// Why: public detail, summary, and chat still run as client fetches after the initial HTML response.

import { hydrateRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";
import "./globals.css";
import { App } from "./app";
import { readRuntimeConfig } from "./runtime-config";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("portal root element missing");
}

hydrateRoot(
  rootElement,
  <BrowserRouter>
    <App config={readRuntimeConfig()} />
  </BrowserRouter>,
);
