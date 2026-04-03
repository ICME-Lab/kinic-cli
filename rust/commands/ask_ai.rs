use std::cmp::Ordering;

use anyhow::{Context, Result};
use ic_agent::export::Principal;
use reqwest::Client;
use tracing::info;

use crate::{
    agent::AgentFactory,
    cli::AskAiArgs,
    clients::memory::MemoryClient,
    embedding::{embedding_base_url, fetch_embedding},
};

use super::CommandContext;

const MAX_QUERY_LEN: usize = 150;
const MAX_RESULTS: usize = 5;
const MAX_HITS_PER_DOC: usize = 6;
const MAX_HIT_LEN: usize = 600;
const MAX_FULL_LEN: usize = 4096;
const CHAT_PATH: &str = "/chat";

pub struct AskAiResult {
    pub prompt: String,
    pub response: String,
    pub context_count: usize,
    pub top_k_used: usize,
}

pub async fn handle(args: AskAiArgs, ctx: &CommandContext) -> Result<()> {
    let memory = Principal::from_text(&args.memory_id)
        .context("Failed to parse canister id for ask-ai command")?;
    let result = ask_ai_flow(&ctx.agent_factory, &memory, &args.query, args.top_k, "en").await?;

    info!(
        canister_id = %memory,
        query = %args.query,
        result_count = result.context_count,
        "ask-ai search completed"
    );

    println!("ask-ai for \"{}\":", args.query);
    if result.context_count == 0 {
        println!("- No context found to answer the query.");
    } else {
        println!(
            "- Generated prompt for LLM (showing top {}).",
            result.top_k_used
        );
        println!("{}", result.prompt);
    }
    println!("\nLLM response:\n{}", result.response);

    Ok(())
}

pub async fn ask_ai_flow(
    agent_factory: &AgentFactory,
    memory_id: &Principal,
    query: &str,
    top_k: usize,
    language: &str,
) -> Result<AskAiResult> {
    let agent = agent_factory.build().await?;
    let client = MemoryClient::new(agent, *memory_id);

    let embedding = fetch_embedding(query).await?;
    let mut results = client.search(embedding).await?;

    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    let limit = top_k.max(1);
    let prompt = build_prompt(query, &results, limit, language);
    let llm_response = call_chat_endpoint(&prompt).await?;

    Ok(AskAiResult {
        prompt,
        response: llm_response,
        context_count: results.len(),
        top_k_used: limit.min(results.len()),
    })
}

pub(crate) async fn call_chat_endpoint(prompt: &str) -> Result<String> {
    let url = format!("{}{}", embedding_base_url(), CHAT_PATH);
    let response = Client::new()
        .post(url)
        .json(&ChatRequest { message: prompt })
        .send()
        .await
        .context("Failed to call chat endpoint")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("chat endpoint returned {status}: {body}");
    }

    let body = response
        .text()
        .await
        .context("Failed to read chat response")?;

    let mut acc = String::new();
    for line in body.lines() {
        if let Some(stripped) = line.strip_prefix("data:") {
            let payload = stripped.trim();
            if payload.is_empty() {
                continue;
            }
            if let Ok(ChatChunk {
                content: Some(content),
            }) = serde_json::from_str::<ChatChunk>(payload)
            {
                acc.push_str(&content);
            }
        }
    }

    if acc.is_empty() {
        acc = body;
    }

    Ok(extract_answer(&acc))
}

#[derive(Clone, Debug)]
struct SearchHit {
    index: usize,
    score: f32,
    content: String,
}

#[derive(Clone, Debug)]
struct SearchResult {
    url: String,
    title: String,
    score: f32,
    hits: Vec<SearchHit>,
}

#[derive(serde::Serialize)]
struct ChatRequest<'a> {
    message: &'a str,
}

#[derive(serde::Deserialize)]
struct ChatChunk {
    content: Option<String>,
}

fn extract_answer(text: &str) -> String {
    let lower = text.to_lowercase();
    let start_tag = "<answer>";
    let end_tag = "</answer>";

    if let (Some(start), Some(end)) = (
        lower.find(start_tag),
        lower.find(end_tag).map(|i| i + end_tag.len()),
    ) {
        let content_start = start + start_tag.len();
        let content_end = end - end_tag.len();
        let snippet = &text[content_start..content_end];
        snippet.trim().to_string()
    } else {
        text.trim().to_string()
    }
}

fn build_prompt(
    query: &str,
    raw_results: &[(f32, String)],
    top_k: usize,
    language: &str,
) -> String {
    let clipped_query = clip(query, MAX_QUERY_LEN);

    let docs: Vec<SearchResult> = raw_results
        .iter()
        .take(top_k.min(MAX_RESULTS))
        .enumerate()
        .map(|(i, (score, text))| SearchResult {
            url: format!("memory://{}", i + 1),
            title: clip(text, 80),
            score: *score,
            hits: vec![SearchHit {
                index: 0,
                score: *score,
                content: text.clone(),
            }],
        })
        .collect();

    ask_ai_prompt(&clipped_query, &docs, language)
}

fn clip(s: &str, max: usize) -> String {
    let clipped: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        format!("{clipped}...")
    } else {
        clipped
    }
}

fn strip_tags(s: &str) -> String {
    s.replace("<thinking>", "")
        .replace("</thinking>", "")
        .replace("<answer>", "")
        .replace("</answer>", "")
        .replace("<THINKING>", "")
        .replace("</THINKING>", "")
        .replace("<ANSWER>", "")
        .replace("</ANSWER>", "")
}

fn get_language_instruction(lang_code: &str) -> &'static str {
    match lang_code {
        "ja" => "日本語 (Japanese)",
        "ko" => "한국어 (Korean)",
        "zh" => "中文 (Chinese)",
        "es" => "Español (Spanish)",
        "fr" => "Français (French)",
        "de" => "Deutsch (German)",
        "it" => "Italiano (Italian)",
        "pt" => "Português (Portuguese)",
        "ru" => "Русский (Russian)",
        _ => "English",
    }
}

fn ask_ai_prompt(query: &str, results: &[SearchResult], language: &str) -> String {
    let language_instruction = get_language_instruction(language);

    let top_results = results.iter().take(MAX_RESULTS).collect::<Vec<_>>();

    let formatted_docs = top_results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let hits_xml = r
                .hits
                .iter()
                .take(MAX_HITS_PER_DOC)
                .map(|h| {
                    format!(
                        "<hit index=\"{}\" score=\"{}\">\n{}\n</hit>",
                        h.index,
                        h.score,
                        strip_tags(&clip(&h.content, MAX_HIT_LEN))
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                "<doc index=\"{index}\">\n<url>{url}</url>\n<title>{title}</title>\n<score>{score}</score>\n<hits>\n{hits}\n</hits>\n</doc>",
                index = i + 1,
                url = r.url,
                title = strip_tags(&r.title),
                score = r.score,
                hits = if hits_xml.is_empty() { r#"<hit index="0">(no hits)</hit>"#.to_string() } else { hits_xml },
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let docs_block = if formatted_docs.is_empty() {
        r#"<doc index="1"><url></url><title></title><hits><hit index="0">(no hits)</hit></hits></doc>"#
            .to_string()
    } else {
        formatted_docs
    };

    let full_document = strip_tags(&clip(
        &top_results
            .iter()
            .flat_map(|r| r.hits.iter().take(MAX_HITS_PER_DOC))
            .map(|h| h.content.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        MAX_FULL_LEN,
    ));

    format!(
        r#"You are an excellent AI assistant that summarizes the content of documents found as search results.
Summarize the main points concisely, taking into account their relevance to the user's search query.

# Instructions
- Before responding, please describe your thinking process within the <thinking>...</thinking> tag (keep under 100 words).
- After thinking, write your final summary within the <answer>...</answer> tag.
- The summary should be objective and grounded in the documents.
- Focus on information related to <user_query>, especially considering the content in <docs>.
- Limit the final summary to 140 words or less.
- Answer in {language_instruction} in <answer> tag. << IMPORTANT!!

# Input

<user_query>
{query}
</user_query>

<docs>
{docs}
</docs>

<full_document>
{full_document}
</full_document>"#,
        docs = docs_block,
        full_document = full_document,
        language_instruction = language_instruction,
        query = query,
    )
}
