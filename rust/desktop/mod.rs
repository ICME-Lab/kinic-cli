//! Desktop command boundary for the Kinic Memory Tauri app.
//! Where: called by `apps/kinic-desktop/src-tauri` only.
//! What: exposes stable DTOs and async functions for the desktop frontend.
//! Why: keep Tauri IPC separate from CLI/TUI internals while reusing Rust domain logic.

mod types;
mod views;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use kinic_core::{derive_file_tag, normalize_insert_file_path_input, tag::normalize_tag_text};

pub use types::*;
use views::{
    memory_details_view, memory_summary_view, preferences_view, search_result_view, session_view,
};

use crate::{
    embedding::fetch_embedding,
    insert_service::InsertRequest,
    preferences,
    shared::cross_memory_search::{collect_searchable_memory_ids, sort_search_hits},
    tui::bridge::{self},
};

pub async fn get_session(
    request: DesktopSessionRequest,
) -> Result<(DesktopSession, DesktopSessionView)> {
    let session = DesktopSession::from_request(request)?;
    let overview = bridge::load_session_account_overview(session.use_mainnet, session.auth()).await;
    Ok((session.clone(), session_view(&session, overview)))
}

pub async fn list_memories(session: &DesktopSession) -> Result<Vec<DesktopMemorySummary>> {
    bridge::list_memories(session.use_mainnet, session.auth())
        .await
        .map(|items| items.into_iter().map(memory_summary_view).collect())
}

pub async fn get_memory_details(
    session: &DesktopSession,
    memory_id: String,
) -> Result<DesktopMemoryDetails> {
    let details =
        bridge::load_memory_details(session.use_mainnet, session.auth(), memory_id.clone()).await?;
    Ok(memory_details_view(memory_id, details))
}

pub async fn search_memories(
    session: &DesktopSession,
    request: DesktopSearchRequest,
) -> Result<DesktopSearchResponse> {
    let query = request.query.trim();
    if query.is_empty() {
        bail!("Search query is required.");
    }

    let target_memory_ids = match request.scope {
        DesktopSearchScope::Selected => {
            vec![
                request
                    .selected_memory_id
                    .filter(|id| !id.trim().is_empty())
                    .ok_or_else(|| anyhow!("Select a memory before searching selected memory."))?,
            ]
        }
        DesktopSearchScope::All => {
            let memories = bridge::list_memories(session.use_mainnet, session.auth()).await?;
            collect_searchable_memory_ids(
                memories
                    .into_iter()
                    .map(|memory| memory.searchable_memory_id),
                "No searchable memories found.",
            )
            .map_err(anyhow::Error::msg)?
        }
    };

    let embedding = fetch_embedding(query).await?;
    let agent = bridge::build_search_agent(session.use_mainnet, session.auth()).await?;
    let mut rows = Vec::new();
    let mut first_error = None;

    for memory_id in target_memory_ids {
        match bridge::search_memory_with_agent(agent.clone(), memory_id, embedding.clone()).await {
            Ok(mut hits) => rows.append(&mut hits),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }

    if rows.is_empty()
        && let Some(error) = first_error
    {
        return Err(error);
    }

    sort_search_hits(&mut rows);
    Ok(DesktopSearchResponse {
        items: rows.into_iter().map(search_result_view).collect(),
    })
}

pub async fn create_memory(
    session: &DesktopSession,
    request: DesktopCreateMemoryRequest,
) -> Result<DesktopCreateMemoryResponse> {
    let name = request.name.trim();
    if name.is_empty() {
        bail!("Memory name is required.");
    }

    let result = bridge::create_memory(
        session.use_mainnet,
        session.auth(),
        name.to_string(),
        request.description,
    )
    .await
    .map_err(|error| anyhow!("{error:?}"))?;

    Ok(DesktopCreateMemoryResponse {
        id: result.id,
        memories: result
            .memories
            .map(|items| items.into_iter().map(memory_summary_view).collect()),
        refresh_warning: result.refresh_warning,
    })
}

pub async fn insert_content(
    session: &DesktopSession,
    request: DesktopInsertRequest,
) -> Result<DesktopInsertResponse> {
    let insert_request = build_insert_request(request)?;
    let result = bridge::run_insert(session.use_mainnet, session.auth(), insert_request)
        .await
        .map_err(|error| anyhow!("{error:?}"))?;

    Ok(DesktopInsertResponse {
        memory_id: result.memory_id,
        tag: result.tag,
        inserted_count: result.inserted_count,
        source_name: result.source_name,
    })
}

pub fn get_preferences() -> Result<DesktopPreferencesView> {
    preferences::load_user_preferences()
        .map(preferences_view)
        .map_err(anyhow::Error::msg)
}

pub fn update_preferences(update: DesktopPreferencesUpdate) -> Result<DesktopPreferencesView> {
    let mut preferences = preferences::load_user_preferences().map_err(anyhow::Error::msg)?;
    preferences.default_memory_id = update
        .default_memory_id
        .and_then(|value| normalize_optional_text(&value));
    preferences::save_user_preferences(&preferences).map_err(anyhow::Error::msg)?;
    Ok(preferences_view(preferences::normalize_user_preferences(
        preferences,
    )))
}

fn build_insert_request(request: DesktopInsertRequest) -> Result<InsertRequest> {
    match request.mode {
        DesktopInsertMode::InlineText => Ok(InsertRequest::Normal {
            memory_id: required_text(&request.memory_id, "Memory ID")?,
            tag: required_tag(request.tag.as_deref(), None)?,
            text: Some(required_text(
                request.text.as_deref().unwrap_or_default(),
                "Text",
            )?),
            file_path: None,
        }),
        DesktopInsertMode::File => build_file_insert_request(request),
        DesktopInsertMode::ManualEmbedding => {
            bail!("Manual embedding is not enabled in the desktop MVP.")
        }
    }
}

fn build_file_insert_request(request: DesktopInsertRequest) -> Result<InsertRequest> {
    let raw_file_path = required_text(
        request.file_path.as_deref().unwrap_or_default(),
        "File path",
    )?;
    let file_path = PathBuf::from(normalize_insert_file_path_input(&raw_file_path));
    let tag = required_tag(request.tag.as_deref(), Some(&file_path))?;
    let memory_id = required_text(&request.memory_id, "Memory ID")?;

    if is_pdf_path(&file_path) {
        Ok(InsertRequest::Pdf {
            memory_id,
            tag,
            file_path,
        })
    } else {
        Ok(InsertRequest::Normal {
            memory_id,
            tag,
            text: None,
            file_path: Some(file_path),
        })
    }
}

fn required_text(value: &str, label: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{label} is required.");
    }
    Ok(trimmed.to_string())
}

fn required_tag(value: Option<&str>, file_path: Option<&Path>) -> Result<String> {
    if let Some(value) = value.and_then(normalize_optional_text) {
        return normalize_tag_text(&value).map_err(|_| anyhow!("Tag is required."));
    }
    if let Some(file_path) = file_path {
        return auto_tag_from_file_path(file_path);
    }
    bail!("Tag is required.")
}

fn auto_tag_from_file_path(path: &Path) -> Result<String> {
    derive_file_tag(path).context("Could not derive tag from file path.")
}

fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

fn normalize_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests;
