// Where: shared by both Cloudflare entrypoints that need IC canister access.
// What: creates anonymous or PEM-backed HttpAgent instances with one host resolution path.
// Why: keep identity handling explicit and minimize auth divergence between public and server-side flows.

import { HttpAgent, type Identity } from "@dfinity/agent";
import { Ed25519KeyIdentity } from "@dfinity/identity";
import { Secp256k1KeyIdentity } from "@dfinity/identity-secp256k1";
import { resolveIcHost, type SharedRuntimeEnv } from "./config";

export function createAnonymousAgent(env: SharedRuntimeEnv): HttpAgent {
  return new HttpAgent({ host: resolveIcHost(env) });
}

export async function createPemAgent(env: SharedRuntimeEnv): Promise<HttpAgent> {
  const identity = await identityFromPem(env.KINIC_MCP_IDENTITY_PEM ?? "");
  return new HttpAgent({ host: resolveIcHost(env), identity });
}

async function identityFromPem(pem: string): Promise<Identity> {
  const normalized = pem.trim();
  if (!normalized) {
    throw new Error("KINIC_MCP_IDENTITY_PEM is required for authenticated calls.");
  }
  if (normalized.startsWith("[")) {
    return Ed25519KeyIdentity.fromJSON(normalized);
  }
  try {
    return Secp256k1KeyIdentity.fromPem(normalized);
  } catch (error) {
    throw new Error(
      error instanceof Error ? `Unsupported identity secret format: ${error.message}` : "Unsupported identity secret format.",
    );
  }
}
