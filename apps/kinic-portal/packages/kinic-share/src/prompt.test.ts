// Where: unit tests for the shared prompt helpers.
// What: verifies XML escaping, answer extraction, and prompt language handling.
// Why: protect the shared chat contract while the new web surfaces are introduced.

import { describe, expect, it } from "vitest";
import {
  buildAskAiPrompt,
  buildMemorySummaryPrompt,
  buildMemorySummarySearchQuery,
  extractAnswer,
} from "./prompt";

describe("prompt helpers", () => {
  it("extracts answer tags case-insensitively", () => {
    expect(extractAnswer("x <AnSwEr>done</aNsWeR> y")).toBe("done");
  });

  it("extracts answer tags from streamed chat chunks", () => {
    expect(
      extractAnswer([
        'data: {"content":"<"}',
        'data: {"content":"answer"}',
        'data: {"content":">done"}',
        'data: {"content":"</"}',
        'data: {"content":"answer>"}',
        'data: {"finish_reason":"stop"}',
      ].join("\n")),
    ).toBe("done");
  });

  it("escapes payloads inside the generated prompt", () => {
    const prompt = buildAskAiPrompt("<query>", [{ score: 0.9, payload: "<doc>unsafe</doc>" }], "ja-JP");
    expect(prompt).toContain("&lt;query&gt;");
    expect(prompt).toContain("&lt;doc&gt;unsafe&lt;/doc&gt;");
    expect(prompt).not.toContain("<doc>unsafe</doc>");
    expect(prompt).not.toContain("<full_document>");
    expect(prompt).toContain("Japanese");
  });

  it("builds a stable summary search query from memory metadata", () => {
    const query = buildMemorySummarySearchQuery("Skill Store", "Shared agent skill inventory");
    expect(query).toContain("Skill Store");
    expect(query).toContain("Shared agent skill inventory");
    expect(query).toContain("overview summary");
  });

  it("builds a public memory summary prompt", () => {
    const prompt = buildMemorySummaryPrompt(
      "Skill Store",
      "Shared agent skill inventory",
      [{ score: 0.9, payload: "Skills for agents and tools." }],
      "en",
    );
    expect(prompt).toContain("<memory_name>");
    expect(prompt).toContain("Skill Store");
    expect(prompt).toContain("2-3 sentences");
    expect(prompt).toContain("Do not exaggerate");
    expect(prompt).toContain("Answer in English");
  });
});
