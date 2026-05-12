// Where: shared route tree used by the browser hydrate path and the Worker SSR path.
// What: defines the public portal routes without any framework-specific wrappers.
// Why: one route tree keeps rendered HTML and hydrated behavior aligned.

import { Route, Routes } from "react-router";
import type { PortalRuntimeConfig } from "./runtime-config";
import { CliLoginPage } from "./routes/cli-login-page";
import { HomePage } from "./routes/home-page";
import { MemoryPage } from "./routes/memory-page";
import { NotFoundPage } from "./routes/not-found-page";

export function App({ config }: { config: PortalRuntimeConfig }) {
  return (
    <Routes>
      <Route path="/" element={<HomePage />} />
      <Route path="/cli-login" element={<CliLoginPage />} />
      <Route path="/m/:memoryId" element={<MemoryPage config={config} />} />
      <Route path="*" element={<NotFoundPage />} />
    </Routes>
  );
}
