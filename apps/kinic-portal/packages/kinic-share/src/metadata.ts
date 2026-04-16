// Where: shared TypeScript package used by public APIs and UI components.
// What: parses the Kinic memory metadata envelope embedded in the canister name field.
// Why: preserve the existing Rust contract so web responses match CLI/TUI behavior.

export type ParsedMemoryMetadata = {
  name?: string;
  description?: string;
};

export function parseMemoryNameFields(raw: string): {
  name: string;
  description: string | null;
} {
  const trimmed = raw.trim();
  const parsed = parseMemoryMetadata(trimmed);
  if (!parsed) {
    return { name: trimmed, description: null };
  }
  return {
    name: parsed.name ?? trimmed,
    description: parsed.description ?? null,
  };
}

function parseMemoryMetadata(raw: string): ParsedMemoryMetadata | null {
  try {
    const value = parseObject(JSON.parse(raw));
    if (!value) {
      return null;
    }
    const name = normalizeField(value.name);
    const description = normalizeField(value.description);
    if (!name && !description) {
      return null;
    }
    return { name: name ?? undefined, description: description ?? undefined };
  } catch {
    return null;
  }
}

function parseObject(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}

function normalizeField(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}
