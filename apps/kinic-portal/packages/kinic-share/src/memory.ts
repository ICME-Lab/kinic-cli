// Where: shared by the public API routes and the remote MCP Worker.
// What: implements the read-only Kinic memory calls needed by Kinic portal v1.
// Why: keep anonymous access checks and response shaping identical across public surfaces.

import { Actor, type ActorMethod, type HttpAgent } from "@dfinity/agent";
import { IDL } from "@dfinity/candid";
import { Principal } from "@dfinity/principal";
import { resolveEmbeddingApiEndpoint, type SharedRuntimeEnv } from "./config";
import { parseMemoryNameFields } from "./metadata";
import {
  classifyPublicMemoryRuntimeError,
  probeAnonymousAccess,
  retryTransientQuery,
  summarizeMemory,
  TRANSIENT_QUERY_ERROR,
  type AnonymousAccessResult,
  type DbMetadata,
} from "./memory-internal";

type MemoryActor = { get_dim: ActorMethod<[], bigint>; get_name: ActorMethod<[], string>; get_metadata: ActorMethod<[], DbMetadata>; search: ActorMethod<[number[]], Array<[number, string]>> };

export type MemoryShowResponse = {
  memory_id: string;
  name: string;
  description: string | null;
  version: string;
  dim: number;
  owners: string[];
  stable_memory_size: number;
  cycle_amount: number;
};

type MemorySummaryResponse = { memory_id: string; name: string; description: string | null; version: string };
type PublicMemoryError = "invalid memory id" | "memory not found" | "anonymous access denied" | typeof TRANSIENT_QUERY_ERROR;
type PublicMemoryState<T> = { kind: "accessible"; memory: T } | { kind: "invalid"; error: "invalid memory id" } | { kind: "not_found"; error: "memory not found" } | { kind: "transient_error"; error: typeof TRANSIENT_QUERY_ERROR } | { kind: "denied"; error: "anonymous access denied" };
type PublicMemoryDetailsState = PublicMemoryState<MemoryShowResponse>;
type PublicMemorySummaryState = PublicMemoryState<MemorySummaryResponse>;

export function isValidPrincipalText(value: string): boolean {
  try {
    Principal.fromText(value);
    return true;
  } catch {
    return false;
  }
}

function isRuntimeErrorKind(error: unknown, kind: Exclude<ReturnType<typeof classifyPublicMemoryRuntimeError>, "unknown">): boolean {
  return classifyPublicMemoryRuntimeError(error) === kind;
}

async function getMemoryDetails(agent: HttpAgent, memoryId: string): Promise<MemoryShowResponse> {
  const actor = createMemoryActor(agent, memoryId);
  const [metadata, dim] = await Promise.all([
    retryTransientQuery(() => actor.get_metadata()),
    retryTransientQuery(() => actor.get_dim()),
  ]);
  const { name, description } = parseMemoryNameFields(metadata.name);
  return {
    memory_id: memoryId,
    name,
    description,
    version: metadata.version,
    dim: Number(dim),
    owners: metadata.owners,
    stable_memory_size: metadata.stable_memory_size,
    cycle_amount: Number(metadata.cycle_amount),
  };
}

async function getMemorySummary(agent: HttpAgent, memoryId: string): Promise<MemorySummaryResponse> {
  const metadata = await retryTransientQuery(() => createMemoryActor(agent, memoryId).get_metadata());
  return summarizeMemory(memoryId, metadata);
}

async function checkAnonymousAccess(agent: HttpAgent, memoryId: string): Promise<AnonymousAccessResult> {
  return probeAnonymousAccess(() => createMemoryActor(agent, memoryId).get_name());
}

async function getPublicMemory(agent: HttpAgent, memoryId: string): Promise<MemoryShowResponse> {
  return getMemoryDetails(agent, memoryId);
}

export async function resolvePublicMemoryDetails(agent: HttpAgent, memoryId: string): Promise<PublicMemoryDetailsState> {
  return resolvePublicMemoryState(memoryId, () => checkAnonymousAccess(agent, memoryId), () => getPublicMemory(agent, memoryId));
}

export async function resolvePublicMemorySummary(agent: HttpAgent, memoryId: string): Promise<PublicMemorySummaryState> {
  return resolvePublicMemoryState(memoryId, () => checkAnonymousAccess(agent, memoryId), () => getMemorySummary(agent, memoryId));
}

export async function searchMemory(agent: HttpAgent, memoryId: string, embedding: number[]): Promise<Array<{ score: number; payload: string }>> {
  const rows = await retryTransientQuery(() => createMemoryActor(agent, memoryId).search(embedding));
  return rows
    .map(([score, payload]) => ({ score, payload }))
    .sort((left, right) => right.score - left.score);
}

export async function fetchEmbedding(text: string, env: SharedRuntimeEnv): Promise<number[]> {
  const response = await fetch(`${resolveEmbeddingApiEndpoint(env)}/embedding`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ content: text }),
  });
  if (!response.ok) {
    throw new Error(`embedding request failed with status ${response.status}`);
  }
  const payload = parseRecord(await response.json());
  if (!payload || !Array.isArray(payload.embedding)) {
    throw new Error("Invalid embedding response.");
  }
  return payload.embedding.filter((value): value is number => typeof value === "number");
}

export async function callChatApi(prompt: string, env: SharedRuntimeEnv): Promise<string> {
  const response = await fetch(`${resolveEmbeddingApiEndpoint(env)}/chat`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ message: prompt }) });
  if (!response.ok) {
    throw new Error(`chat request failed with status ${response.status}`);
  }
  return response.text();
}

function createMemoryActor(agent: HttpAgent, memoryId: string): MemoryActor {
  return Actor.createActor<MemoryActor>(memoryIdlFactory, {
    agent,
    canisterId: Principal.fromText(memoryId),
  });
}

function parseRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}

async function resolvePublicMemoryState<T>(memoryId: string, checkAccess: () => Promise<AnonymousAccessResult>, loadMemory: () => Promise<T>): Promise<PublicMemoryState<T>> {
  if (!isValidPrincipalText(memoryId)) {
    return { kind: "invalid", error: "invalid memory id" };
  }

  const access = await checkAccess();
  if (!access.accessible) {
    if (access.error === "memory not found") {
      return { kind: "not_found", error: access.error };
    }
    if (access.error === TRANSIENT_QUERY_ERROR) {
      return { kind: "transient_error", error: access.error };
    }
    return { kind: "denied", error: access.error };
  }

  try {
    return { kind: "accessible", memory: await loadMemory() };
  } catch (error) {
    if (isRuntimeErrorKind(error, "not_found")) {
      return { kind: "not_found", error: "memory not found" };
    }
    if (isRuntimeErrorKind(error, "denied")) {
      return { kind: "denied", error: "anonymous access denied" };
    }
    if (isRuntimeErrorKind(error, "transient")) {
      return { kind: "transient_error", error: TRANSIENT_QUERY_ERROR };
    }
    throw error;
  }
}

const memoryIdlFactory: IDL.InterfaceFactory = ({ IDL: Types }) =>
  Types.Service({
    get_dim: Types.Func([], [Types.Nat64], ["query"]),
    get_name: Types.Func([], [Types.Text], ["query"]),
    get_metadata: Types.Func(
      [],
      [
        Types.Record({
          is_complete_hnsw_chunks: Types.Bool,
          owners: Types.Vec(Types.Text),
          name: Types.Text,
          is_deserialized: Types.Bool,
          stable_memory_size: Types.Nat32,
          version: Types.Text,
          cycle_amount: Types.Nat64,
          db_key: Types.Text,
          is_complete_source_chunks: Types.Bool,
        }),
      ],
      ["query"],
    ),
    search: Types.Func([Types.Vec(Types.Float32)], [Types.Vec(Types.Tuple(Types.Float32, Types.Text))], ["query"]),
  });
