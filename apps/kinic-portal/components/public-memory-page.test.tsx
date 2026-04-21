// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PublicMemoryPage } from "./public-memory-page";

const memoryViewMock = vi.fn();

vi.mock("./memory-view", () => ({
  MemoryView: (props: unknown) => {
    memoryViewMock(props);
    return <div>memory ready</div>;
  },
}));

function jsonResponse(body: unknown, init?: ResponseInit): Response {
  return new Response(JSON.stringify(body), {
    status: init?.status ?? 200,
    headers: { "content-type": "application/json", ...init?.headers },
  });
}

describe("PublicMemoryPage", () => {
  const fetchMock = vi.fn<typeof fetch>();

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock);
    memoryViewMock.mockReset();
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("loads memory details from the portal Worker", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({
        memory_id: "m1",
        name: "Skill Store",
        description: "Shared notes",
        version: "0.2.5",
        dim: 1536,
        owners: ["user"],
        stable_memory_size: 10,
        cycle_amount: 20,
      }),
    );

    render(
      <PublicMemoryPage
        memoryId="m1"
        publicApiOrigin="https://api.kinic.test"
        mcpEndpoint="https://mcp.kinic.test/mcp"
      />,
    );

    await screen.findByText("memory ready");
    expect(fetchMock).toHaveBeenCalledWith("/api/public/memories/m1", {
      method: "GET",
      signal: expect.any(AbortSignal),
    });
  });

  it("renders access denied for 403", async () => {
    fetchMock.mockResolvedValueOnce(jsonResponse({ error: "anonymous access denied" }, { status: 403 }));

    render(
      <PublicMemoryPage
        memoryId="m1"
        publicApiOrigin="https://api.kinic.test"
        mcpEndpoint={null}
      />,
    );

    await screen.findByText("Anonymous access is blocked.");
  });
});
