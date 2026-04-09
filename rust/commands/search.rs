//! Search commands for one memory or all searchable memories.
//! Where: handles `kinic-cli search`.
//! What: embeds the query once, then searches either a single memory or all searchable memories.
//! Why: close the feature gap with the TUI while keeping the CLI search surface compact.

use std::sync::Arc;

use anyhow::{Context, Result};
use serde::Serialize;
use tokio::{sync::Semaphore, task::JoinSet};
use tracing::info;

use crate::{
    cli::SearchArgs,
    clients::{launcher::LauncherClient, memory::MemoryClient},
    embedding::fetch_embedding,
    shared::cross_memory_search::{
        SearchHit, collect_searchable_memory_ids, fold_search_batches,
        searchable_memory_id_from_state, sort_search_hits,
    },
};

use super::CommandContext;

const MAX_CONCURRENT_SEARCHES: usize = 10;

pub async fn handle(args: SearchArgs, ctx: &CommandContext) -> Result<()> {
    let agent = ctx.agent_factory.build().await?;
    let embedding = fetch_embedding(&args.query).await?;
    let output = if args.all {
        let target_memory_ids = searchable_memory_ids(agent.clone()).await?;
        let batch = search_across_memories(agent, target_memory_ids, embedding).await?;
        SearchOutput {
            query: args.query.clone(),
            scope: "all",
            memory_id: None,
            searched_memory_ids: batch.searched_memory_ids,
            result_count: batch.items.len(),
            failed_memory_ids: batch.failed_memory_ids,
            join_error_count: batch.join_error_count,
            items: batch.items,
        }
    } else {
        let memory_id = args
            .memory_id
            .clone()
            .context("search requires --memory-id unless --all is set")?;
        let batch = search_single_memory(agent, memory_id.clone(), embedding).await?;
        SearchOutput {
            query: args.query.clone(),
            scope: "selected",
            memory_id: Some(memory_id),
            searched_memory_ids: Vec::new(),
            result_count: batch.items.len(),
            failed_memory_ids: Vec::new(),
            join_error_count: 0,
            items: batch.items,
        }
    };

    info!(
        query = %args.query,
        result_count = output.items.len(),
        searched_all = args.all,
        "search completed"
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_search_text(&args.query, args.all, &output.items);
        if !output.failed_memory_ids.is_empty() {
            println!(
                "Note: {} memory searches failed while collecting results.",
                output.failed_memory_ids.len()
            );
        }
        if output.join_error_count > 0 {
            println!(
                "Note: {} background search task(s) failed before reporting a memory id.",
                output.join_error_count
            );
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, PartialEq)]
struct SearchOutput {
    query: String,
    scope: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    searched_memory_ids: Vec<String>,
    result_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    failed_memory_ids: Vec<String>,
    #[serde(skip_serializing_if = "is_zero")]
    join_error_count: usize,
    items: Vec<SearchHit>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct SearchBatch {
    pub searched_memory_ids: Vec<String>,
    pub failed_memory_ids: Vec<String>,
    pub join_error_count: usize,
    pub items: Vec<SearchHit>,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

pub(crate) async fn searchable_memory_ids(agent: ic_agent::Agent) -> Result<Vec<String>> {
    let states = LauncherClient::new(agent).list_memories().await?;
    collect_searchable_memory_ids(
        states.into_iter().map(searchable_memory_id_from_state),
        "no searchable memories are available",
    )
    .map_err(anyhow::Error::msg)
}

async fn search_single_memory(
    agent: ic_agent::Agent,
    memory_id: String,
    embedding: Vec<f32>,
) -> Result<SearchBatch> {
    let rows = search_single_memory_items(agent, memory_id.clone(), embedding).await?;
    Ok(SearchBatch {
        searched_memory_ids: vec![memory_id],
        failed_memory_ids: Vec::new(),
        join_error_count: 0,
        items: rows,
    })
}

pub(crate) async fn search_single_memory_items(
    agent: ic_agent::Agent,
    memory_id: String,
    embedding: Vec<f32>,
) -> Result<Vec<SearchHit>> {
    let client = build_memory_client(agent, &memory_id)?;
    let mut rows = client
        .search(embedding)
        .await
        .context("Failed to search memory canister")?
        .into_iter()
        .map(|(score, payload)| SearchHit {
            memory_id: memory_id.clone(),
            score,
            payload,
        })
        .collect::<Vec<_>>();
    sort_search_hits(&mut rows);
    Ok(rows)
}

pub(crate) async fn search_across_memories(
    agent: ic_agent::Agent,
    memory_ids: Vec<String>,
    embedding: Vec<f32>,
) -> Result<SearchBatch> {
    let target_count = memory_ids.len();
    let searched_memory_ids = memory_ids.clone();
    let concurrency = usize::min(MAX_CONCURRENT_SEARCHES, memory_ids.len().max(1));
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut tasks = JoinSet::new();

    for memory_id in memory_ids {
        let agent = agent.clone();
        let embedding = embedding.clone();
        let semaphore = semaphore.clone();
        tasks.spawn(async move {
            let permit = semaphore
                .acquire_owned()
                .await
                .expect("search semaphore should remain open");
            let result = search_single_memory_items(agent, memory_id.clone(), embedding).await;
            drop(permit);
            (memory_id, result)
        });
    }

    let mut results = Vec::new();
    let mut join_errors = Vec::new();

    while let Some(task) = tasks.join_next().await {
        match task {
            Ok((memory_id, result)) => results.push((memory_id, result)),
            Err(error) => join_errors.push(error.to_string()),
        }
    }

    let mut batch = fold_search_batches(
        target_count,
        results,
        join_errors,
        "Search failed before any memory returned results.",
    )
    .map_err(anyhow::Error::msg)?;

    sort_search_hits(&mut batch.items);
    Ok(SearchBatch {
        searched_memory_ids,
        failed_memory_ids: batch.failed_memory_ids,
        join_error_count: batch.join_error_count,
        items: batch.items,
    })
}

fn build_memory_client(agent: ic_agent::Agent, memory_id: &str) -> Result<MemoryClient> {
    let memory = ic_agent::export::Principal::from_text(memory_id)
        .context("Failed to parse memory canister id")?;
    Ok(MemoryClient::new(agent, memory))
}

fn print_search_text(query: &str, searched_all: bool, rows: &[SearchHit]) {
    if rows.is_empty() {
        println!("No matches found for query \"{}\".", query);
        return;
    }

    println!("Search results for \"{}\":", query);
    for row in rows {
        let display = SearchTextRow::from_payload(&row.payload);
        if searched_all {
            println!(
                "- [{}] [{:.4}] {}",
                row.memory_id, row.score, display.summary
            );
        } else {
            println!("- [{:.4}] {}", row.score, display.summary);
        }
        if let Some(tag) = display.tag {
            println!("  tag={tag}");
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SearchTextRow {
    summary: String,
    tag: Option<String>,
}

impl SearchTextRow {
    fn from_payload(payload: &str) -> Self {
        let trimmed = payload.trim();
        let Some(value) = serde_json::from_str::<serde_json::Value>(trimmed).ok() else {
            return Self {
                summary: trimmed.to_string(),
                tag: None,
            };
        };
        let Some(object) = value.as_object() else {
            return Self {
                summary: trimmed.to_string(),
                tag: None,
            };
        };

        let sentence = object
            .get("sentence")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let tag = object
            .get("tag")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        Self {
            summary: sentence.unwrap_or_else(|| trimmed.to_string()),
            tag,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_rows_orders_by_score_desc_then_memory_id() {
        let mut rows = vec![
            SearchHit {
                memory_id: "bbbbb-bb".to_string(),
                score: 0.2,
                payload: "beta".to_string(),
            },
            SearchHit {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.9,
                payload: "alpha".to_string(),
            },
            SearchHit {
                memory_id: "ccccc-cc".to_string(),
                score: 0.9,
                payload: "gamma".to_string(),
            },
        ];

        sort_search_hits(&mut rows);

        assert_eq!(rows[0].memory_id, "aaaaa-aa");
        assert_eq!(rows[1].memory_id, "ccccc-cc");
        assert_eq!(rows[2].memory_id, "bbbbb-bb");
    }

    #[test]
    fn search_output_serializes_empty_results_for_json_consumers() {
        let output = serde_json::to_value(SearchOutput {
            query: "hello".to_string(),
            scope: "selected",
            memory_id: Some("aaaaa-aa".to_string()),
            searched_memory_ids: Vec::new(),
            result_count: 0,
            failed_memory_ids: Vec::new(),
            join_error_count: 0,
            items: Vec::new(),
        })
        .expect("search output should serialize");

        assert_eq!(output["query"], serde_json::json!("hello"));
        assert_eq!(output["scope"], serde_json::json!("selected"));
        assert_eq!(output["result_count"], serde_json::json!(0));
        assert_eq!(output["items"], serde_json::json!([]));
        assert!(output.get("join_error_count").is_none());
    }

    #[test]
    fn search_output_serializes_join_error_count_separately_from_failed_memory_ids() {
        let output = serde_json::to_value(SearchOutput {
            query: "hello".to_string(),
            scope: "all",
            memory_id: None,
            searched_memory_ids: vec!["aaaaa-aa".to_string(), "bbbbb-bb".to_string()],
            result_count: 1,
            failed_memory_ids: vec!["bbbbb-bb".to_string()],
            join_error_count: 2,
            items: vec![SearchHit {
                memory_id: "aaaaa-aa".to_string(),
                score: 0.8,
                payload: "alpha".to_string(),
            }],
        })
        .expect("search output should serialize");

        assert_eq!(output["failed_memory_ids"], serde_json::json!(["bbbbb-bb"]));
        assert_eq!(output["join_error_count"], serde_json::json!(2));
    }

    #[test]
    fn search_text_row_prefers_sentence_and_tag_from_json_payload() {
        let row =
            SearchTextRow::from_payload(r##"{"sentence":"# Run tests with output","tag":"docs"}"##);

        assert_eq!(row.summary, "# Run tests with output");
        assert_eq!(row.tag.as_deref(), Some("docs"));
    }

    #[test]
    fn search_text_row_falls_back_to_raw_payload() {
        let row = SearchTextRow::from_payload("plain text payload");

        assert_eq!(row.summary, "plain text payload");
        assert_eq!(row.tag, None);
    }
}
