import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  esbuild: {
    jsx: "automatic",
    jsxImportSource: "react",
  },
  resolve: {
    alias: {
      "@kinic/kinic-share/memory-internal": path.resolve(
        __dirname,
        "packages/kinic-share/src/memory-internal.ts",
      ),
      "@": path.resolve(__dirname, "."),
      "@kinic/kinic-share": path.resolve(__dirname, "packages/kinic-share/src/index.ts"),
    },
  },
  test: {
    // Route/helper tests stay on node. Component tests opt into jsdom via file pragma.
    environment: "node",
    exclude: ["node_modules/**", "workers/**", "dist/**", ".cache/**"],
  },
});
