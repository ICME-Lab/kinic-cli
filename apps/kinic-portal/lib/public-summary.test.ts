import { describe, expect, it } from "vitest";
import { DEFAULT_SUMMARY_LANGUAGE, normalizeSummaryLanguage, resolveSummaryLanguage } from "./public-summary";

describe("public summary language helpers", () => {
  it("prefers the query parameter over headers", () => {
    const request = new Request("https://portal.kinic.io/api/memories/x/summary?language=ja-JP", {
      headers: { "accept-language": "en-US,en;q=0.9" },
    });
    expect(resolveSummaryLanguage(request)).toBe("ja-jp");
  });

  it("falls back to the first accept-language entry", () => {
    const request = new Request("https://portal.kinic.io/api/memories/x/summary", {
      headers: { "accept-language": "ko-KR,ko;q=0.9,en;q=0.8" },
    });
    expect(resolveSummaryLanguage(request)).toBe("ko-kr");
  });

  it("uses english when the input is empty", () => {
    expect(normalizeSummaryLanguage("")).toBe(DEFAULT_SUMMARY_LANGUAGE);
    expect(normalizeSummaryLanguage(undefined)).toBe(DEFAULT_SUMMARY_LANGUAGE);
  });
});
