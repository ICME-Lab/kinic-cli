// Where: shared by the public summary endpoint.
// What: normalizes summary language selection from query params and request headers.
// Why: summary cache keys and prompts must stay stable across browsers and routes.

export const DEFAULT_SUMMARY_LANGUAGE = "en";
const SUPPORTED_SUMMARY_LANGUAGES = new Set(["en", "ja", "ko", "zh", "es", "fr", "de", "it", "pt", "ru"]);
const MAX_SUMMARY_LANGUAGE_LENGTH = 16;

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

function normalizeSummaryLanguage(value: string | null | undefined): string {
  const candidate = value
    ?.split(",", 1)[0]
    ?.split(";", 1)[0]
    ?.trim()
    ?.toLowerCase()
    ?.replaceAll("_", "-")
    ?.replace(/[^a-z0-9-]/g, "");

  const primary = candidate?.split("-", 1)[0] || DEFAULT_SUMMARY_LANGUAGE;
  if (primary.length > MAX_SUMMARY_LANGUAGE_LENGTH) {
    return DEFAULT_SUMMARY_LANGUAGE;
  }
  return SUPPORTED_SUMMARY_LANGUAGES.has(primary) ? primary : DEFAULT_SUMMARY_LANGUAGE;
}
