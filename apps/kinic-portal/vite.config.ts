// Where: Vite build entrypoint for the Kinic portal shell.
// What: emits one deterministic client bundle for the Cloudflare SSR Worker.
// Why: the Worker renders HTML directly, so asset names must stay stable and predictable.

import path from "node:path";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "."),
      "@kinic/kinic-share": path.resolve(__dirname, "packages/kinic-share/src/index.ts"),
    },
  },
  build: {
    outDir: "dist/client",
    emptyOutDir: true,
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: "assets/portal.js",
        chunkFileNames: "assets/[name]-[hash].js",
        assetFileNames: (assetInfo) => {
          if (assetInfo.names.includes("style.css")) {
            return "assets/portal.css";
          }
          return "assets/[name]-[hash][extname]";
        },
      },
    },
  },
});
