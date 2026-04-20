// Where: shared by portal routes and the remote MCP Worker.
// What: re-exports local-only public-memory runtime helpers and shared error copy.
// Why: keep repo-internal callers off the package root while avoiding duplicated transient-error text.

export {
  classifyPublicMemoryRuntimeError,
  TRANSIENT_QUERY_ERROR as TRANSIENT_PUBLIC_MEMORY_ERROR,
} from "@kinic/kinic-share/memory-internal";
