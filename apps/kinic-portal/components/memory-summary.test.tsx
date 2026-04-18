// @vitest-environment jsdom

import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemorySummary } from "./memory-summary";

type DeferredResponse = {
  promise: Promise<Response>;
  resolve: (response: Response) => void;
  reject: (error: unknown) => void;
};

function deferredResponse(): DeferredResponse {
  let resolve!: (response: Response) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<Response>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

function jsonResponse(body: unknown, init?: ResponseInit): Response {
  return new Response(JSON.stringify(body), {
    status: init?.status ?? 200,
    headers: { "content-type": "application/json", ...init?.headers },
  });
}

describe("MemorySummary", () => {
  const fetchMock = vi.fn<typeof fetch>();

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock);
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

  it("fetches one summary on first render", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({
        summary: "Alpha summary",
        cached: false,
        updatedAt: "2026-04-17T00:00:00.000Z",
      }),
    );

    render(<MemorySummary memoryId="alpha" />);

    await screen.findByText("Alpha summary");
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock).toHaveBeenCalledWith("/api/memories/alpha/summary?language=en-US", {
      method: "GET",
      signal: expect.any(AbortSignal),
    });
  });

  it("refetches and resets to loading when memoryId changes", async () => {
    fetchMock
      .mockResolvedValueOnce(
        jsonResponse({
          summary: "Alpha summary",
          cached: false,
          updatedAt: "2026-04-17T00:00:00.000Z",
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          summary: "Beta summary",
          cached: false,
          updatedAt: "2026-04-17T00:00:01.000Z",
        }),
      );

    const view = render(<MemorySummary memoryId="alpha" />);

    await screen.findByText("Alpha summary");

    view.rerender(<MemorySummary memoryId="beta" />);

    expect(screen.queryByText("Alpha summary")).toBeNull();
    await screen.findByText("Beta summary");
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock).toHaveBeenNthCalledWith(2, "/api/memories/beta/summary?language=en-US", {
      method: "GET",
      signal: expect.any(AbortSignal),
    });
  });

  it("ignores the stale response after navigating to another memory", async () => {
    const alpha = deferredResponse();
    const beta = deferredResponse();
    fetchMock
      .mockImplementationOnce((_input, init) => {
        const signal = init?.signal;
        signal?.addEventListener("abort", () => {
          alpha.reject(new DOMException("Aborted", "AbortError"));
        });
        return alpha.promise;
      })
      .mockImplementationOnce(() => beta.promise);

    const view = render(<MemorySummary memoryId="alpha" />);
    view.rerender(<MemorySummary memoryId="beta" />);

    await act(async () => {
      beta.resolve(
        jsonResponse({
          summary: "Beta summary",
          cached: false,
          updatedAt: "2026-04-17T00:00:02.000Z",
        }),
      );
    });

    await screen.findByText("Beta summary");
    expect(screen.queryByText("Alpha summary")).toBeNull();
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(2);
    });
  });

  it("recovers from an error after navigating to another memory", async () => {
    fetchMock
      .mockResolvedValueOnce(jsonResponse({ error: "summary unavailable" }, { status: 502 }))
      .mockResolvedValueOnce(
        jsonResponse({
          summary: "Recovered summary",
          cached: false,
          updatedAt: "2026-04-17T00:00:03.000Z",
        }),
      );

    const view = render(<MemorySummary memoryId="alpha" />);

    await screen.findByText("Summary unavailable right now.");

    view.rerender(<MemorySummary memoryId="beta" />);

    expect(screen.queryByText("Summary unavailable right now.")).toBeNull();
    await screen.findByText("Recovered summary");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});
