// Where: shared TypeScript helpers used by the public portal metadata and OGP image routes.
// What: normalizes social-card copy and compact stats from one memory payload.
// Why: keep SSR metadata text and generated image content aligned without duplicating truncation rules.

export const DEFAULT_MEMORY_METADATA_DESCRIPTION = "Public read-only memory";
export const DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION = "Public read-only memory";
export const DEFAULT_MEMORY_OGP_IMAGE_TITLE = "Shared Memory";
export const DEFAULT_MEMORY_OGP_SEARCH_EXCERPT = "Search result preview unavailable";

const TITLE_LIMIT = 72;
const DESCRIPTION_LIMIT = 160;
const MEMORY_ID_HEAD = 6;
const MEMORY_ID_TAIL = 4;
const SEARCH_QUERY_LIMIT = 120;
const SEARCH_CARD_LIMIT = 3;
const SEARCH_CARD_TAG_LIMIT = 20;
const SEARCH_CARD_EXCERPT_LIMIT = 110;

export type MemoryOgpInput = {
  memoryId?: string | null;
  name?: string | null;
  description?: string | null;
};

export type MemoryOgpCardModel = {
  title: string;
  description: string;
  shortMemoryId: string;
};

export type MemoryOgpSearchCard = {
  tag: string;
  excerpt: string;
};

export function buildMemoryPageTitle(name?: string | null): string {
  const normalized = normalizeCopy(name, "Untitled Memory");
  return `${clamp(normalized, TITLE_LIMIT)} | Kinic`;
}

export function buildMemoryMetadataDescription(description?: string | null): string {
  return clamp(normalizeCopy(description, DEFAULT_MEMORY_METADATA_DESCRIPTION), DESCRIPTION_LIMIT);
}

export function buildMemoryOgpCardModel(input: MemoryOgpInput): MemoryOgpCardModel {
  return {
    title: clamp(normalizeAsciiCopy(input.name, DEFAULT_MEMORY_OGP_IMAGE_TITLE), TITLE_LIMIT),
    description: clamp(normalizeAsciiCopy(input.description, DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION), DESCRIPTION_LIMIT),
    shortMemoryId: shortenMemoryId(input.memoryId),
  };
}

export function shortenMemoryId(memoryId?: string | null): string {
  const normalized = normalizeCopy(memoryId, "-");
  if (normalized.length <= MEMORY_ID_HEAD + MEMORY_ID_TAIL + 1) {
    return normalized;
  }
  return `${normalized.slice(0, MEMORY_ID_HEAD)}...${normalized.slice(-MEMORY_ID_TAIL)}`;
}

export function buildMemorySearchQuery(input: MemoryOgpInput): string {
  const parts = [normalizeOptionalCopy(input.name), normalizeOptionalCopy(input.description)].filter(Boolean);
  return clamp(parts.join(" ").trim() || "public memory", SEARCH_QUERY_LIMIT);
}

export function buildMemoryOgpSearchCards(
  hits: Array<{ payload: string }>,
): MemoryOgpSearchCard[] {
  return hits
    .map((hit) => parseSearchCard(hit.payload))
    .filter((card): card is MemoryOgpSearchCard => card !== null)
    .slice(0, SEARCH_CARD_LIMIT);
}

function normalizeCopy(value: string | null | undefined, fallback: string): string {
  const normalized = normalizeOptionalCopy(value);
  return normalized || fallback;
}

function normalizeOptionalCopy(value: string | null | undefined): string {
  if (typeof value !== "string") {
    return "";
  }
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized;
}

function clamp(value: string, limit: number): string {
  if (value.length <= limit) {
    return value;
  }
  return `${value.slice(0, Math.max(0, limit - 1)).trimEnd()}…`;
}

function parseSearchCard(payload: string): MemoryOgpSearchCard | null {
  const parsed = parseRecord(payload);
  const excerpt = clamp(
    normalizeCopy(typeof parsed?.payload === "string" ? parsed.payload : payload, ""),
    SEARCH_CARD_EXCERPT_LIMIT,
  );
  if (!excerpt) {
    return null;
  }
  return {
    tag: clamp(normalizeAsciiCopy(typeof parsed?.tag === "string" ? parsed.tag : "Search Hit", "Search Hit"), SEARCH_CARD_TAG_LIMIT),
    excerpt: clamp(normalizeAsciiCopy(excerpt, DEFAULT_MEMORY_OGP_SEARCH_EXCERPT), SEARCH_CARD_EXCERPT_LIMIT),
  };
}

function normalizeAsciiCopy(value: string | null | undefined, fallback: string): string {
  const normalized = normalizeOptionalCopy(value)
    .normalize("NFKC")
    .replace(/[^\x20-\x7E]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return normalized || fallback;
}

function parseRecord(value: string): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(value);
    return typeof parsed === "object" && parsed !== null ? Object.fromEntries(Object.entries(parsed)) : null;
  } catch {
    return null;
  }
}
