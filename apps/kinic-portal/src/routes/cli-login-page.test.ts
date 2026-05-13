import { AnonymousIdentity } from "@dfinity/agent";
import { describe, expect, it, vi } from "vitest";
import {
  type CliLoginAuthClient,
  isAllowedLocalCallback,
  parseCliLoginHash,
  sendCliLoginDelegation,
} from "./cli-login-page";

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

describe("cli login flow", () => {
  it("logs out after a successful callback", async () => {
    const logout = vi.fn<() => Promise<void>>().mockResolvedValue(undefined);
    const fetchCallback: typeof fetch = async () => new Response(null, { status: 200 });

    await sendCliLoginDelegation(params(), {
      createAuthClient: async () => authClient(logout),
      loginClient: async () => undefined,
      createDelegation: async () => ({ ok: true }),
      fetchCallback,
    });

    expect(logout).toHaveBeenCalledOnce();
  });

  it("logs out after a callback network failure", async () => {
    const logout = vi.fn<() => Promise<void>>().mockResolvedValue(undefined);
    const fetchCallback: typeof fetch = async () => {
      throw new Error("network failed");
    };

    await expect(sendCliLoginDelegation(params(), {
      createAuthClient: async () => authClient(logout),
      loginClient: async () => undefined,
      createDelegation: async () => ({ ok: true }),
      fetchCallback,
    })).rejects.toThrow("network failed");
    expect(logout).toHaveBeenCalledOnce();
  });

  it("logs out after a non-2xx callback and preserves the callback error", async () => {
    const logout = vi.fn<() => Promise<void>>().mockRejectedValue(new Error("logout failed"));
    const fetchCallback: typeof fetch = async () => new Response(null, {
      status: 500,
      statusText: "Nope",
    });

    await expect(sendCliLoginDelegation(params(), {
      createAuthClient: async () => authClient(logout),
      loginClient: async () => undefined,
      createDelegation: async () => ({ ok: true }),
      fetchCallback,
    })).rejects.toThrow("Callback failed: 500 Nope");
    expect(logout).toHaveBeenCalledOnce();
  });
});

function authClient(logout: () => Promise<void>): CliLoginAuthClient {
  return {
    getIdentity: () => new AnonymousIdentity(),
    login: async () => undefined,
    logout,
  };
}

function params() {
  return {
    publicKey: "abc",
    callback: "http://127.0.0.1:8620/callback",
  };
}
