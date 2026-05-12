// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryPage } from "@/src/routes/memory-page";
import { PublicMemoryPage } from "./public-memory-page";

const memoryViewMock = vi.fn();

vi.mock("./memory-view", () => ({
  MemoryView: (props: { memory: { memory_id: string } }) => {
    memoryViewMock(props);
    const [draft, setDraft] = useState(props.memory.memory_id);
    return (
      <div>
        <div>memory ready</div>
        <div>draft:{draft}</div>
        <button type="button" onClick={() => setDraft(`dirty:${props.memory.memory_id}`)}>
          mutate draft
        </button>
      </div>
    );
  },
}));

const useParamsMock = vi.fn();

vi.mock("react-router", () => ({
  useParams: () => useParamsMock(),
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
    useParamsMock.mockReset();
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
        cycle_amount: "20",
      }),
    );

    render(
      <PublicMemoryPage
        memoryId="m1"
        initialState={null}
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
        initialState={null}
        publicApiOrigin="https://api.kinic.test"
        mcpEndpoint={null}
      />,
    );

    await screen.findByText("Anonymous access is blocked.");
  });

  it("keeps injected denied state without returning to loading", () => {
    render(
      <PublicMemoryPage
        memoryId="m1"
        initialState={{ kind: "denied", memoryId: "m1" }}
        publicApiOrigin="https://api.kinic.test"
        mcpEndpoint={null}
      />,
    );

    expect(screen.getByText("Anonymous access is blocked.")).toBeTruthy();
    expect(screen.queryByText("Loading")).toBeNull();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("keeps injected not found state without returning to loading", () => {
    render(
      <PublicMemoryPage
        memoryId="m1"
        initialState={{ kind: "not_found", memoryId: "m1" }}
        publicApiOrigin="https://api.kinic.test"
        mcpEndpoint={null}
      />,
    );

    expect(screen.getByText("Shared memory not found.")).toBeTruthy();
    expect(screen.queryByText("Loading")).toBeNull();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("keeps injected temporary error state without returning to loading", () => {
    render(
      <PublicMemoryPage
        memoryId="m1"
        initialState={{ kind: "temporary_error", memoryId: "m1" }}
        publicApiOrigin="https://api.kinic.test"
        mcpEndpoint={null}
      />,
    );

    expect(screen.getByText("Temporary network error")).toBeTruthy();
    expect(screen.queryByText("Loading")).toBeNull();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("remounts PublicMemoryPage when the route memory id changes", async () => {
    fetchMock
      .mockResolvedValueOnce(
        jsonResponse({
          memory_id: "m1",
          name: "Skill Store",
          description: "Shared notes",
          version: "0.2.5",
          dim: 1536,
          owners: ["user"],
          stable_memory_size: 10,
          cycle_amount: "20",
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          memory_id: "m2",
          name: "Second Memory",
          description: "Other notes",
          version: "0.2.6",
          dim: 1536,
          owners: ["user"],
          stable_memory_size: 11,
          cycle_amount: "21",
        }),
      );

    useParamsMock.mockReturnValue({ memoryId: "m1" });
    const view = render(
      <MemoryPage
        config={{
          portalOrigin: "https://memory.kinic.xyz",
          publicApiOrigin: "https://api.kinic.test",
          mcpEndpoint: "https://mcp.kinic.test/mcp",
          initialMemoryState: null,
        }}
      />,
    );

    await screen.findByText("draft:m1");
    fireEvent.click(screen.getByRole("button", { name: "mutate draft" }));
    expect(screen.getByText("draft:dirty:m1")).toBeTruthy();

    useParamsMock.mockReturnValue({ memoryId: "m2" });
    view.rerender(
      <MemoryPage
        config={{
          portalOrigin: "https://memory.kinic.xyz",
          publicApiOrigin: "https://api.kinic.test",
          mcpEndpoint: "https://mcp.kinic.test/mcp",
          initialMemoryState: null,
        }}
      />,
    );

    await screen.findByText("draft:m2");
    expect(screen.queryByText("draft:dirty:m1")).toBeNull();
  });

  it("ignores injected initialState when it belongs to another memory id", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({
        memory_id: "m2",
        name: "Second Memory",
        description: "Other notes",
        version: "0.2.6",
        dim: 1536,
        owners: ["user"],
        stable_memory_size: 11,
        cycle_amount: "21",
      }),
    );

    useParamsMock.mockReturnValue({ memoryId: "m2" });
    render(
      <MemoryPage
        config={{
          portalOrigin: "https://memory.kinic.xyz",
          publicApiOrigin: "https://api.kinic.test",
          mcpEndpoint: "https://mcp.kinic.test/mcp",
          initialMemoryState: { kind: "denied", memoryId: "m1" },
        }}
      />,
    );

    expect(screen.queryByText("Anonymous access is blocked.")).toBeNull();
    await screen.findByText("draft:m2");
  });

  it("rejects memory detail responses with numeric cycle amounts", async () => {
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
        initialState={null}
        publicApiOrigin="https://api.kinic.test"
        mcpEndpoint={null}
      />,
    );

    await screen.findByText("Temporary network error");
  });
});
