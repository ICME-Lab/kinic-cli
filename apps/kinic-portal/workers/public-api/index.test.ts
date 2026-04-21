import { describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  handleMemoryChat: vi.fn(),
  handleMemoryOgp: vi.fn(),
  handleSiteOgp: vi.fn(),
}));

vi.mock("./src/memory-routes", () => ({
  handleMemoryChat: mocks.handleMemoryChat,
}));

vi.mock("./src/ogp", () => ({
  handleMemoryOgp: mocks.handleMemoryOgp,
  handleSiteOgp: mocks.handleSiteOgp,
}));

import app from "./src/index";

const executionCtx = {
  waitUntil() {
    return undefined;
  },
};

describe("public api hono router", () => {
  it("returns cors preflight for options", async () => {
    const response = await app.fetch(new Request("https://api.kinic.test/api/public/memories/m1", { method: "OPTIONS" }), env());

    expect(response.status).toBe(204);
    expect(response.headers.get("Access-Control-Allow-Origin")).toBe("*");
    expect(mocks.handleMemoryChat).not.toHaveBeenCalled();
  });

  it("dispatches memory chat by path param", async () => {
    mocks.handleMemoryChat.mockResolvedValueOnce(new Response("ok"));

    const response = await app.fetch(
      new Request("https://api.kinic.test/api/public/memories/m1/chat", { method: "POST" }),
      env(),
    );

    expect(response.status).toBe(200);
    expect(mocks.handleMemoryChat).toHaveBeenCalledOnce();
  });

  it("dispatches head site ogp without body", async () => {
    mocks.handleSiteOgp.mockResolvedValueOnce(new Response(null, { status: 200, headers: { "content-type": "image/png" } }));

    const response = await app.fetch(
      new Request("https://api.kinic.test/opengraph-image", { method: "HEAD" }),
      env(),
      executionCtx,
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("image/png");
    expect(await response.text()).toBe("");
    expect(mocks.handleSiteOgp).toHaveBeenCalledWith("HEAD", expect.anything());
  });

  it("returns not found for unknown routes", async () => {
    const response = await app.fetch(new Request("https://api.kinic.test/missing"), env());

    expect(response.status).toBe(404);
    expect(await response.text()).toBe("not found");
  });
});

function env(): Env {
  return {
    IC_HOST: "https://ic0.app",
    EMBEDDING_API_ENDPOINT: "https://api.kinic.test",
    SUMMARY_CACHE_TTL_SECONDS: "86400",
  };
}
