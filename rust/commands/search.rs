//! Search commands for one memory or all searchable memories.
//! Where: handles `kinic-cli search`.
//! What: embeds the query once, then searches either a single memory or all searchable memories.
//! Why: close the feature gap with the TUI while keeping the CLI search surface compact.

use std::sync::Arc;

use anyhow::{Context, Result};
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
    let rows = if args.all {
        let target_memory_ids = searchable_memory_ids(agent.clone()).await?;
        search_across_memories(agent, target_memory_ids, embedding).await?
    } else {
        let memory_id = args
            .memory_id
            .clone()
            .context("search requires --memory-id unless --all is set")?;
        search_single_memory(agent, memory_id, embedding).await?
    };

    info!(
        query = %args.query,
        result_count = rows.len(),
        searched_all = args.all,
        "search completed"
    );

    if rows.is_empty() {
        println!("No matches found for query \"{}\".", args.query);
        return Ok(());
    }

    println!("Search results for \"{}\":", args.query);
    for row in rows {
        if args.all {
            println!("- [{}] [{:.4}] {}", row.memory_id, row.score, row.payload);
        } else {
            println!("- [{:.4}] {}", row.score, row.payload);
        }
    }
    Ok(())
}

async fn searchable_memory_ids(agent: ic_agent::Agent) -> Result<Vec<String>> {
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

async fn search_across_memories(
    agent: ic_agent::Agent,
    memory_ids: Vec<String>,
    embedding: Vec<f32>,
) -> Result<Vec<SearchHit>> {
    let target_count = memory_ids.len();
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
            let result = search_single_memory(agent, memory_id.clone(), embedding).await;
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
        "join-error",
        "Search failed before any memory returned results.",
    )
    .map_err(anyhow::Error::msg)?;

    sort_search_hits(&mut batch.items);
    if !batch.failed_memory_ids.is_empty() {
        println!(
            "Note: {} memory searches failed while collecting results.",
            batch.failed_memory_ids.len()
        );
    }
    Ok(batch.items)
}

fn build_memory_client(agent: ic_agent::Agent, memory_id: &str) -> Result<MemoryClient> {
    let memory = ic_agent::export::Principal::from_text(memory_id)
        .context("Failed to parse memory canister id")?;
    Ok(MemoryClient::new(agent, memory))
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
}
