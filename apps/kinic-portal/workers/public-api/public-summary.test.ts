import { describe, expect, it } from "vitest";
import { buildSummaryCacheKey } from "./src/summary-cache";
import { resolveSummaryLanguage } from "./src/public-summary";

describe("resolveSummaryLanguage", () => {
  it("returns supported primary language codes", () => {
    expect(resolveSummaryLanguage(new Request("https://api.kinic.test/summary?language=ja-JP"))).toBe("ja");
  });

  it("folds unsupported query languages into english", () => {
    const first = resolveSummaryLanguage(new Request("https://api.kinic.test/summary?language=foo1"));
    const second = resolveSummaryLanguage(new Request("https://api.kinic.test/summary?language=foo2"));

    expect(first).toBe("en");
    expect(second).toBe("en");
    expect(buildSummaryCacheKey("m1", "v1", first)).toBe(buildSummaryCacheKey("m1", "v1", second));
  });

  it("folds unsupported and oversized accept-language values into english", () => {
    expect(
      resolveSummaryLanguage(
        new Request("https://api.kinic.test/summary", {
          headers: { "accept-language": "unsupported-language-tag-that-is-too-long,en;q=0.8" },
        }),
      ),
    ).toBe("en");
  });
});
