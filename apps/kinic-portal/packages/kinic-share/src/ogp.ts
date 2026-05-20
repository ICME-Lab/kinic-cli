// Where: shared TypeScript helpers used by the public portal metadata and OGP image routes.
// What: normalizes social-card copy and compact stats from one memory payload.
// Why: keep SSR metadata text and generated image content aligned without duplicating truncation rules.

export const DEFAULT_MEMORY_METADATA_DESCRIPTION = "Explore a public memory shared on Kinic.";
export const DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION = "Public notes and context shared on Kinic";
export const DEFAULT_MEMORY_OGP_IMAGE_TITLE = "Public Memory";

const TITLE_LIMIT = 72;
const DESCRIPTION_LIMIT = 160;
const MEMORY_ID_HEAD = 6;
const MEMORY_ID_TAIL = 4;
const SEARCH_QUERY_LIMIT = 120;

export type MemoryOgpInput = {
  memoryId?: string | null;
  name?: string | null;
  description?: string | null;
  owner?: string | null;
};

export type MemoryOgpCardModel = {
  title: string;
  description: string;
  shortMemoryId: string;
  owner: string | null;
};

export type MemoryOgpImageCopy = {
  title: string;
  description: string;
};

export function buildMemoryPageTitle(name?: string | null, memoryId?: string | null): string {
  const normalizedName = normalizeOptionalCopy(name);
  const normalizedMemoryId = normalizeOptionalCopy(memoryId);
  const title = normalizedName
    ? clamp(normalizedMemoryId ? `${normalizedName} · ${normalizedMemoryId}` : normalizedName, TITLE_LIMIT)
    : normalizeCopy(normalizedMemoryId, "Untitled Memory");
  return `${title} | Kinic`;
}

export function buildMemoryMetadataDescription(description?: string | null): string {
  return clamp(normalizeCopy(description, DEFAULT_MEMORY_METADATA_DESCRIPTION), DESCRIPTION_LIMIT);
}

export function buildMemoryOgpCardModel(input: MemoryOgpInput): MemoryOgpCardModel {
  const copy = buildMemoryOgpImageCopy(input);
  return {
    title: copy.title,
    description: copy.description,
    shortMemoryId: normalizeMemoryId(input.memoryId),
    owner: normalizeOptionalCopy(input.owner) || null,
  };
}

export function buildMemoryOgpImageCopy(input: MemoryOgpInput): MemoryOgpImageCopy {
  return {
    title: clamp(normalizeCopy(input.name, DEFAULT_MEMORY_OGP_IMAGE_TITLE), TITLE_LIMIT),
    description: clamp(normalizeCopy(input.description, DEFAULT_MEMORY_OGP_IMAGE_DESCRIPTION), DESCRIPTION_LIMIT),
  };
}

export function shortenMemoryId(memoryId?: string | null): string {
  const normalized = normalizeMemoryId(memoryId);
  if (normalized.length <= MEMORY_ID_HEAD + MEMORY_ID_TAIL + 1) {
    return normalized;
  }
  return `${normalized.slice(0, MEMORY_ID_HEAD)}...${normalized.slice(-MEMORY_ID_TAIL)}`;
}

function normalizeMemoryId(memoryId?: string | null): string {
  return normalizeCopy(memoryId, "-");
}

export function buildMemorySearchQuery(input: MemoryOgpInput): string {
  const parts = [normalizeOptionalCopy(input.name), normalizeOptionalCopy(input.description)].filter(Boolean);
  return clamp(parts.join(" ").trim() || "public memory", SEARCH_QUERY_LIMIT);
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
