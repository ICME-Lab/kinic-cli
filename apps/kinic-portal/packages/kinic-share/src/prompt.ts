// Where: shared by the public chat route and the remote MCP search/chat scaffolding.
// What: ports the existing Rust ask-ai prompt construction and answer extraction logic.
// Why: keep web answers aligned with the current CLI/TUI semantics without inventing a new prompt contract.

const MAX_QUERY_LEN = 150;
const MAX_RESULTS = 5;
const MAX_HITS_PER_DOC = 6;
const MAX_HIT_LEN = 600;
const MAX_FULL_LEN = 4096;

export type PromptSearchHit = {
  score: number;
  payload: string;
};

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
  const fullDocument = escapeXml(
    clip(
      docs
        .flatMap((doc) => doc.hits.slice(0, MAX_HITS_PER_DOC))
        .map((hit) => hit.content)
        .join("\n"),
      MAX_FULL_LEN,
    ),
  );

  return `You are an excellent AI assistant that summarizes the content of documents found as search results.
Summarize the main points concisely, taking into account their relevance to the user's search query.

# Instructions
- Before responding, please describe your thinking process within the <thinking>...</thinking> tag (keep under 100 words).
- After thinking, write your final summary within the <answer>...</answer> tag.
- The summary should be objective and grounded in the documents.
- Focus on information related to <user_query>, especially considering the content in <docs>.
- Limit the final summary to 140 words or less.
- Answer in ${promptLanguageInstruction(language)} in <answer> tag. << IMPORTANT!!

# Input

<user_query>
${escapeXml(clip(query, MAX_QUERY_LEN))}
</user_query>

<docs>
${docsBlock}
</docs>

<full_document>
${fullDocument}
</full_document>`;
}

export function extractAnswer(text: string): string {
  const normalized = normalizeChatResponse(text);
  const lower = normalized.toLowerCase();
  const start = lower.indexOf("<answer>");
  if (start === -1) {
    return normalized.trim();
  }
  const contentStart = start + "<answer>".length;
  const end = lower.indexOf("</answer>", contentStart);
  if (end === -1) {
    return normalized.trim();
  }
  return normalized.slice(contentStart, end).trim();
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

function parseRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null ? Object.fromEntries(Object.entries(value)) : null;
}
