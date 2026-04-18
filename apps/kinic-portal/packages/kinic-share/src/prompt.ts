// Where: shared by the public chat route and the remote MCP search/chat scaffolding.
// What: ports the existing Rust ask-ai prompt construction and answer extraction logic.
// Why: keep web answers aligned with the current CLI/TUI semantics without inventing a new prompt contract.

const MAX_QUERY_LEN = 150;
const MAX_RESULTS = 5;
const MAX_HITS_PER_DOC = 6;
const MAX_HIT_LEN = 600;
const SUMMARY_QUERY = "overview summary main topics purpose contents";

export type PromptSearchHit = {
  score: number;
  payload: string;
};

export class PromptContractError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PromptContractError";
  }
}

export function buildAskAiPrompt(
  query: string,
  rawResults: PromptSearchHit[],
  language: string,
): string {
  const docs = rawResults.slice(0, MAX_RESULTS).map((result, index) => ({
    url: `memory://${index + 1}`,
    title: clip(result.payload, 80),
    score: result.score,
    hits: [{ index: 0, score: result.score, content: result.payload }],
  }));
  const docsBlock = docs.length
    ? docs
        .map((doc, index) => {
          const hits = doc.hits
            .slice(0, MAX_HITS_PER_DOC)
            .map(
              (hit) =>
                `<hit index="${hit.index}" score="${hit.score}">\n${escapeXml(
                  clip(hit.content, MAX_HIT_LEN),
                )}\n</hit>`,
            )
            .join("\n");
          return `<doc index="${index + 1}">\n<url>${escapeXml(doc.url)}</url>\n<title>${escapeXml(
            doc.title,
          )}</title>\n<score>${doc.score}</score>\n<hits>\n${hits || '<hit index="0">(no hits)</hit>'}\n</hits>\n</doc>`;
        })
        .join("\n\n")
    : '<doc index="1"><url></url><title></title><hits><hit index="0">(no hits)</hit></hits></doc>';
  return `You are an excellent AI assistant that summarizes the content of documents found as search results.
Summarize the main points concisely, taking into account their relevance to the user's search query.

# Instructions
- The summary should be objective and grounded in the documents.
- Focus on information related to <user_query>, especially considering the content in <docs>.
- Limit the final summary to 140 words or less.
- Return only the final summary text with no XML tags, labels, or prefatory text.
- Answer in ${promptLanguageInstruction(language)}. << IMPORTANT!!

# Input

<user_query>
${escapeXml(clip(query, MAX_QUERY_LEN))}
</user_query>

<docs>
${docsBlock}
</docs>`;
}

export function buildMemorySummarySearchQuery(name: string, description: string | null): string {
  return [name.trim(), description?.trim(), SUMMARY_QUERY].filter(Boolean).join("\n");
}

export function buildMemorySummaryPrompt(
  name: string,
  description: string | null,
  rawResults: PromptSearchHit[],
  language: string,
): string {
  const docs = rawResults.slice(0, MAX_RESULTS).map((result, index) => ({
    url: `memory://${index + 1}`,
    title: clip(result.payload, 80),
    score: result.score,
    hits: [{ index: 0, score: result.score, content: result.payload }],
  }));
  const docsBlock = docs.length
    ? docs
        .map((doc, index) => {
          const hits = doc.hits
            .slice(0, MAX_HITS_PER_DOC)
            .map(
              (hit) =>
                `<hit index="${hit.index}" score="${hit.score}">\n${escapeXml(
                  clip(hit.content, MAX_HIT_LEN),
                )}\n</hit>`,
            )
            .join("\n");
          return `<doc index="${index + 1}">\n<url>${escapeXml(doc.url)}</url>\n<title>${escapeXml(
            doc.title,
          )}</title>\n<score>${doc.score}</score>\n<hits>\n${hits || '<hit index="0">(no hits)</hit>'}\n</hits>\n</doc>`;
        })
        .join("\n\n")
    : '<doc index="1"><url></url><title></title><hits><hit index="0">(no hits)</hit></hits></doc>';

  return `You are an excellent AI assistant that writes a short public-facing summary for one memory.
Summarize only what is supported by the memory metadata and retrieved content.

# Instructions
- The final summary must be one short paragraph of 2-3 sentences.
- Do not use bullet points.
- Do not exaggerate, speculate, or add facts that are not present in the memory.
- Mention the memory's purpose, main topics, and what kind of content it contains when supported by the evidence.
- Return only the final summary text with no XML tags, labels, or prefatory text.
- Answer in ${promptLanguageInstruction(language)}.

# Input

<memory_name>
${escapeXml(clip(name, 120))}
</memory_name>

<memory_description>
${escapeXml(clip(description?.trim() || "(none)", 240))}
</memory_description>

<docs>
${docsBlock}
</docs>`;
}

export function extractAnswer(text: string): string {
  const normalized = normalizeChatResponse(text);
  const lower = normalized.toLowerCase();
  const start = lower.indexOf("<answer>");
  const extracted = start === -1 ? normalized : extractTaggedAnswer(normalized, lower, start);
  const answer = extracted.trim();
  if (!answer) {
    throw new PromptContractError("empty model output");
  }
  return answer;
}

export function escapeXml(input: string): string {
  return input
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function clip(value: string, max: number): string {
  const chars = Array.from(value);
  return chars.length > max ? `${chars.slice(0, max).join("")}...` : value;
}

function promptLanguageInstruction(language: string): string {
  switch (language.split(/[-_]/, 1)[0]?.toLowerCase()) {
    case "ja":
      return "Japanese";
    case "ko":
      return "Korean";
    case "zh":
      return "Chinese";
    case "es":
      return "Spanish";
    case "fr":
      return "French";
    case "de":
      return "German";
    case "it":
      return "Italian";
    case "pt":
      return "Portuguese";
    case "ru":
      return "Russian";
    default:
      return "English";
  }
}

function normalizeChatResponse(text: string): string {
  if (!text.includes("data:")) {
    return text;
  }

  const chunks = text
    .split("\n")
    .flatMap((line) => {
      const trimmed = line.trim();
      if (!trimmed.startsWith("data:")) {
        return [];
      }
      const payload = trimmed.slice("data:".length).trim();
      try {
        const record = parseRecord(JSON.parse(payload));
        return typeof record?.content === "string" ? [record.content] : [];
      } catch {
        return [];
      }
    });

  return chunks.length > 0 ? chunks.join("") : text;
}

function extractTaggedAnswer(normalized: string, lower: string, start: number): string {
  const contentStart = start + "<answer>".length;
  const end = lower.indexOf("</answer>", contentStart);
  return end === -1 ? normalized : normalized.slice(contentStart, end);
}

function parseRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}
