// Where: shared by public read-only Worker routes.
// What: trims and bounds user search/chat queries before expensive calls.
// Why: public endpoints must reject oversized input before embedding generation.

export const MAX_PUBLIC_QUERY_LENGTH = 150;
export const PUBLIC_QUERY_TOO_LONG_ERROR = "query is too long";
export const PUBLIC_QUERY_REQUIRED_ERROR = "query is required";

export type PublicQueryValidationResult =
  | { kind: "ready"; query: string }
  | { kind: "empty"; error: typeof PUBLIC_QUERY_REQUIRED_ERROR; status: 400 }
  | { kind: "too_long"; error: typeof PUBLIC_QUERY_TOO_LONG_ERROR; status: 413 };

export function normalizePublicQuery(value: string | null | undefined): PublicQueryValidationResult {
  const query = value?.trim() ?? "";
  if (!query) {
    return { kind: "empty", error: PUBLIC_QUERY_REQUIRED_ERROR, status: 400 };
  }
  if (Array.from(query).length > MAX_PUBLIC_QUERY_LENGTH) {
    return { kind: "too_long", error: PUBLIC_QUERY_TOO_LONG_ERROR, status: 413 };
  }
  return { kind: "ready", query };
}
