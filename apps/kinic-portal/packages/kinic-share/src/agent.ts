// Where: shared by the public portal routes and the remote MCP Worker.
// What: creates the anonymous HttpAgent used by the read-only public surface.
// Why: keep IC host resolution in one place without carrying unused auth paths.

import { HttpAgent } from "@dfinity/agent";
import { resolveIcHost, type SharedRuntimeEnv } from "./config";

export function createAnonymousAgent(env: SharedRuntimeEnv): HttpAgent {
  return new HttpAgent({ host: resolveIcHost(env) });
}
