// Where: shared by the public summary endpoint.
// What: resolves summary language selection from query params and request headers.
// Why: summary cache keys and prompts must stay stable across browsers and routes.

import { DEFAULT_PROMPT_LANGUAGE, normalizePromptLanguage } from "@kinic/kinic-share";

export const DEFAULT_SUMMARY_LANGUAGE = DEFAULT_PROMPT_LANGUAGE;

export function resolveSummaryLanguage(request: Request): string {
  const url = new URL(request.url);
  const queryLanguage = url.searchParams.get("language");
  if (queryLanguage) {
    return normalizePromptLanguage(queryLanguage);
  }
  const headerLanguage = request.headers.get("accept-language");
  if (!headerLanguage) {
    return DEFAULT_SUMMARY_LANGUAGE;
  }
  return normalizePromptLanguage(headerLanguage);
}
