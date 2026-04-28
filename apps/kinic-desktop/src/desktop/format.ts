// Where: UI display helpers for desktop DTOs.
// What: formats optional technical values for dense app panes.
// Why: keep presentation logic out of the Zustand store.

export function valueOrUnavailable(value: string | number | null | undefined) {
  if (value === null || value === undefined || value === "") {
    return "Unavailable";
  }
  return String(value);
}

export function shortPrincipal(value: string | null | undefined) {
  if (!value || value.length <= 18) {
    return valueOrUnavailable(value);
  }
  return `${value.slice(0, 10)}...${value.slice(-6)}`;
}

export function statusText(error: string | null, loading: boolean, fallback: string) {
  if (loading) {
    return "Loading...";
  }
  if (error) {
    return error;
  }
  return fallback;
}
