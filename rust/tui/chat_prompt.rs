//! TUI-only multi-turn prompt builder.
//! Where: used by the TUI bridge when asking AI from the memories chat panel.
//! What: combines recent visible chat history with search results into one prompt.
//! Why: keep CLI/Python ask-ai single-turn while letting the TUI preserve conversation context.

use crate::prompt_utils::escape_xml;

const MAX_HIT_LEN: usize = 600;
const MAX_FULL_LEN: usize = 4096;
const MAX_HISTORY_MESSAGES: usize = 8;
const MAX_HISTORY_MESSAGE_LEN: usize = 500;
const MAX_REWRITE_HISTORY_MESSAGES: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptHistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PromptDocument {
    pub memory_id: String,
    pub memory_name: String,
    pub score: f32,
    pub content: String,
}

pub fn build_search_rewrite_prompt(
    query: &str,
    history: &[PromptHistoryMessage],
    language: &str,
) -> String {
    let language_instruction = language_instruction(language);
    let conversation_block = render_conversation_block(history, MAX_REWRITE_HISTORY_MESSAGES);

    format!(
        r#"You rewrite a user's latest message into a standalone semantic search query.

# Instructions
- Use the latest message plus the recent conversation only to resolve references like "it", "that", or "the previous point".
- Return only one short standalone search query inside <answer>...</answer>.
- Do not answer the question.
- Do not include XML, quotes, or explanations in <answer>.
- Keep the rewritten query concise and specific.
- Use {language_instruction} if the latest user message is in that language.

<latest_user_query>
{query}
</latest_user_query>

<conversation>
{conversation}
</conversation>"#,
        conversation = if conversation_block.is_empty() {
            "<message role=\"system\">(no prior conversation)</message>".to_string()
        } else {
            conversation_block
        },
        language_instruction = language_instruction,
        query = escape_xml(query.trim()),
    )
}

pub fn build_multi_turn_chat_prompt(
    latest_user_query: &str,
    search_query: &str,
    history: &[PromptHistoryMessage],
    docs: &[PromptDocument],
    language: &str,
    failed_memory_search_count: usize,
) -> String {
    let language_instruction = language_instruction(language);
    let conversation_block = render_conversation_block(history, MAX_HISTORY_MESSAGES);

    let retrieval_status_block = if failed_memory_search_count == 0 {
        "\n".to_string()
    } else {
        format!(
            r#"<retrieval_status>
{count} parallel memory search(es) failed. <docs> only include evidence from memories that returned results; do not imply you searched the full set.
</retrieval_status>

"#,
            count = failed_memory_search_count,
        )
    };

    let docs_block = docs
        .iter()
        .enumerate()
        .map(|(index, doc)| {
            format!(
                "<doc index=\"{}\" memory_id=\"{}\" memory_name=\"{}\">\n<score>{}</score>\n<hit index=\"0\">{}</hit>\n</doc>",
                index + 1,
                escape_xml(doc.memory_id.as_str()),
                escape_xml(doc.memory_name.as_str()),
                doc.score,
                escape_xml(&clip(doc.content.as_str(), MAX_HIT_LEN))
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let docs_block = if docs_block.is_empty() {
        "<doc index=\"1\"><score>0</score><hit index=\"0\">(no hits)</hit></doc>".to_string()
    } else {
        docs_block
    };

    let full_document = escape_xml(&clip(
        &docs
            .iter()
            .map(|doc| {
                format!(
                    "[{} | {} | {}]\n{}",
                    doc.memory_name, doc.memory_id, doc.score, doc.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        MAX_FULL_LEN,
    ));

    let conversation_block = if conversation_block.is_empty() {
        "<message role=\"system\">(no prior conversation)</message>".to_string()
    } else {
        conversation_block
    };

    format!(
        r#"You are an excellent AI assistant helping a user continue a conversation about memory search results.
Answer the latest user message using the document evidence in <docs> and the recent conversation in <conversation>.

# Instructions
- Before responding, describe your reasoning in <thinking>...</thinking> using under 100 words.
- Then provide the final answer in <answer>...</answer>.
- Treat <docs> as the source of truth. Use <conversation> only to preserve continuity and resolve references.
- If <docs> do not support a claim, say so briefly instead of inventing facts.
- If <retrieval_status> reports failed memory searches, mention briefly in <answer> that some sources were unavailable.
- Keep the final answer concise and grounded in the retrieved content.
- Answer in {language_instruction} inside the <answer> tag.

# Input

<latest_user_query>
{latest_user_query}
</latest_user_query>

<search_query>
{search_query}
</search_query>
{retrieval_status}<conversation>
{conversation}
</conversation>

<docs>
{docs}
</docs>

<full_document>
{full_document}
</full_document>"#,
        conversation = conversation_block,
        docs = docs_block,
        full_document = full_document,
        latest_user_query = escape_xml(latest_user_query.trim()),
        language_instruction = language_instruction,
        retrieval_status = retrieval_status_block,
        search_query = escape_xml(search_query.trim()),
    )
}

fn clip(s: &str, max: usize) -> String {
    let clipped: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        format!("{clipped}...")
    } else {
        clipped
    }
}

fn normalized_recent_history(
    history: &[PromptHistoryMessage],
    max_messages: usize,
) -> Vec<PromptHistoryMessage> {
    let recent_history = history
        .iter()
        .filter(|message| matches!(message.role.as_str(), "user" | "assistant"))
        .filter_map(|message| {
            let content = clip(message.content.trim(), MAX_HISTORY_MESSAGE_LEN);
            (!content.is_empty()).then(|| PromptHistoryMessage {
                role: message.role.clone(),
                content,
            })
        })
        .collect::<Vec<_>>();
    let start = recent_history.len().saturating_sub(max_messages);
    recent_history.into_iter().skip(start).collect()
}

fn render_conversation_block(history: &[PromptHistoryMessage], max_messages: usize) -> String {
    normalized_recent_history(history, max_messages)
        .into_iter()
        .map(|message| {
            format!(
                "<message role=\"{}\">\n{}\n</message>",
                escape_xml(message.role.as_str()),
                escape_xml(message.content.as_str())
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn language_instruction(lang_code: &str) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::{
        PromptDocument, PromptHistoryMessage, build_multi_turn_chat_prompt,
        build_search_rewrite_prompt,
    };

    #[test]
    fn rewrite_prompt_includes_recent_history_and_latest_query() {
        let history = vec![
            PromptHistoryMessage {
                role: "user".to_string(),
                content: "What were the Q1 goals?".to_string(),
            },
            PromptHistoryMessage {
                role: "assistant".to_string(),
                content: "Revenue growth and hiring.".to_string(),
            },
        ];
        let prompt = build_search_rewrite_prompt("Who owns that?", &history, "en");

        assert!(prompt.contains("<latest_user_query>\nWho owns that?\n</latest_user_query>"));
        assert!(prompt.contains("<message role=\"user\">\nWhat were the Q1 goals?\n</message>"));
        assert!(
            prompt.contains("<message role=\"assistant\">\nRevenue growth and hiring.\n</message>")
        );
    }

    #[test]
    fn rewrite_prompt_limits_history_window_and_clips_long_messages() {
        let history = (0..8)
            .map(|index| PromptHistoryMessage {
                role: if index % 2 == 0 {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                },
                content: if index == 7 {
                    "y".repeat(700)
                } else {
                    format!("message-{index}")
                },
            })
            .collect::<Vec<_>>();

        let prompt = build_search_rewrite_prompt("what about that?", &history, "en");

        assert!(!prompt.contains("message-0"));
        assert!(!prompt.contains("message-1"));
        assert!(prompt.contains("message-2"));
        assert!(prompt.contains(&(String::from("y").repeat(500))));
        assert!(!prompt.contains(&(String::from("y").repeat(600))));
    }

    #[test]
    fn prompt_includes_recent_history_and_latest_query() {
        let history = vec![
            PromptHistoryMessage {
                role: "user".to_string(),
                content: "first".to_string(),
            },
            PromptHistoryMessage {
                role: "assistant".to_string(),
                content: "second".to_string(),
            },
        ];
        let prompt = build_multi_turn_chat_prompt(
            "latest question",
            "rewritten latest question",
            &history,
            &[PromptDocument {
                memory_id: "aaaaa-aa".to_string(),
                memory_name: "Alpha".to_string(),
                score: 0.9,
                content: "doc text".to_string(),
            }],
            "en",
            0,
        );

        assert!(prompt.contains("<latest_user_query>\nlatest question\n</latest_user_query>"));
        assert!(prompt.contains("<search_query>\nrewritten latest question\n</search_query>"));
        assert!(prompt.contains("<message role=\"user\">\nfirst\n</message>"));
        assert!(prompt.contains("<message role=\"assistant\">\nsecond\n</message>"));
        assert!(prompt.contains("memory_id=\"aaaaa-aa\""));
        assert!(prompt.contains("memory_name=\"Alpha\""));
    }

    #[test]
    fn prompt_limits_history_window_and_clips_long_messages() {
        let history = (0..10)
            .map(|index| PromptHistoryMessage {
                role: if index % 2 == 0 {
                    "user".to_string()
                } else {
                    "assistant".to_string()
                },
                content: if index == 9 {
                    "x".repeat(700)
                } else {
                    format!("message-{index}")
                },
            })
            .collect::<Vec<_>>();

        let prompt =
            build_multi_turn_chat_prompt("latest", "rewritten latest", &history, &[], "en", 0);

        assert!(!prompt.contains("message-0"));
        assert!(!prompt.contains("message-1"));
        assert!(prompt.contains("message-2"));
        assert!(prompt.contains(&(String::from("x").repeat(500))));
        assert!(!prompt.contains(&(String::from("x").repeat(600))));
    }

    #[test]
    fn prompt_includes_source_metadata_for_multiple_docs() {
        let prompt = build_multi_turn_chat_prompt(
            "latest",
            "rewritten latest",
            &[],
            &[
                PromptDocument {
                    memory_id: "aaaaa-aa".to_string(),
                    memory_name: "Alpha".to_string(),
                    score: 0.8,
                    content: "alpha doc".to_string(),
                },
                PromptDocument {
                    memory_id: "bbbbb-bb".to_string(),
                    memory_name: "Beta".to_string(),
                    score: 0.7,
                    content: "beta doc".to_string(),
                },
            ],
            "en",
            0,
        );

        assert!(prompt.contains("memory_id=\"aaaaa-aa\""));
        assert!(prompt.contains("memory_name=\"Alpha\""));
        assert!(prompt.contains("memory_id=\"bbbbb-bb\""));
        assert!(prompt.contains("memory_name=\"Beta\""));
    }

    #[test]
    fn prompt_includes_retrieval_status_when_some_memory_searches_failed() {
        let prompt = build_multi_turn_chat_prompt("q", "rq", &[], &[], "en", 2);
        assert!(prompt.contains("<retrieval_status>"));
        assert!(prompt.contains("2 parallel memory search(es) failed."));
        assert!(prompt.contains("If <retrieval_status> reports failed"));
    }

    #[test]
    fn prompt_escapes_xml_like_input() {
        let prompt = build_multi_turn_chat_prompt(
            "<latest>",
            "</search>",
            &[PromptHistoryMessage {
                role: "assistant".to_string(),
                content: "<doc>unsafe</doc>".to_string(),
            }],
            &[PromptDocument {
                memory_id: "aaaaa-aa".to_string(),
                memory_name: "\"Alpha\"".to_string(),
                score: 0.5,
                content: "<conversation>".to_string(),
            }],
            "en",
            0,
        );

        assert!(prompt.contains("&lt;latest&gt;"));
        assert!(prompt.contains("&lt;/search&gt;"));
        assert!(prompt.contains("&lt;doc&gt;unsafe&lt;/doc&gt;"));
        assert!(prompt.contains("memory_name=\"&quot;Alpha&quot;\""));
        assert!(prompt.contains("&lt;conversation&gt;"));
        assert!(!prompt.contains("<doc>unsafe</doc>"));
    }

    #[test]
    fn prompt_uses_chinese_instruction_for_ideograph_fallback() {
        let prompt = build_multi_turn_chat_prompt("总结一下", "搜索结果", &[], &[], "zh", 0);

        assert!(prompt.contains("Answer in 中文 (Chinese) inside the <answer> tag."));
    }
}
