//! Desktop DTO conversion helpers.
//! Where: used by `rust/desktop/mod.rs`.
//! What: maps existing TUI/domain models into stable desktop DTOs.
//! Why: keep command orchestration small and readable.

use kinic_core::amount::{format_e8s_to_kinic_string_nat, format_e8s_to_kinic_string_u128};

use super::{
    DesktopMemoryDetails, DesktopMemorySummary, DesktopMemoryUser, DesktopPreferencesView,
    DesktopSearchResult, DesktopSession, DesktopSessionView, SearchPayload,
};
use crate::{
    embedding::embedding_base_url,
    preferences::{self, UserPreferences},
    tui::bridge::{MemoryDetails, MemorySummary, SearchResultItem},
};

pub(super) fn session_view(
    session: &DesktopSession,
    overview: tui_kit_runtime::SessionAccountOverview,
) -> DesktopSessionView {
    let mut errors = overview.account_issue_messages();
    let balance_kinic = overview
        .balance_base_units
        .map(format_e8s_to_kinic_string_u128);
    let required_total = match (overview.price_base_units.clone(), overview.fee_base_units) {
        (Some(price), Some(fee)) => Some(price + candid::Nat::from(fee)),
        _ => None,
    };
    let sufficient_balance = match (&required_total, overview.balance_base_units) {
        (Some(required), Some(balance)) => Some(candid::Nat::from(balance) >= *required),
        _ => None,
    };

    if overview.session.principal_id == "unavailable" && errors.is_empty() {
        errors.push("Principal unavailable.".to_string());
    }

    DesktopSessionView {
        identity: session.identity.clone(),
        network: session.network_label().to_string(),
        auth_mode: overview.session.auth_mode,
        principal_id: overview.session.principal_id,
        embedding_api_endpoint: embedding_base_url(),
        balance_base_units: overview.balance_base_units.map(|value| value.to_string()),
        balance_kinic,
        fee_base_units: overview.fee_base_units.map(|value| value.to_string()),
        price_base_units: overview.price_base_units.map(|value| value.to_string()),
        required_total_kinic: required_total.as_ref().map(format_e8s_to_kinic_string_nat),
        required_total_base_units: required_total.map(|value| value.to_string()),
        sufficient_balance,
        errors,
    }
}

pub(super) fn memory_summary_view(memory: MemorySummary) -> DesktopMemorySummary {
    DesktopMemorySummary {
        id: memory.id,
        status: memory.status,
        detail: memory.detail,
        searchable_memory_id: memory.searchable_memory_id,
        name: memory.name,
        version: memory.version,
        dim: memory.dim,
    }
}

pub(super) fn memory_details_view(
    memory_id: String,
    details: MemoryDetails,
) -> DesktopMemoryDetails {
    DesktopMemoryDetails {
        memory_id,
        display_name: details.display_name,
        metadata_name: details.metadata_name,
        version: details.version,
        dim: details.dim,
        owners: details.owners,
        stable_memory_size: details.stable_memory_size,
        cycle_amount: details.cycle_amount,
        users: details
            .users
            .into_iter()
            .map(|user| DesktopMemoryUser {
                principal_id: user.principal_id,
                role: user.role,
            })
            .collect(),
        users_load_error: details.users_load_error,
    }
}

pub(super) fn search_result_view(item: SearchResultItem) -> DesktopSearchResult {
    let payload = serde_json::from_str::<SearchPayload>(&item.payload).ok();
    DesktopSearchResult {
        memory_id: item.memory_id,
        score: item.score,
        tag: payload.as_ref().and_then(|payload| payload.tag.clone()),
        sentence: payload
            .and_then(|payload| payload.sentence)
            .unwrap_or(item.payload),
    }
}

pub(super) fn preferences_view(preferences: UserPreferences) -> DesktopPreferencesView {
    let preferences = preferences::normalize_user_preferences(preferences);
    DesktopPreferencesView {
        default_memory_id: preferences.default_memory_id,
        saved_tags: preferences.saved_tags,
        manual_memory_ids: preferences.manual_memory_ids,
        chat_overall_top_k: preferences.chat_overall_top_k,
        chat_per_memory_cap: preferences.chat_per_memory_cap,
        chat_mmr_lambda: preferences.chat_mmr_lambda,
    }
}
