// Where: shared by public API Worker routes.
// What: centralizes CORS headers and HEAD response conversion for the Hono transport layer.
// Why: browser callers hit this Worker cross-origin from the portal shell, and OGP routes must preserve headers on HEAD.

export const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET,POST,OPTIONS",
  "Access-Control-Allow-Headers": "content-type",
};

export function withCors(response: Response): Response {
  const next = new Response(response.body, response);
  for (const [key, value] of Object.entries(CORS_HEADERS)) {
    next.headers.set(key, value);
  }
  return next;
}

export function headFrom(response: Response): Response {
  return new Response(null, {
    status: response.status,
    headers: response.headers,
  });
}
