// Where: unit tests for generated next/og asset helpers.
// What: verifies generated data URLs still match the public asset sources.
// Why: public files are the single source of truth and stale generated output must be caught in CI.

import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { OGP_FRAME_SRC, OGP_LOGO_SRC, OGP_MEMORY_CHROME_SRC } from "./ogp-assets";

const appRoot = path.resolve(__dirname, "..");
const publicRoot = path.join(appRoot, "public", "og");

describe("ogp assets", () => {
  it("matches the public frame svg", () => {
    expect(OGP_FRAME_SRC).toBe(
      toSvgDataUrl(readText("ogp-frame.svg")),
    );
  });

  it("matches the public logo svg", () => {
    expect(OGP_LOGO_SRC).toBe(
      toSvgDataUrl(readText("ogp-logo.svg")),
    );
  });

  it("matches the public memory chrome svg", () => {
    expect(OGP_MEMORY_CHROME_SRC).toBe(
      toPngDataUrl(readBinary("ogp-memory-chrome.png")),
    );
  });
});

function readText(name: string): string {
  return fs.readFileSync(path.join(publicRoot, name), "utf8");
}

function readBinary(name: string): Buffer {
  return fs.readFileSync(path.join(publicRoot, name));
}

function toSvgDataUrl(svg: string): string {
  return `data:image/svg+xml;base64,${Buffer.from(
    svg.replace(/\r\n/g, "\n").trim(),
  ).toString("base64")}`;
}

function toPngDataUrl(png: Buffer): string {
  return `data:image/png;base64,${png.toString("base64")}`;
}
