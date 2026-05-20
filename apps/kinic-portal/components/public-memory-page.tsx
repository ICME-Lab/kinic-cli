// Where: interactive route shell for the public memory page.
// What: fetches read-only memory details from the portal Worker, then renders the resolved view.
// Why: same-origin detail removes avoidable cross-origin failures during normal page loads.

import { useEffect, useState } from "react";
import type { MemoryShowResponse } from "@kinic/kinic-share";
import { LoaderCircle } from "lucide-react";
import { MemoryAccessDenied } from "@/components/memory-access-denied";
import { MemoryNotFound } from "@/components/memory-not-found";
import { MemoryTemporaryError } from "@/components/memory-temporary-error";
import { MemoryView } from "@/components/memory-view";
import type { InitialMemoryRouteState } from "@/src/runtime-config";

type MemoryLoadState =
  | { kind: "loading" }
  | { kind: "ready"; memory: MemoryShowResponse }
  | { kind: "denied" }
  | { kind: "not_found" }
  | { kind: "temporary_error" };

export function PublicMemoryPage({
  memoryId,
  initialState,
  mcpEndpoint,
  publicApiOrigin,
}: {
  memoryId: string;
  initialState: InitialMemoryRouteState | null;
  mcpEndpoint: string | null;
  publicApiOrigin: string;
}) {
  const [state, setState] = useState<MemoryLoadState>(() => initialStateToLoadState(initialState));

  useEffect(() => {
    if (initialState) {
      return undefined;
    }
    const controller = new AbortController();
    setState({ kind: "loading" });

    void (async () => {
      try {
        const response = await fetch(`/api/public/memories/${memoryId}`, {
          method: "GET",
          signal: controller.signal,
        });
        if (response.status === 403) {
          setState({ kind: "denied" });
          return;
        }
        if (response.status === 404 || response.status === 400) {
          setState({ kind: "not_found" });
          return;
        }
        if (response.status === 503) {
          setState({ kind: "temporary_error" });
          return;
        }
        const payload = parseMemoryPayload(await response.json());
        if (!response.ok || !payload.memory) {
          throw new Error(payload.error || "memory unavailable");
        }
        setState({ kind: "ready", memory: payload.memory });
      } catch {
        if (!controller.signal.aborted) {
          setState({ kind: "temporary_error" });
        }
      }
    })();

    return () => controller.abort();
  }, [initialState, memoryId]);

  if (state.kind === "loading") {
    return <MemoryLoading memoryId={memoryId} />;
  }
  if (state.kind === "denied") {
    return <MemoryAccessDenied memoryId={memoryId} />;
  }
  if (state.kind === "not_found") {
    return <MemoryNotFound memoryId={memoryId} />;
  }
  if (state.kind === "temporary_error") {
    return <MemoryTemporaryError memoryId={memoryId} />;
  }
  return <MemoryView memory={state.memory} mcpEndpoint={mcpEndpoint} publicApiOrigin={publicApiOrigin} />;
}

function MemoryLoading({ memoryId }: { memoryId: string }) {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <section className="hero-wash rounded-[32px] border border-border px-6 py-8 md:px-10 md:py-12">
        <div className="max-w-4xl space-y-5">
          <div className="flex flex-wrap items-center gap-3">
            <span className="inline-flex items-center gap-2 rounded-full border border-border bg-background/80 px-3 py-1 text-xs uppercase tracking-[0.16em] text-muted-foreground">
              <LoaderCircle className="size-3 animate-spin" />
              Loading
            </span>
          </div>
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">Memory ID</p>
          <p className="font-mono text-sm text-foreground/80">{memoryId}</p>
          <div className="space-y-3">
            <div className="h-6 w-48 rounded-full bg-muted/80 animate-pulse" />
            <div className="h-14 w-full max-w-3xl rounded-[20px] bg-muted/70 animate-pulse" />
            <div className="h-14 w-[80%] max-w-2xl rounded-[20px] bg-muted/60 animate-pulse" />
          </div>
        </div>
      </section>
    </main>
  );
}

function parseMemoryPayload(value: unknown): { memory: MemoryShowResponse | null; error?: string } {
  const record = toRecord(value);
  if (!record) {
    return { memory: null, error: "invalid response" };
  }
  return {
    memory: parseMemoryShowResponse(record),
    error: typeof record.error === "string" ? record.error : undefined,
  };
}

function parseMemoryShowResponse(value: Record<string, unknown>): MemoryShowResponse | null {
  if (
    typeof value.memory_id !== "string"
    || typeof value.name !== "string"
    || (typeof value.description !== "string" && value.description !== null)
    || typeof value.version !== "string"
    || typeof value.dim !== "number"
    || !Array.isArray(value.owners)
    || typeof value.stable_memory_size !== "number"
    || typeof value.cycle_amount !== "string"
  ) {
    return null;
  }

  const owners = value.owners.filter((owner): owner is string => typeof owner === "string");
  return {
    memory_id: value.memory_id,
    name: value.name,
    description: value.description,
    version: value.version,
    dim: value.dim,
    owners,
    stable_memory_size: value.stable_memory_size,
    cycle_amount: value.cycle_amount,
  };
}

function toRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}

function initialStateToLoadState(initialState: InitialMemoryRouteState | null): MemoryLoadState {
  if (!initialState) {
    return { kind: "loading" };
  }
  if (initialState.kind === "temporary_error") {
    return { kind: "temporary_error" };
  }
  return { kind: initialState.kind };
}
