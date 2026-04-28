use std::sync::Mutex;

use _lib::desktop::{
    self, DesktopCreateMemoryRequest, DesktopCreateMemoryResponse, DesktopInsertRequest,
    DesktopInsertResponse, DesktopMemoryDetails, DesktopMemorySummary, DesktopPreferencesUpdate,
    DesktopPreferencesView, DesktopSearchRequest, DesktopSearchResponse, DesktopSession,
    DesktopSessionRequest, DesktopSessionView,
};
use tauri::State;

#[derive(Default)]
struct AppState {
    session: Mutex<Option<DesktopSession>>,
}

#[tauri::command]
async fn get_session(
    state: State<'_, AppState>,
    request: DesktopSessionRequest,
) -> Result<DesktopSessionView, String> {
    let (session, view) = desktop::get_session(request)
        .await
        .map_err(|error| error.to_string())?;
    *state
        .session
        .lock()
        .map_err(|_| "Session lock poisoned.".to_string())? = Some(session);
    Ok(view)
}

#[tauri::command]
async fn list_memories(state: State<'_, AppState>) -> Result<Vec<DesktopMemorySummary>, String> {
    let session = require_session(&state)?;
    desktop::list_memories(&session)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn search_memories(
    state: State<'_, AppState>,
    request: DesktopSearchRequest,
) -> Result<DesktopSearchResponse, String> {
    let session = require_session(&state)?;
    desktop::search_memories(&session, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_memory_details(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<DesktopMemoryDetails, String> {
    let session = require_session(&state)?;
    desktop::get_memory_details(&session, memory_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn create_memory(
    state: State<'_, AppState>,
    request: DesktopCreateMemoryRequest,
) -> Result<DesktopCreateMemoryResponse, String> {
    let session = require_session(&state)?;
    desktop::create_memory(&session, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn insert_content(
    state: State<'_, AppState>,
    request: DesktopInsertRequest,
) -> Result<DesktopInsertResponse, String> {
    let session = require_session(&state)?;
    desktop::insert_content(&session, request)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_preferences() -> Result<DesktopPreferencesView, String> {
    desktop::get_preferences().map_err(|error| error.to_string())
}

#[tauri::command]
fn update_preferences(update: DesktopPreferencesUpdate) -> Result<DesktopPreferencesView, String> {
    desktop::update_preferences(update).map_err(|error| error.to_string())
}

fn require_session(state: &State<'_, AppState>) -> Result<DesktopSession, String> {
    state
        .session
        .lock()
        .map_err(|_| "Session lock poisoned.".to_string())?
        .clone()
        .ok_or_else(|| "Start a Kinic session first.".to_string())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_session,
            list_memories,
            search_memories,
            get_memory_details,
            create_memory,
            insert_content,
            get_preferences,
            update_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Kinic Memory desktop app");
}
