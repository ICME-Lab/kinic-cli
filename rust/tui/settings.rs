//! Kinic TUI settings model.
//! Where: shared between provider and bridge in the embedded TUI runtime.
//! What: stores account/session snapshot data plus persisted user preferences and chat history.
//! Why: keep settings rendering stable while separating session/account data from saved prefs.

use std::time::{SystemTime, UNIX_EPOCH};

use kinic_core::amount::format_e8s_to_kinic_string_u128;
use serde::{Deserialize, Serialize};
use tui_kit_host::settings::SettingsError;
#[cfg(not(test))]
use tui_kit_host::settings::{load_yaml_or_default, save_yaml};
use tui_kit_runtime::{
    SETTINGS_ENTRY_CHAT_DIVERSITY_ID, SETTINGS_ENTRY_CHAT_PER_MEMORY_LIMIT_ID,
    SETTINGS_ENTRY_CHAT_RESULT_LIMIT_ID, SETTINGS_ENTRY_DEFAULT_MEMORY_ID,
    SETTINGS_ENTRY_EMBEDDING_MODEL_ID, SessionAccountOverview, SessionSettingsSnapshot,
    SettingsEntry, SettingsSection, SettingsSnapshot,
};

use crate::embedding_config::supported_embedding_backends;
use crate::preferences::{
    UserPreferences, chat_diversity_display, chat_per_memory_limit_display,
    chat_result_limit_display,
};
use crate::tui::TuiAuth;

#[cfg(not(test))]
const APP_NAMESPACE: &str = "kinic";
#[cfg(not(test))]
const CHAT_HISTORY_FILE_NAME: &str = "chat-threads.yaml";
const UNAVAILABLE: &str = "unavailable";
const NOT_SET: &str = "not set";
const EMBEDDING_MODEL_NOTE: &str = "Affects create/search/insert. API keeps existing memories usable. Local backends may require reindex. Same-dimension model mismatches are not detectable.";
const CHAT_HISTORY_MAX_MESSAGES: usize = 40;
const CHAT_MESSAGE_MAX_CONTENT_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PreferencesHealth {
    pub load_error: Option<String>,
    pub save_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChatHistoryStore {
    #[serde(default)]
    pub contexts: Vec<ChatContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatContext {
    pub network: String,
    #[serde(default)]
    pub principal_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub identity_label: String,
    pub thread_key: String,
    pub active_thread_id: String,
    #[serde(default)]
    pub threads: Vec<ChatThread>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatThread {
    pub thread_id: String,
    #[serde(default)]
    pub messages: Vec<StoredChatMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredChatMessage {
    pub role: String,
    pub content: String,
    pub saved_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveChatThread {
    pub thread_id: String,
    pub messages: Vec<(String, String)>,
}

pub fn current_chat_saved_at() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
pub fn load_or_create_active_chat_thread(
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
) -> Result<ActiveChatThread, SettingsError> {
    let mut store = test_chat_history_store()
        .lock()
        .expect("test chat history store lock should be available");
    Ok(load_or_create_active_chat_thread_in_store(
        &mut store,
        network,
        principal_id,
        identity_label,
        thread_key,
    ))
}

#[cfg(not(test))]
pub fn load_or_create_active_chat_thread(
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
) -> Result<ActiveChatThread, SettingsError> {
    let mut store: ChatHistoryStore = load_yaml_or_default(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME)?;
    let active_thread = load_or_create_active_chat_thread_in_store(
        &mut store,
        network,
        principal_id,
        identity_label,
        thread_key,
    );
    save_yaml(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME, &store)?;
    Ok(active_thread)
}

#[cfg(test)]
pub fn create_chat_thread(
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
) -> Result<String, SettingsError> {
    let mut store = test_chat_history_store()
        .lock()
        .expect("test chat history store lock should be available");
    Ok(create_chat_thread_in_store(
        &mut store,
        network,
        principal_id,
        identity_label,
        thread_key,
        next_chat_thread_id(),
    ))
}

#[cfg(not(test))]
pub fn create_chat_thread(
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
) -> Result<String, SettingsError> {
    let mut store: ChatHistoryStore = load_yaml_or_default(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME)?;
    let thread_id = create_chat_thread_in_store(
        &mut store,
        network,
        principal_id,
        identity_label,
        thread_key,
        next_chat_thread_id(),
    );
    save_yaml(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME, &store)?;
    Ok(thread_id)
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub fn append_chat_history_message(
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
    thread_id: &str,
    role: &str,
    content: &str,
    saved_at: u64,
) -> Result<(), SettingsError> {
    let mut guard = test_chat_history_store()
        .lock()
        .expect("test chat history store lock should be available");
    append_chat_history_message_to_store(
        &mut guard,
        network,
        principal_id,
        identity_label,
        thread_key,
        thread_id,
        role,
        content,
        saved_at,
    );
    Ok(())
}

#[cfg(not(test))]
#[allow(clippy::too_many_arguments)]
pub fn append_chat_history_message(
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
    thread_id: &str,
    role: &str,
    content: &str,
    saved_at: u64,
) -> Result<(), SettingsError> {
    let mut store: ChatHistoryStore = load_yaml_or_default(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME)?;
    append_chat_history_message_to_store(
        &mut store,
        network,
        principal_id,
        identity_label,
        thread_key,
        thread_id,
        role,
        content,
        saved_at,
    );
    save_yaml(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME, &store)
}

#[allow(clippy::too_many_arguments)]
fn append_chat_history_message_to_store(
    store: &mut ChatHistoryStore,
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
    thread_id: &str,
    role: &str,
    content: &str,
    saved_at: u64,
) {
    let normalized_content = clip_chat_message_content(content);
    if normalized_content.is_empty() {
        return;
    }

    let thread = ensure_chat_thread_mut(
        store,
        network,
        principal_id,
        identity_label,
        thread_key,
        thread_id,
    );
    if let Some(last) = thread.messages.last()
        && last.role == role
        && last.content == normalized_content
    {
        return;
    }

    thread.messages.push(StoredChatMessage {
        role: role.to_string(),
        content: normalized_content,
        saved_at,
    });
    normalize_thread_messages(&mut thread.messages);
}

fn load_or_create_active_chat_thread_in_store(
    store: &mut ChatHistoryStore,
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
) -> ActiveChatThread {
    let context_index =
        ensure_chat_context_index(store, network, principal_id, identity_label, thread_key);
    let context = store
        .contexts
        .get_mut(context_index)
        .expect("chat context should exist after upsert");
    if context.active_thread_id.is_empty()
        || !context
            .threads
            .iter()
            .any(|thread| thread.thread_id == context.active_thread_id)
    {
        context.active_thread_id = create_chat_thread_for_context(context, next_chat_thread_id())
            .thread_id
            .clone();
    }
    project_active_chat_thread(context)
}

fn create_chat_thread_in_store(
    store: &mut ChatHistoryStore,
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
    thread_id: String,
) -> String {
    let context_index =
        ensure_chat_context_index(store, network, principal_id, identity_label, thread_key);
    let context = store
        .contexts
        .get_mut(context_index)
        .expect("chat context should exist after upsert");
    context.active_thread_id = thread_id.clone();
    if !context
        .threads
        .iter()
        .any(|thread| thread.thread_id == thread_id)
    {
        create_chat_thread_for_context(context, thread_id.clone());
    }
    thread_id
}

fn ensure_chat_thread_mut<'a>(
    store: &'a mut ChatHistoryStore,
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
    thread_id: &str,
) -> &'a mut ChatThread {
    let context_index =
        ensure_chat_context_index(store, network, principal_id, identity_label, thread_key);
    let context = store
        .contexts
        .get_mut(context_index)
        .expect("chat context should exist after upsert");
    context.active_thread_id = thread_id.to_string();
    let thread_index = context
        .threads
        .iter()
        .position(|thread| thread.thread_id == thread_id)
        .unwrap_or_else(|| {
            create_chat_thread_for_context(context, thread_id.to_string());
            context.threads.len().saturating_sub(1)
        });
    context
        .threads
        .get_mut(thread_index)
        .expect("chat thread should exist after upsert")
}

fn ensure_chat_context_index(
    store: &mut ChatHistoryStore,
    network: &str,
    principal_id: &str,
    identity_label: &str,
    thread_key: &str,
) -> usize {
    if let Some(index) = store.contexts.iter().position(|context| {
        context.network == network
            && context.principal_id == principal_id
            && context.identity_label == identity_label
            && context.thread_key == thread_key
    }) {
        return index;
    }

    // Legacy records only had identity_label. Preserve that identity boundary while
    // backfilling principal_id so mixed old/new stores remain readable.
    if let Some(index) = store.contexts.iter().position(|context| {
        context.network == network
            && context.principal_id.is_empty()
            && context.identity_label == identity_label
            && context.thread_key == thread_key
    }) {
        let context = store
            .contexts
            .get_mut(index)
            .expect("legacy chat context should exist after lookup");
        context.principal_id = principal_id.to_string();
        return index;
    }

    store.contexts.push(ChatContext {
        network: network.to_string(),
        principal_id: principal_id.to_string(),
        identity_label: identity_label.to_string(),
        thread_key: thread_key.to_string(),
        active_thread_id: String::new(),
        threads: Vec::new(),
    });
    store.contexts.len().saturating_sub(1)
}

fn create_chat_thread_for_context(context: &mut ChatContext, thread_id: String) -> &mut ChatThread {
    context.threads.push(ChatThread {
        thread_id,
        messages: Vec::new(),
    });
    context
        .threads
        .last_mut()
        .expect("chat thread should exist after push")
}

fn project_active_chat_thread(context: &ChatContext) -> ActiveChatThread {
    let thread = context
        .threads
        .iter()
        .find(|thread| thread.thread_id == context.active_thread_id)
        .or_else(|| context.threads.last())
        .expect("chat context should always have at least one thread");
    let mut messages = thread.messages.clone();
    normalize_thread_messages(&mut messages);
    ActiveChatThread {
        thread_id: thread.thread_id.clone(),
        messages: messages
            .into_iter()
            .map(|message| (message.role, message.content))
            .collect(),
    }
}

fn normalize_thread_messages(messages: &mut Vec<StoredChatMessage>) {
    for message in messages.iter_mut() {
        message.content = clip_chat_message_content(&message.content);
    }
    messages.retain(|message| !message.content.is_empty());
    if messages.len() > CHAT_HISTORY_MAX_MESSAGES {
        let overflow = messages.len() - CHAT_HISTORY_MAX_MESSAGES;
        messages.drain(0..overflow);
    }
}

fn clip_chat_message_content(content: &str) -> String {
    content.chars().take(CHAT_MESSAGE_MAX_CONTENT_LEN).collect()
}

fn next_chat_thread_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("thread-{nanos}")
}

#[cfg(test)]
pub fn clear_chat_history_for_tests() {
    *test_chat_history_store()
        .lock()
        .expect("test chat history store lock should be available") = ChatHistoryStore::default();
}

#[cfg(test)]
fn test_chat_history_store() -> &'static std::sync::Mutex<ChatHistoryStore> {
    static STORE: std::sync::OnceLock<std::sync::Mutex<ChatHistoryStore>> =
        std::sync::OnceLock::new();
    STORE.get_or_init(|| std::sync::Mutex::new(ChatHistoryStore::default()))
}

pub fn build_settings_snapshot(
    overview: &SessionAccountOverview,
    preferences: &UserPreferences,
    selector_items: &[String],
    selector_labels: &[String],
    health: &PreferencesHealth,
) -> SettingsSnapshot {
    let session = overview.session.clone();
    let default_memory_display =
        default_memory_display(preferences, selector_items, selector_labels);
    let saved_tags_display = saved_tags_display(preferences);
    let embedding_model_display = embedding_model_display(preferences);
    let chat_result_limit_display = chat_result_limit_display(preferences.chat_overall_top_k);
    let chat_per_memory_limit_display =
        chat_per_memory_limit_display(preferences.chat_per_memory_cap);
    let chat_diversity_display = chat_diversity_display(preferences.chat_mmr_lambda);

    SettingsSnapshot {
        quick_entries: vec![
            SettingsEntry {
                id: "principal_id".to_string(),
                label: "Principal ID".to_string(),
                value: abbreviate_principal_id(session.principal_id.as_str()),
                note: None,
            },
            SettingsEntry {
                id: "kinic_balance".to_string(),
                label: "KINIC balance".to_string(),
                value: account_balance_value(overview),
                note: account_balance_note(overview),
            },
            SettingsEntry {
                id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                label: "Default memory".to_string(),
                value: default_memory_display.clone(),
                note: None,
            },
            SettingsEntry {
                id: "saved_tags".to_string(),
                label: "Saved tags".to_string(),
                value: saved_tags_display.clone(),
                note: None,
            },
            SettingsEntry {
                id: "embedding_api_endpoint".to_string(),
                label: "Chat API".to_string(),
                value: session.embedding_api_endpoint.clone(),
                note: None,
            },
        ],
        sections: vec![
            SettingsSection {
                title: "Saved preferences".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                        label: "Default memory".to_string(),
                        value: default_memory_display,
                        note: None,
                    },
                    SettingsEntry {
                        id: "saved_tags".to_string(),
                        label: "Saved tags".to_string(),
                        value: saved_tags_display,
                        note: None,
                    },
                    SettingsEntry {
                        id: SETTINGS_ENTRY_EMBEDDING_MODEL_ID.to_string(),
                        label: "Embedding backend".to_string(),
                        value: embedding_model_display,
                        note: Some(EMBEDDING_MODEL_NOTE.to_string()),
                    },
                    SettingsEntry {
                        id: "preferences_status".to_string(),
                        label: "Preferences status".to_string(),
                        value: preferences_status_label(health),
                        note: None,
                    },
                ],
                footer: None,
            },
            SettingsSection {
                title: "Chat retrieval".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: SETTINGS_ENTRY_CHAT_RESULT_LIMIT_ID.to_string(),
                        label: "Chat result limit".to_string(),
                        value: chat_result_limit_display,
                        note: None,
                    },
                    SettingsEntry {
                        id: SETTINGS_ENTRY_CHAT_PER_MEMORY_LIMIT_ID.to_string(),
                        label: "Per-memory limit".to_string(),
                        value: chat_per_memory_limit_display,
                        note: None,
                    },
                    SettingsEntry {
                        id: SETTINGS_ENTRY_CHAT_DIVERSITY_ID.to_string(),
                        label: "Chat diversity".to_string(),
                        value: chat_diversity_display,
                        note: None,
                    },
                ],
                footer: None,
            },
            SettingsSection {
                title: "Current session".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: "identity_name".to_string(),
                        label: "Identity name".to_string(),
                        value: session.identity_name.clone(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "auth_mode".to_string(),
                        label: "Auth mode".to_string(),
                        value: session.auth_mode.clone(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "network".to_string(),
                        label: "Network".to_string(),
                        value: session.network.clone(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "embedding_api_endpoint".to_string(),
                        label: "Chat API".to_string(),
                        value: session.embedding_api_endpoint.clone(),
                        note: None,
                    },
                ],
                footer: None,
            },
            SettingsSection {
                title: "Account".to_string(),
                entries: account_entries(overview),
                footer: None,
            },
        ],
    }
}

pub fn session_settings_snapshot(
    auth: &TuiAuth,
    use_mainnet: bool,
    principal_id: Option<String>,
    embedding_api_endpoint: String,
) -> SessionSettingsSnapshot {
    SessionSettingsSnapshot {
        auth_mode: auth_mode_label(auth),
        identity_name: identity_name_label(auth),
        principal_id: principal_id.unwrap_or_else(|| UNAVAILABLE.to_string()),
        network: network_label(use_mainnet),
        embedding_api_endpoint,
    }
}

fn auth_mode_label(auth: &TuiAuth) -> String {
    match auth {
        TuiAuth::DeferredIdentity { .. } => "keyring identity".to_string(),
        TuiAuth::ResolvedIdentity(_) => "live identity".to_string(),
    }
}

fn identity_name_label(auth: &TuiAuth) -> String {
    match auth {
        TuiAuth::DeferredIdentity { identity_name, .. } => identity_name.clone(),
        TuiAuth::ResolvedIdentity(_) => "provided".to_string(),
    }
}

fn network_label(use_mainnet: bool) -> String {
    if use_mainnet {
        "mainnet".to_string()
    } else {
        "local".to_string()
    }
}

fn default_memory_display(
    preferences: &UserPreferences,
    selector_items: &[String],
    selector_labels: &[String],
) -> String {
    match preferences.default_memory_id.as_ref() {
        Some(memory_id) if selector_items.is_empty() => memory_id.clone(),
        Some(memory_id) => selector_items
            .iter()
            .position(|item| item == memory_id)
            .and_then(|index| selector_labels.get(index).cloned())
            .unwrap_or_else(|| format!("{memory_id} (missing)")),
        None => NOT_SET.to_string(),
    }
}

fn account_entries(overview: &SessionAccountOverview) -> Vec<SettingsEntry> {
    vec![
        SettingsEntry {
            id: "principal_id".to_string(),
            label: "Principal ID".to_string(),
            value: overview.session.principal_id.clone(),
            note: None,
        },
        SettingsEntry {
            id: "kinic_balance".to_string(),
            label: "KINIC balance".to_string(),
            value: account_balance_value(overview),
            note: account_balance_note(overview),
        },
    ]
}

fn account_balance_value(overview: &SessionAccountOverview) -> String {
    overview
        .balance_base_units
        .map(format_e8s_to_kinic_string_u128)
        .map(|value| format_kinic_value(truncate_kinic_fraction_to_three_digits(value).as_str()))
        .unwrap_or_else(|| UNAVAILABLE.to_string())
}

fn account_balance_note(overview: &SessionAccountOverview) -> Option<String> {
    overview.account_issue_note()
}

fn abbreviate_principal_id(value: &str) -> String {
    if value == UNAVAILABLE {
        return value.to_string();
    }

    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= 10 {
        return value.to_string();
    }

    let prefix = chars.iter().take(5).collect::<String>();
    let suffix = chars
        .iter()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix}...{suffix}")
}

fn format_kinic_value(value: &str) -> String {
    format!("{value} KINIC")
}

/// Shortens the fractional part of a KINIC amount string for compact UI (up to three digits,
/// truncation not rounding). Full precision strings come from `format_e8s_to_kinic_string_*`.
fn truncate_kinic_fraction_to_three_digits(value: String) -> String {
    match value.split_once('.') {
        Some((whole, fraction)) => {
            let limited = &fraction[..fraction.len().min(3)];
            format!("{whole}.{limited}")
        }
        None => value,
    }
}

fn preferences_status_label(health: &PreferencesHealth) -> String {
    match (&health.load_error, &health.save_error) {
        (Some(_), _) => "preferences unavailable".to_string(),
        (None, Some(_)) => "last save failed".to_string(),
        (None, None) => "ok".to_string(),
    }
}

fn saved_tags_display(preferences: &UserPreferences) -> String {
    if preferences.saved_tags.is_empty() {
        return NOT_SET.to_string();
    }

    preferences.saved_tags.join(", ")
}

fn embedding_model_display(preferences: &UserPreferences) -> String {
    supported_embedding_backends()
        .into_iter()
        .find(|model| model.id == preferences.embedding_model_id)
        .map(|model| format!("{} ({})", model.label, model.dimension))
        .unwrap_or_else(|| preferences.embedding_model_id.clone())
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
