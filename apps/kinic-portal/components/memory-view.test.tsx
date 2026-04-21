// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryView } from "./memory-view";
import { DEV_VITE_SHELL_CHAT_ERROR } from "@/src/runtime-config";

vi.mock("./memory-summary", () => ({
  MemorySummary: () => <div>summary stub</div>,
}));

function jsonResponse(body: unknown, init?: ResponseInit): Response {
  return new Response(JSON.stringify(body), {
    status: init?.status ?? 200,
    headers: { "content-type": "application/json", ...init?.headers },
  });
}

const memory = {
  memory_id: "m1",
  name: "Skill Store",
  description: "Shared notes",
  version: "0.2.5",
  dim: 1536,
  owners: ["user"],
  stable_memory_size: 10,
  cycle_amount: 20,
};

describe("MemoryView", () => {
  const fetchMock = vi.fn<typeof fetch>();
  const openMock = vi.fn();

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal("open", openMock);
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { href: "https://portal.example.com/m/m1", origin: "https://portal.example.com" },
    });
    Object.defineProperty(window.navigator, "language", {
      configurable: true,
      value: "en-US",
    });
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("submits chat queries to the public API worker", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({
        answer: "Grounded answer",
        context_count: 3,
      }),
    );

    render(
      <MemoryView
        memory={memory}
        mcpEndpoint="https://mcp.example.com/mcp"
        publicApiOrigin="https://api.example.com"
      />,
    );

    fireEvent.change(screen.getByPlaceholderText("Ask this public memory"), {
      target: { value: "What is here?" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Chat" }));

    await screen.findByText("Grounded answer");
    expect(fetchMock).toHaveBeenCalledWith("https://api.example.com/api/public/memories/m1/chat", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ query: "What is here?", language: "en-US" }),
    });
  });

  it("renders request errors from the public API worker", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ error: "chat unavailable right now" }, { status: 502 }),
    );

    render(
      <MemoryView
        memory={memory}
        mcpEndpoint={null}
        publicApiOrigin="https://api.example.com"
      />,
    );

    fireEvent.change(screen.getByPlaceholderText("Ask this public memory"), {
      target: { value: "What is here?" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Chat" }));

    await waitFor(() => {
      expect(screen.getByText("chat unavailable right now")).toBeTruthy();
    });
  });

  it("blocks chat in dev:vite-shell before any network call", async () => {
    render(
      <MemoryView
        memory={memory}
        mcpEndpoint={null}
        publicApiOrigin="https://portal.example.com"
      />,
    );

    fireEvent.change(screen.getByPlaceholderText("Ask this public memory"), {
      target: { value: "What is here?" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Chat" }));

    await waitFor(() => {
      expect(screen.getByText(DEV_VITE_SHELL_CHAT_ERROR)).toBeTruthy();
    });
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
