// Where: shared by portal metadata and client-side fetches.
// What: resolves the dedicated public API Worker origin and builds absolute URLs.
// Why: portal pages must stay light while read-only APIs move to a separate Worker.

const DEFAULT_PUBLIC_API_ORIGIN = "https://kinic-portal-public-api.kasane.workers.dev";

export function resolvePublicApiOrigin(): string {
  const raw = process.env.KINIC_PUBLIC_API_ORIGIN?.trim();
  if (raw) {
    return raw.replace(/\/+$/, "");
  }
  return DEFAULT_PUBLIC_API_ORIGIN;
}

export function buildPublicApiUrl(path: string, origin = resolvePublicApiOrigin()): string {
  return new URL(path, `${origin}/`).toString();
}
