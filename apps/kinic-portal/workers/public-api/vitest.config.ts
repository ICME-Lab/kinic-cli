import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    alias: {
      "@kinic/kinic-share": path.resolve(__dirname, "../../packages/kinic-share/src/index.ts"),
      "@kinic/kinic-share/memory-internal": path.resolve(
        __dirname,
        "../../packages/kinic-share/src/memory-internal.ts",
      ),
    },
  },
  test: {
    environment: "node",
    globals: true,
  },
});
