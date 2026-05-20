// @vitest-environment jsdom

// Where: client bootstrap tests for the portal shell.
// What: verifies hydration only runs for server-rendered roots.
// Why: the Vite shell starts with an empty root and must use client rendering.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  createRoot: vi.fn(),
  hydrateRoot: vi.fn(),
  render: vi.fn(),
}));

vi.mock("react-dom/client", () => ({
  createRoot: mocks.createRoot,
  hydrateRoot: mocks.hydrateRoot,
}));

vi.mock("./app", () => ({
  App: () => <div>app</div>,
}));

vi.mock("./runtime-config", () => ({
  readRuntimeConfig: () => ({
    portalOrigin: "https://portal.kinic.test",
    publicApiOrigin: "https://api.kinic.test",
    mcpEndpoint: null,
    icHost: "https://ic0.app",
    remoteCanisterMcpCanisterId: null,
    initialMemoryState: null,
  }),
}));

describe("client bootstrap", () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    mocks.createRoot.mockReturnValue({ render: mocks.render });
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("client-renders an empty Vite shell root", async () => {
    document.body.innerHTML = '<div id="root"></div>';

    await import("./client");

    expect(mocks.createRoot).toHaveBeenCalledWith(document.getElementById("root"));
    expect(mocks.render).toHaveBeenCalledOnce();
    expect(mocks.hydrateRoot).not.toHaveBeenCalled();
  });

  it("hydrates a root that already contains SSR HTML", async () => {
    document.body.innerHTML = '<div id="root"><main>SSR</main></div>';

    await import("./client");

    expect(mocks.hydrateRoot).toHaveBeenCalledWith(document.getElementById("root"), expect.anything());
    expect(mocks.createRoot).not.toHaveBeenCalled();
  });
});
