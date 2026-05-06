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
  const writeTextMock = vi.fn();

  beforeEach(() => {
    writeTextMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal("open", openMock);
    Object.defineProperty(window.navigator, "clipboard", {
      configurable: true,
      value: { writeText: writeTextMock },
    });
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

  it("keeps share controls disabled until an absolute browser URL is available", () => {
    Object.defineProperty(window, "location", {
      configurable: true,
      value: { href: "", origin: "https://portal.example.com" },
    });

    render(
      <MemoryView
        memory={memory}
        mcpEndpoint={null}
        publicApiOrigin="https://api.example.com"
      />,
    );

    expect(screen.getByRole("button", { name: "Share on X" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Copy share URL" })).toHaveProperty("disabled", true);
    fireEvent.click(screen.getByRole("button", { name: "Copy share URL" }));
    expect(writeTextMock).not.toHaveBeenCalled();
  });

  it("shares the absolute browser URL after it is available", async () => {
    writeTextMock.mockResolvedValueOnce(undefined);

    render(
      <MemoryView
        memory={memory}
        mcpEndpoint={null}
        publicApiOrigin="https://api.example.com"
      />,
    );

    const shareOnX = await screen.findByRole("link", { name: "Share on X" });
    expect(shareOnX.getAttribute("href")).toEqual(
      expect.stringContaining(encodeURIComponent("https://portal.example.com/m/m1")),
    );

    fireEvent.click(screen.getByRole("button", { name: "Copy share URL" }));

    await waitFor(() => {
      expect(writeTextMock).toHaveBeenCalledWith("https://portal.example.com/m/m1");
    });
  });

  it("shows share copy failures without the mcp card", async () => {
    writeTextMock.mockRejectedValueOnce(new Error("blocked"));

    render(
      <MemoryView
        memory={memory}
        mcpEndpoint={null}
        publicApiOrigin="https://api.example.com"
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Copy share URL" }));

    await waitFor(() => {
      expect(screen.getByText("Clipboard unavailable")).toBeTruthy();
    });
  });
});
