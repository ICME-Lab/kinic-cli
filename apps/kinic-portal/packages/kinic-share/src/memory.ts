// Where: shared by the public API routes and the remote MCP Worker.
// What: implements the Kinic memory and launcher canister calls needed by Kinic portal v1.
// Why: keep canister semantics, anonymous visibility checks, and response shaping identical across surfaces.

import { Actor, type ActorMethod, type HttpAgent } from "@dfinity/agent";
import { IDL } from "@dfinity/candid";
import { Principal } from "@dfinity/principal";
import {
  DEFAULT_VECTOR_DIM,
  requireLauncherCanisterId,
  resolveEmbeddingApiEndpoint,
  type SharedRuntimeEnv,
} from "./config";
import { parseMemoryNameFields } from "./metadata";

type DbMetadata = {
  owners: string[];
  name: string;
  stable_memory_size: number;
  version: string;
  cycle_amount: bigint;
};

type MemoryActor = {
  add_new_user: ActorMethod<[Principal, number], void>;
  get_dim: ActorMethod<[], bigint>;
  get_name: ActorMethod<[], string>;
  get_metadata: ActorMethod<[], DbMetadata>;
  get_users: ActorMethod<[], Array<[string, number]>>;
  insert: ActorMethod<[number[], string], number>;
  search: ActorMethod<[number[]], Array<[number, string]>>;
};

type LauncherState =
  | { Empty: string }
  | { Pending: string }
  | { Creation: string }
  | { Installation: [Principal, string] }
  | { SettingUp: Principal }
  | { Running: Principal };

type DeployInstanceResult = { Ok: string } | { Err: unknown };

type LauncherActor = {
  deploy_instance: ActorMethod<[string, bigint], DeployInstanceResult>;
  list_instance: ActorMethod<[], LauncherState[]>;
};

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

export type MemorySummaryResponse = {
  memory_id: string;
  name: string;
  description: string | null;
  version: string;
};

export type PublicMemoryDetailsState =
  | { kind: "accessible"; memory: MemoryShowResponse }
  | { kind: "invalid"; error: "invalid memory id" }
  | { kind: "not_found"; error: "memory not found" }
  | { kind: "transient_error"; error: typeof TRANSIENT_QUERY_ERROR }
  | { kind: "denied"; error: "anonymous access denied" };

export type PublicMemorySummaryState =
  | { kind: "accessible"; memory: MemorySummaryResponse }
  | { kind: "invalid"; error: "invalid memory id" }
  | { kind: "not_found"; error: "memory not found" }
  | { kind: "transient_error"; error: typeof TRANSIENT_QUERY_ERROR }
  | { kind: "denied"; error: "anonymous access denied" };

export type AnonymousAccessResult =
  | { accessible: true }
  | { accessible: false; error: "anonymous access denied" | "memory not found" | "temporary network error" };

export const TRANSIENT_QUERY_ERROR = "temporary network error";

export function isValidPrincipalText(value: string): boolean {
  try {
    Principal.fromText(value);
    return true;
  } catch {
    return false;
  }
}

export function isAnonymousAccessError(error: unknown): boolean {
  const message = extractErrorMessage(error).toLowerCase();
  return message.includes("permission denied") || message.includes("invalid user");
}

export function isTransientQueryError(error: unknown): boolean {
  const message = extractErrorMessage(error).toLowerCase();
  return message.includes("invalid certificate") || message.includes("invalid signature");
}

export function isPublicMemoryNotFoundError(error: unknown): boolean {
  const message = extractErrorMessage(error).toLowerCase();
  return [
    "canister not found",
    "could not find canister",
    "destination invalid",
    "query method does not exist",
    "has no query method",
    "method not found",
    "failed to decode",
    "cannot decode",
    "decode error",
  ].some((pattern) => message.includes(pattern));
}

export async function getMemoryDetails(
  agent: HttpAgent,
  memoryId: string,
): Promise<MemoryShowResponse> {
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

export async function getMemorySummary(
  agent: HttpAgent,
  memoryId: string,
): Promise<MemorySummaryResponse> {
  const metadata = await retryTransientQuery(() => createMemoryActor(agent, memoryId).get_metadata());
  return summarizeMemory(memoryId, metadata);
}

export async function checkAnonymousAccess(
  agent: HttpAgent,
  memoryId: string,
): Promise<AnonymousAccessResult> {
  return probeAnonymousAccess(() => createMemoryActor(agent, memoryId).get_name());
}

export async function getPublicMemory(
  agent: HttpAgent,
  memoryId: string,
): Promise<MemoryShowResponse> {
  return getMemoryDetails(agent, memoryId);
}

export async function resolvePublicMemoryDetails(
  agent: HttpAgent,
  memoryId: string,
): Promise<PublicMemoryDetailsState> {
  return resolvePublicMemoryState(memoryId, () => checkAnonymousAccess(agent, memoryId), () => getPublicMemory(agent, memoryId));
}

export async function resolvePublicMemorySummary(
  agent: HttpAgent,
  memoryId: string,
): Promise<PublicMemorySummaryState> {
  return resolvePublicMemoryState(memoryId, () => checkAnonymousAccess(agent, memoryId), () => getMemorySummary(agent, memoryId));
}

export async function searchMemory(
  agent: HttpAgent,
  memoryId: string,
  embedding: number[],
): Promise<Array<{ score: number; payload: string }>> {
  const rows = await retryTransientQuery(() => createMemoryActor(agent, memoryId).search(embedding));
  return rows
    .map(([score, payload]) => ({ score, payload }))
    .sort((left, right) => right.score - left.score);
}

export async function listMemories(agent: HttpAgent, env: SharedRuntimeEnv): Promise<string[]> {
  const actor = createLauncherActor(agent, requireLauncherCanisterId(env));
  const items = await actor.list_instance();
  return items.flatMap((item) => ("Running" in item ? [item.Running.toText()] : []));
}

export async function createMemory(
  agent: HttpAgent,
  env: SharedRuntimeEnv,
  name: string,
  description: string,
): Promise<string> {
  const actor = createLauncherActor(agent, requireLauncherCanisterId(env));
  const payload = JSON.stringify({ name, description });
  const response = await actor.deploy_instance(payload, DEFAULT_VECTOR_DIM);
  if ("Err" in response) {
    throw new Error(`deploy_instance failed: ${JSON.stringify(response.Err)}`);
  }
  return response.Ok;
}

export async function insertMarkdown(
  agent: HttpAgent,
  env: SharedRuntimeEnv,
  memoryId: string,
  tag: string,
  text: string,
): Promise<{ memory_id: string; tag: string; chunks_inserted: number }> {
  const actor = createMemoryActor(agent, memoryId);
  const chunks = await lateChunk(text, env);
  for (const chunk of chunks) {
    const payload = JSON.stringify({ tag, payload: chunk.sentence });
    await actor.insert(chunk.embedding, payload);
  }
  return { memory_id: memoryId, tag, chunks_inserted: chunks.length };
}

export async function makeMemoryPublic(agent: HttpAgent, memoryId: string): Promise<void> {
  await createMemoryActor(agent, memoryId).add_new_user(Principal.anonymous(), 3);
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
  const response = await fetch(`${resolveEmbeddingApiEndpoint(env)}/chat`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message: prompt }),
  });
  if (!response.ok) {
    throw new Error(`chat request failed with status ${response.status}`);
  }
  return response.text();
}

async function lateChunk(
  text: string,
  env: SharedRuntimeEnv,
): Promise<Array<{ embedding: number[]; sentence: string }>> {
  const response = await fetch(`${resolveEmbeddingApiEndpoint(env)}/late-chunking`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ markdown: text }),
  });
  if (!response.ok) {
    throw new Error(`late chunking request failed with status ${response.status}`);
  }
  const payload = parseRecord(await response.json());
  if (!payload || !Array.isArray(payload.chunks)) {
    throw new Error("Invalid late chunking response.");
  }
  return payload.chunks.flatMap((chunk) => {
    const row = parseRecord(chunk);
    return row && Array.isArray(row.embedding) && typeof row.sentence === "string"
      ? [{ embedding: row.embedding.filter((value): value is number => typeof value === "number"), sentence: row.sentence }]
      : [];
  });
}

function createMemoryActor(agent: HttpAgent, memoryId: string): MemoryActor {
  return Actor.createActor<MemoryActor>(memoryIdlFactory, {
    agent,
    canisterId: Principal.fromText(memoryId),
  });
}

function createLauncherActor(agent: HttpAgent, canisterId: string): LauncherActor {
  return Actor.createActor<LauncherActor>(launcherIdlFactory, {
    agent,
    canisterId: Principal.fromText(canisterId),
  });
}

function parseRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}

export async function probeAnonymousAccess(getName: () => Promise<string>): Promise<AnonymousAccessResult> {
  try {
    await retryTransientQuery(getName);
    return { accessible: true };
  } catch (error) {
    if (isPublicMemoryNotFoundError(error)) {
      return { accessible: false, error: "memory not found" };
    }
    if (isAnonymousAccessError(error)) {
      return { accessible: false, error: "anonymous access denied" };
    }
    if (isTransientQueryError(error)) {
      return { accessible: false, error: TRANSIENT_QUERY_ERROR };
    }
    throw error;
  }
}

export async function retryTransientQuery<T>(call: () => Promise<T>): Promise<T> {
  try {
    return await call();
  } catch (error) {
    if (!isTransientQueryError(error)) {
      throw error;
    }
  }
  return call();
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

export function summarizeMemory(memoryId: string, metadata: DbMetadata): MemorySummaryResponse {
  const { name, description } = parseMemoryNameFields(metadata.name);
  return {
    memory_id: memoryId,
    name,
    description,
    version: metadata.version,
  };
}

async function resolvePublicMemoryState<T>(
  memoryId: string,
  checkAccess: () => Promise<AnonymousAccessResult>,
  loadMemory: () => Promise<T>,
): Promise<
  | { kind: "accessible"; memory: T }
  | { kind: "invalid"; error: "invalid memory id" }
  | { kind: "not_found"; error: "memory not found" }
  | { kind: "transient_error"; error: typeof TRANSIENT_QUERY_ERROR }
  | { kind: "denied"; error: "anonymous access denied" }
> {
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
    if (isPublicMemoryNotFoundError(error)) {
      return { kind: "not_found", error: "memory not found" };
    }
    if (isAnonymousAccessError(error)) {
      return { kind: "denied", error: "anonymous access denied" };
    }
    if (isTransientQueryError(error)) {
      return { kind: "transient_error", error: TRANSIENT_QUERY_ERROR };
    }
    throw error;
  }
}

const memoryIdlFactory: IDL.InterfaceFactory = ({ IDL: Types }) =>
  Types.Service({
    add_new_user: Types.Func([Types.Principal, Types.Nat8], [], []),
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
    get_users: Types.Func([], [Types.Vec(Types.Tuple(Types.Text, Types.Nat8))], ["query"]),
    insert: Types.Func([Types.Vec(Types.Float32), Types.Text], [Types.Nat32], []),
    search: Types.Func([Types.Vec(Types.Float32)], [Types.Vec(Types.Tuple(Types.Float32, Types.Text))], ["query"]),
  });

const launcherIdlFactory: IDL.InterfaceFactory = ({ IDL: Types }) =>
  Types.Service({
    deploy_instance: Types.Func(
      [Types.Text, Types.Nat64],
      [
        Types.Variant({
          Ok: Types.Text,
          Err: Types.Variant({
            IndexOutOfLange: Types.Null,
            SettingUpCanister: Types.Text,
            Refund: Types.Null,
            NoInstances: Types.Null,
            CreateCanister: Types.Null,
            InstallCanister: Types.Null,
            CheckBalance: Types.Text,
            AlreadyRunning: Types.Null,
          }),
        }),
      ],
      [],
    ),
    list_instance: Types.Func(
      [],
      [
        Types.Vec(
          Types.Variant({
            Empty: Types.Text,
            Pending: Types.Text,
            Creation: Types.Text,
            Installation: Types.Tuple(Types.Principal, Types.Text),
            SettingUp: Types.Principal,
            Running: Types.Principal,
          }),
        ),
      ],
      ["query"],
    ),
  });
