// Where: portal Cloudflare deployment helper.
// What: removes Vite's generated index.html before Worker bundling.
// Why: the Worker must own `/` so route-aware HTML is not shadowed by the static asset layer.

import fs from "node:fs";
import path from "node:path";

const indexPath = path.resolve(import.meta.dirname, "..", "dist", "client", "index.html");

if (fs.existsSync(indexPath)) {
  fs.unlinkSync(indexPath);
}
