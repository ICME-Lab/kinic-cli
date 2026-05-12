import { describe, expect, it } from "vitest";
import { isAllowedLocalCallback, parseCliLoginHash } from "./cli-login-page";

describe("cli login params", () => {
  it("returns empty for direct visits without hash params", () => {
    const result = parseCliLoginHash("");

    expect(result).toEqual({ kind: "empty" });
  });

  it("accepts localhost callback hashes from icp-cli", () => {
    const result = parseCliLoginHash("#public_key=abc&callback=http%3A%2F%2F127.0.0.1%3A1234%2Fcallback");

    expect(result).toEqual({
      kind: "ok",
      params: {
        publicKey: "abc",
        callback: "http://127.0.0.1:1234/callback",
      },
    });
  });

  it("accepts ipv6 loopback callbacks", () => {
    const result = parseCliLoginHash("#public_key=abc&callback=http%3A%2F%2F%5B%3A%3A1%5D%3A1234%2Fcallback");

    expect(result).toEqual({
      kind: "ok",
      params: {
        publicKey: "abc",
        callback: "http://[::1]:1234/callback",
      },
    });
  });

  it("rejects missing public keys", () => {
    const result = parseCliLoginHash("#callback=http%3A%2F%2F127.0.0.1%3A1234%2Fcallback");

    expect(result.kind).toBe("error");
  });

  it("rejects missing callbacks", () => {
    const result = parseCliLoginHash("#public_key=abc");

    expect(result.kind).toBe("error");
  });

  it("rejects remote callbacks", () => {
    expect(isAllowedLocalCallback("https://memory.kinic.xyz/callback")).toBe(false);
    expect(isAllowedLocalCallback("http://localhost:1234/callback")).toBe(false);
    expect(parseCliLoginHash("#public_key=abc&callback=https%3A%2F%2Fmemory.kinic.xyz%2Fcallback").kind).toBe("error");
    expect(parseCliLoginHash("#public_key=abc&callback=http%3A%2F%2Flocalhost%3A1234%2Fcallback").kind).toBe("error");
  });
});
