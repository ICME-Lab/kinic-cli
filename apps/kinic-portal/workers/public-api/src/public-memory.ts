// Where: shared by public API Worker handlers.
// What: adapts Worker env bindings to the repo-local public-memory runtime helper.
// Why: detail, summary, chat, and OGP share the same anonymous resolution path here.

export {
  resolvePublicMemory,
  resolvePublicMemorySummaryOnly,
  type PublicMemoryState,
  type PublicMemorySummaryState,
  toPublicMemoryRuntimeEnv as toSharedRuntimeEnv,
} from "../../shared/public-memory-runtime";
