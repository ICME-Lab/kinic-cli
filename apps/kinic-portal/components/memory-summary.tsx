"use client";

// Where: client component inside the public memory hero.
// What: fetches an AI summary on mount and renders loading, success, or fallback states.
// Why: the page should feel fast while still exposing a concise overview of the memory contents.

import { LoaderCircle } from "lucide-react";
import { useEffect, useState } from "react";

type SummaryState =
  | { kind: "loading" }
  | { kind: "ready"; summary: string }
  | { kind: "error" };

type SummaryPayload = {
  summary: string;
  cached: boolean;
  updatedAt: string;
  error?: string;
};

export function MemorySummary({ memoryId }: { memoryId: string }) {
  const [state, setState] = useState<SummaryState>({ kind: "loading" });

  useEffect(() => {
    const controller = new AbortController();
    const language = typeof navigator === "object" && navigator.language ? navigator.language : "en";
    setState({ kind: "loading" });

    void (async () => {
      try {
        const response = await fetch(`/api/memories/${memoryId}/summary?language=${encodeURIComponent(language)}`, {
          method: "GET",
          signal: controller.signal,
        });
        const payload = parseSummaryPayload(await response.json());
        if (!response.ok || !payload.summary) {
          throw new Error(payload.error || "summary unavailable");
        }
        setState({ kind: "ready", summary: payload.summary });
      } catch (error) {
        if (controller.signal.aborted) {
          return;
        }
        setState({ kind: "error" });
      }
    })();

    return () => controller.abort();
  }, [memoryId]);

  return (
    <section className="w-full rounded-[28px] border border-border/80 bg-background/70 px-5 py-4 shadow-[0_1px_2px_rgba(0,0,0,0.04)] backdrop-blur">
      <div className="flex items-center gap-2">
        <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Summary</p>
        {state.kind === "loading" ? <LoaderCircle className="size-3.5 animate-spin text-muted-foreground" /> : null}
      </div>
      {state.kind === "ready" ? (
        <p className="mt-3 text-sm leading-7 text-foreground md:text-[15px]">{state.summary}</p>
      ) : null}
      {state.kind === "loading" ? (
        <div className="mt-4 space-y-2.5">
          <div className="h-3 rounded-full bg-muted/80 animate-pulse" />
          <div className="h-3 w-[92%] rounded-full bg-muted/80 animate-pulse" />
          <div className="h-3 w-[78%] rounded-full bg-muted/80 animate-pulse" />
        </div>
      ) : null}
      {state.kind === "error" ? (
        <p className="mt-3 text-sm leading-7 text-muted-foreground">Summary unavailable right now.</p>
      ) : null}
    </section>
  );
}

function parseSummaryPayload(value: unknown): SummaryPayload {
  const record = toRecord(value);
  if (!record) {
    return { summary: "", cached: false, updatedAt: "", error: "invalid response" };
  }

  return {
    summary: typeof record.summary === "string" ? record.summary : "",
    cached: typeof record.cached === "boolean" ? record.cached : false,
    updatedAt: typeof record.updatedAt === "string" ? record.updatedAt : "",
    error: typeof record.error === "string" ? record.error : undefined,
  };
}

function toRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}
