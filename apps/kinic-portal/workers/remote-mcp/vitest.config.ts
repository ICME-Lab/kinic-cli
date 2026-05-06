// Where: remote MCP Worker tests.
// What: maps workspace packages to source files for Vitest.
// Why: contract tests must run without relying on the portal app test config.

import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    alias: {
      "@kinic/kinic-share": path.resolve(__dirname, "../../packages/kinic-share/src/index.ts"),
    },
  },
  test: {
    environment: "node",
  },
});
