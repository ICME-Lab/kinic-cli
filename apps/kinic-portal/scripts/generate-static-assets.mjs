// Where: portal asset maintenance script.
// What: regenerates next/og SVG data URLs and favicon.ico from public asset sources.
// Why: public files stay the single source of truth while build-time consumers use generated artifacts.

import fs from "node:fs";
import path from "node:path";

const appRoot = path.resolve(import.meta.dirname, "..");
const publicRoot = path.join(appRoot, "public");
const ogRoot = path.join(publicRoot, "og");
const framePath = path.join(ogRoot, "ogp-frame.svg");
const logoPath = path.join(ogRoot, "ogp-logo.svg");
const faviconPngPath = path.join(publicRoot, "favicon.png");
const faviconIcoPath = path.join(publicRoot, "favicon.ico");
const ogpAssetsPath = path.join(appRoot, "lib", "ogp-assets.ts");

const frame = fs.readFileSync(framePath, "utf8");
const logo = fs.readFileSync(logoPath, "utf8");
const faviconPng = fs.readFileSync(faviconPngPath);

const ogpModule = `// Where: shared by OGP image routes.
// What: provides runtime-safe SVG data URLs sourced from public assets.
// Why: next/og on Cloudflare must keep asset decode cost low during prerender.

export const OGP_FRAME_SRC = ${JSON.stringify(toSvgDataUrl(frame))};
export const OGP_LOGO_SRC = ${JSON.stringify(toSvgDataUrl(logo))};
`;

fs.writeFileSync(ogpAssetsPath, ogpModule);
fs.writeFileSync(faviconIcoPath, buildIcoFromPng(faviconPng));

function toSvgDataUrl(svg) {
  const normalized = svg.replace(/\r\n/g, "\n").trim();
  return `data:image/svg+xml;base64,${Buffer.from(normalized).toString("base64")}`;
}

function buildIcoFromPng(png) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(1, 4);

  const entry = Buffer.alloc(16);
  entry.writeUInt8(0, 0);
  entry.writeUInt8(0, 1);
  entry.writeUInt8(0, 2);
  entry.writeUInt8(0, 3);
  entry.writeUInt16LE(1, 4);
  entry.writeUInt16LE(32, 6);
  entry.writeUInt32LE(png.length, 8);
  entry.writeUInt32LE(6 + 16, 12);

  return Buffer.concat([header, entry, png]);
}
