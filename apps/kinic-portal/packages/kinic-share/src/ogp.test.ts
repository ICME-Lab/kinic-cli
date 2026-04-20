// Where: unit tests for shared OGP helpers.
// What: verifies social-card copy fallback, truncation, and compact memory stats.
// Why: OGP text must stay deterministic because portal metadata and image routes share this contract.

import { describe, expect, it } from "vitest";
import {
  buildMemoryOgpImageCopy,
  buildMemoryOgpSearchCards,
  DEFAULT_MEMORY_METADATA_DESCRIPTION,
  DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION,
  DEFAULT_MEMORY_OGP_IMAGE_TITLE,
  buildMemoryMetadataDescription,
  buildMemoryOgpCardModel,
  buildMemoryPageTitle,
  buildMemorySearchQuery,
  shortenMemoryId,
} from "./ogp";

describe("ogp helpers", () => {
  it("uses the default description when memory description is absent", () => {
    expect(buildMemoryMetadataDescription(null)).toBe(DEFAULT_MEMORY_METADATA_DESCRIPTION);
  });

  it("truncates long metadata copy", () => {
    const value = buildMemoryMetadataDescription("a".repeat(240));
    expect(value.length).toBeLessThanOrEqual(160);
    expect(value.endsWith("…")).toBe(true);
  });

  it("shortens memory ids for compact stat cards", () => {
    expect(shortenMemoryId("aaaaaa-bbbbb-ccccc-dddd")).toBe("aaaaaa...dddd");
  });

  it("builds an ogp card model with fallbacks for missing values", () => {
    expect(buildMemoryOgpCardModel({ name: "Alpha", description: "", memoryId: null })).toEqual({
      title: "Alpha",
      description: DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION,
      shortMemoryId: "-",
    });
  });

  it("keeps the full memory id in the card model", () => {
    expect(
      buildMemoryOgpCardModel({
        name: "Alpha",
        description: "Beta",
        memoryId: "ywega-gaaaa-aaaak-apg6q-cai",
      }),
    ).toEqual({
      title: "Alpha",
      description: "Beta",
      shortMemoryId: "ywega-gaaaa-aaaak-apg6q-cai",
    });
  });

  it("uses the new shared-memory fallback copy for blank cards", () => {
    expect(buildMemoryOgpCardModel({ name: "", description: "", memoryId: null })).toEqual({
      title: DEFAULT_MEMORY_OGP_IMAGE_TITLE,
      description: DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION,
      shortMemoryId: "-",
    });
  });

  it("keeps non-ascii title and description in OGP copy", () => {
    expect(buildMemoryOgpCardModel({ name: "Память", description: "Резюме" })).toEqual({
      title: "Память",
      description: "Резюме",
      shortMemoryId: "-",
    });
  });

  it("clamps OGP image query copy before it reaches metadata urls", () => {
    const copy = buildMemoryOgpImageCopy({
      name: "a".repeat(120),
      description: "b".repeat(220),
    });

    expect(copy.title.length).toBeLessThanOrEqual(72);
    expect(copy.description.length).toBeLessThanOrEqual(160);
    expect(copy.title.endsWith("…")).toBe(true);
    expect(copy.description.endsWith("…")).toBe(true);
  });

  it("builds the memory page title from the resolved name", () => {
    expect(buildMemoryPageTitle("Quarterly Goals")).toBe("Quarterly Goals | Kinic");
  });

  it("builds an OGP search query from name and description", () => {
    expect(buildMemorySearchQuery({ name: "Alpha", description: "Quarterly goals" })).toBe(
      "Alpha Quarterly goals",
    );
  });

  it("extracts actual search cards from stored payload rows", () => {
    expect(
      buildMemoryOgpSearchCards([
        { payload: "{\"tag\":\"notes\",\"payload\":\"Revenue increased 18 percent quarter over quarter.\"}" },
      ]),
    ).toEqual([
      {
        tag: "notes",
        excerpt: "Revenue increased 18 percent quarter over quarter.",
      },
    ]);
  });

  it("falls back when search payload is non-ascii only", () => {
    expect(
      buildMemoryOgpSearchCards([{ payload: "{\"tag\":\"Тег\",\"payload\":\"Резюме\"}" }]),
    ).toEqual([
      {
        tag: "Search Hit",
        excerpt: "Search result preview unavailable",
      },
    ]);
  });
});
