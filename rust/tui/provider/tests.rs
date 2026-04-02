use super::*;
use crate::create_domain::derive_create_cost;
use candid::Nat;
use std::{
    env, fs,
    time::{SystemTime, UNIX_EPOCH},
};
use tui_kit_runtime::{
    CreateCostState, LoadedCreateCost, PickerConfirmKind, PickerContext, PickerItem,
    PickerListMode, PickerState, SessionAccountOverview,
};

fn session_snapshot(principal_id: &str) -> tui_kit_runtime::SessionSettingsSnapshot {
    crate::tui::settings::session_settings_snapshot(
        &TuiAuth::resolved_for_tests(),
        false,
        Some(principal_id.to_string()),
        "https://api.kinic.io".to_string(),
    )
}

fn live_config() -> TuiConfig {
    TuiConfig {
        auth: TuiAuth::resolved_for_tests(),
        use_mainnet: false,
    }
}

fn write_temp_file_with_extension(extension: &str, contents: &str) -> String {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let path = env::temp_dir().join(format!("kinic-provider-test-{unique_suffix}.{extension}"));
    fs::write(&path, contents).expect("temporary file should be writable");
    path.display().to_string()
}

fn live_memory(id: &str, title: &str) -> KinicRecord {
    KinicRecord::new(
        id,
        title,
        "memories",
        "Status: running",
        format!("detail for {id}"),
    )
}

fn running_memory_summary(id: &str, detail: &str) -> MemorySummary {
    MemorySummary {
        id: id.to_string(),
        status: "running".to_string(),
        detail: detail.to_string(),
    }
}

fn pending_search_context(request_id: u64, memory_id: &str, query: &str) -> SearchRequestContext {
    SearchRequestContext {
        request_id,
        memory_id: memory_id.to_string(),
        query: query.to_string(),
    }
}

fn refreshed_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_snapshot("aaaaa-aa"));
    overview.balance_base_units = Some(1_234_000_000u128);
    overview.price_base_units = Some(Nat::from(150_000_000u128));
    overview
}

fn balance_only_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_snapshot("aaaaa-aa"));
    overview.balance_base_units = Some(1_234_000_000u128);
    overview.price_error = Some("price unavailable".to_string());
    overview
}

fn price_only_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_snapshot("aaaaa-aa"));
    overview.balance_error = Some("ledger unavailable".to_string());
    overview.price_base_units = Some(Nat::from(150_000_000u128));
    overview
}

fn unavailable_session_overview() -> SessionAccountOverview {
    SessionAccountOverview::new(session_snapshot("aaaaa-aa"))
}

fn principal_error_session_overview() -> SessionAccountOverview {
    let mut overview = SessionAccountOverview::new(session_snapshot("unavailable"));
    overview.principal_error = Some("identity lookup failed".to_string());
    overview.balance_error = Some("ledger unavailable".to_string());
    overview
}

fn loaded_create_cost(overview: SessionAccountOverview) -> CreateCostState {
    let details = derive_create_cost(
        overview.session.principal_id.as_str(),
        overview.balance_base_units,
        overview.price_base_units.as_ref(),
    );
    CreateCostState::Loaded(Box::new(LoadedCreateCost { overview, details }))
}

fn mainnet_principal_error_session_overview() -> SessionAccountOverview {
    let mut overview =
        SessionAccountOverview::new(crate::tui::settings::session_settings_snapshot(
            &TuiAuth::resolved_for_tests(),
            true,
            None,
            "https://api.kinic.io".to_string(),
        ));
    overview.principal_error = Some("identity lookup failed".to_string());
    overview.balance_error = Some("ledger unavailable".to_string());
    overview
}

fn quick_entry_value<'a>(snapshot: &'a ProviderSnapshot, id: &str) -> &'a str {
    snapshot
        .settings
        .quick_entries
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.value.as_str())
        .expect("quick entry should exist")
}

fn section_entry_value<'a>(snapshot: &'a ProviderSnapshot, section: &str, id: &str) -> &'a str {
    snapshot
        .settings
        .sections
        .iter()
        .find(|current| current.title == section)
        .and_then(|current| current.entries.iter().find(|entry| entry.id == id))
        .map(|entry| entry.value.as_str())
        .expect("section entry should exist")
}

fn section_entry_note<'a>(
    snapshot: &'a ProviderSnapshot,
    section: &str,
    id: &str,
) -> Option<&'a str> {
    snapshot
        .settings
        .sections
        .iter()
        .find(|current| current.title == section)
        .and_then(|current| current.entries.iter().find(|entry| entry.id == id))
        .and_then(|entry| entry.note.as_deref())
}

#[path = "tests/insert_submit.rs"]
mod insert_submit;

#[path = "tests/live_browser.rs"]
mod live_browser;

#[path = "tests/search.rs"]
mod search;

#[path = "tests/settings.rs"]
mod settings;

#[path = "tests/snapshot.rs"]
mod snapshot;
