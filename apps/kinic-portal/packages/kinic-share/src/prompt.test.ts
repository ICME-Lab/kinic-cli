// Where: unit tests for the shared prompt helpers.
// What: verifies XML escaping, answer extraction, and prompt language handling.
// Why: protect the shared chat contract while the new web surfaces are introduced.

import { describe, expect, it } from "vitest";
import { buildAskAiPrompt, extractAnswer } from "./prompt";

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
    expect(prompt).toContain("Japanese");
  });
});
