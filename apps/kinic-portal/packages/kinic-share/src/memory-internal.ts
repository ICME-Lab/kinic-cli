// Where: internal helpers behind the shared public-memory entrypoints.
// What: centralizes runtime error classification, retry logic, anonymous probing, and summary shaping.
// Why: keep the package public API narrow while preserving focused unit coverage for the internal rules.

import { parseMemoryNameFields } from "./metadata";

export const TRANSIENT_QUERY_ERROR = "temporary network error";

export type DbMetadata = {
  owners: string[];
  name: string;
  stable_memory_size: number;
  version: string;
  cycle_amount: bigint;
};

export type AnonymousAccessResult =
  | { accessible: true }
  | { accessible: false; error: "memory not found" | "anonymous access denied" | typeof TRANSIENT_QUERY_ERROR };

export type RuntimeErrorKind = "not_found" | "denied" | "transient" | "unknown";

export function classifyPublicMemoryRuntimeError(error: unknown): RuntimeErrorKind {
  const message = extractErrorMessage(error).toLowerCase();
  if (message.includes("permission denied") || message.includes("invalid user")) {
    return "denied";
  }
  if (message.includes("invalid certificate") || message.includes("invalid signature")) {
    return "transient";
  }
  if (
    [
      "canister not found",
      "could not find canister",
      "destination invalid",
      "query method does not exist",
      "has no query method",
      "method not found",
      "failed to decode",
      "cannot decode",
      "decode error",
    ].some((pattern) => message.includes(pattern))
  ) {
    return "not_found";
  }
  return "unknown";
}

export async function probeAnonymousAccess(getName: () => Promise<string>): Promise<AnonymousAccessResult> {
  try {
    await retryTransientQuery(getName);
    return { accessible: true };
  } catch (error) {
    const kind = classifyPublicMemoryRuntimeError(error);
    if (kind === "not_found") {
      return { accessible: false, error: "memory not found" };
    }
    if (kind === "denied") {
      return { accessible: false, error: "anonymous access denied" };
    }
    if (kind === "transient") {
      return { accessible: false, error: TRANSIENT_QUERY_ERROR };
    }
    throw error;
  }
}

export async function retryTransientQuery<T>(call: () => Promise<T>): Promise<T> {
  try {
    return await call();
  } catch (error) {
    if (classifyPublicMemoryRuntimeError(error) !== "transient") {
      throw error;
    }
  }
  return call();
}

export function summarizeMemory(memoryId: string, metadata: DbMetadata) {
  const { name, description } = parseMemoryNameFields(metadata.name);
  return {
    memory_id: memoryId,
    name,
    description,
    version: metadata.version,
  };
}

function extractErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return "";
  }
}
