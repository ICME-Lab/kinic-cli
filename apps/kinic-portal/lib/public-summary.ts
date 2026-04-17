// Where: portal-side helpers for public summary requests.
// What: normalizes summary language selection from query params and request headers.
// Why: summary cache keys and prompts must stay stable across browsers and routes.

export const DEFAULT_SUMMARY_LANGUAGE = "en";

export function resolveSummaryLanguage(request: Request): string {
  const url = new URL(request.url);
  const queryLanguage = url.searchParams.get("language");
  if (queryLanguage) {
    return normalizeSummaryLanguage(queryLanguage);
  }

  const headerLanguage = request.headers.get("accept-language");
  if (!headerLanguage) {
    return DEFAULT_SUMMARY_LANGUAGE;
  }

  return normalizeSummaryLanguage(headerLanguage);
}

export function normalizeSummaryLanguage(value: string | null | undefined): string {
  const candidate = value
    ?.split(",", 1)[0]
    ?.split(";", 1)[0]
    ?.trim()
    ?.toLowerCase()
    ?.replaceAll("_", "-")
    ?.replace(/[^a-z0-9-]/g, "");

  return candidate?.split("-", 1)[0] || DEFAULT_SUMMARY_LANGUAGE;
}
