//! Kinic TUI settings model.
//! Where: shared between provider and bridge in the embedded TUI runtime.
//! What: stores account/session snapshot data plus persisted user preferences and chat history.
//! Why: keep settings rendering stable while separating session/account data from saved prefs.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tui_kit_host::settings::SettingsError;
#[cfg(not(test))]
use tui_kit_host::settings::{load_yaml_or_default, save_yaml};
use tui_kit_runtime::{
    SETTINGS_ENTRY_CHAT_CANDIDATE_POOL_ID, SETTINGS_ENTRY_CHAT_DIVERSITY_ID,
    SETTINGS_ENTRY_CHAT_PER_MEMORY_LIMIT_ID, SETTINGS_ENTRY_CHAT_RESULT_LIMIT_ID,
    SETTINGS_ENTRY_DEFAULT_MEMORY_ID, SessionAccountOverview, SessionSettingsSnapshot,
    SettingsEntry, SettingsSection, SettingsSnapshot, format_e8s_to_kinic_string_u128,
};

use crate::tui::TuiAuth;

#[cfg(not(test))]
const APP_NAMESPACE: &str = "kinic";
#[cfg(not(test))]
const SETTINGS_FILE_NAME: &str = "tui.yaml";
#[cfg(not(test))]
const CHAT_HISTORY_FILE_NAME: &str = "chat-threads.yaml";
const UNAVAILABLE: &str = "unavailable";
const NOT_SET: &str = "not set";
const CHAT_HISTORY_MAX_MESSAGES: usize = 40;
const CHAT_MESSAGE_MAX_CONTENT_LEN: usize = 4096;
pub const DEFAULT_CHAT_OVERALL_TOP_K: usize = 8;
pub const DEFAULT_CHAT_PER_MEMORY_CAP: usize = 3;
pub const DEFAULT_CHAT_CANDIDATE_POOL_SIZE: usize = 24;
pub const DEFAULT_CHAT_MMR_LAMBDA: u8 = 70;
const CHAT_RESULT_LIMIT_OPTIONS: &[usize] = &[4, 6, 8, 10, 12];
const CHAT_PER_MEMORY_LIMIT_OPTIONS: &[usize] = &[1, 2, 3, 4];
const CHAT_CANDIDATE_POOL_OPTIONS: &[usize] = &[12, 16, 24, 32, 48];
const CHAT_DIVERSITY_OPTIONS: &[u8] = &[60, 70, 80, 90];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// The current on-disk schema is intentionally fixed; unsupported legacy shapes fail to decode.
pub struct UserPreferences {
    pub default_memory_id: Option<String>,
    pub saved_tags: Vec<String>,
    pub manual_memory_ids: Vec<String>,
    #[serde(default = "default_chat_overall_top_k")]
    pub chat_overall_top_k: usize,
    #[serde(default = "default_chat_per_memory_cap")]
    pub chat_per_memory_cap: usize,
    #[serde(default = "default_chat_candidate_pool_size")]
    pub chat_candidate_pool_size: usize,
    #[serde(default = "default_chat_mmr_lambda")]
    pub chat_mmr_lambda: u8,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            default_memory_id: None,
            saved_tags: Vec::new(),
            manual_memory_ids: Vec::new(),
            chat_overall_top_k: DEFAULT_CHAT_OVERALL_TOP_K,
            chat_per_memory_cap: DEFAULT_CHAT_PER_MEMORY_CAP,
            chat_candidate_pool_size: DEFAULT_CHAT_CANDIDATE_POOL_SIZE,
            chat_mmr_lambda: DEFAULT_CHAT_MMR_LAMBDA,
        }
    }
}

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

pub fn default_chat_overall_top_k() -> usize {
    DEFAULT_CHAT_OVERALL_TOP_K
}

pub fn default_chat_per_memory_cap() -> usize {
    DEFAULT_CHAT_PER_MEMORY_CAP
}

pub fn default_chat_candidate_pool_size() -> usize {
    DEFAULT_CHAT_CANDIDATE_POOL_SIZE
}

pub fn default_chat_mmr_lambda() -> u8 {
    DEFAULT_CHAT_MMR_LAMBDA
}

#[cfg(test)]
pub fn load_user_preferences() -> Result<UserPreferences, SettingsError> {
    Ok(normalize_user_preferences(UserPreferences::default()))
}

#[cfg(not(test))]
pub fn load_user_preferences() -> Result<UserPreferences, SettingsError> {
    let preferences: UserPreferences = load_yaml_or_default(APP_NAMESPACE, SETTINGS_FILE_NAME)?;
    Ok(normalize_user_preferences(preferences))
}

#[cfg(test)]
pub fn save_user_preferences(_preferences: &UserPreferences) -> Result<(), SettingsError> {
    Ok(())
}

#[cfg(not(test))]
pub fn save_user_preferences(preferences: &UserPreferences) -> Result<(), SettingsError> {
    save_yaml(
        APP_NAMESPACE,
        SETTINGS_FILE_NAME,
        &normalize_user_preferences(preferences.clone()),
    )
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
    identity_label: &str,
    thread_key: &str,
) -> Result<ActiveChatThread, SettingsError> {
    let mut store = test_chat_history_store()
        .lock()
        .expect("test chat history store lock should be available");
    Ok(load_or_create_active_chat_thread_in_store(
        &mut store,
        network,
        identity_label,
        thread_key,
    ))
}

#[cfg(not(test))]
pub fn load_or_create_active_chat_thread(
    network: &str,
    identity_label: &str,
    thread_key: &str,
) -> Result<ActiveChatThread, SettingsError> {
    let mut store: ChatHistoryStore = load_yaml_or_default(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME)?;
    let active_thread =
        load_or_create_active_chat_thread_in_store(&mut store, network, identity_label, thread_key);
    save_yaml(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME, &store)?;
    Ok(active_thread)
}

#[cfg(test)]
pub fn create_chat_thread(
    network: &str,
    identity_label: &str,
    thread_key: &str,
) -> Result<String, SettingsError> {
    let mut store = test_chat_history_store()
        .lock()
        .expect("test chat history store lock should be available");
    Ok(create_chat_thread_in_store(
        &mut store,
        network,
        identity_label,
        thread_key,
        next_chat_thread_id(),
    ))
}

#[cfg(not(test))]
pub fn create_chat_thread(
    network: &str,
    identity_label: &str,
    thread_key: &str,
) -> Result<String, SettingsError> {
    let mut store: ChatHistoryStore = load_yaml_or_default(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME)?;
    let thread_id = create_chat_thread_in_store(
        &mut store,
        network,
        identity_label,
        thread_key,
        next_chat_thread_id(),
    );
    save_yaml(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME, &store)?;
    Ok(thread_id)
}

#[cfg(test)]
pub fn append_chat_history_message(
    network: &str,
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
pub fn append_chat_history_message(
    network: &str,
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
        identity_label,
        thread_key,
        thread_id,
        role,
        content,
        saved_at,
    );
    save_yaml(APP_NAMESPACE, CHAT_HISTORY_FILE_NAME, &store)
}

fn append_chat_history_message_to_store(
    store: &mut ChatHistoryStore,
    network: &str,
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

    let thread = ensure_chat_thread_mut(store, network, identity_label, thread_key, thread_id);
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
    identity_label: &str,
    thread_key: &str,
) -> ActiveChatThread {
    let context_index = ensure_chat_context_index(store, network, identity_label, thread_key);
    let context = store
        .contexts
        .get_mut(context_index)
        .expect("chat context should exist after upsert");
    if context.active_thread_id.is_empty() || !context.threads.iter().any(|thread| thread.thread_id == context.active_thread_id) {
        context.active_thread_id =
            create_chat_thread_for_context(context, next_chat_thread_id()).thread_id.clone();
    }
    project_active_chat_thread(context)
}

fn create_chat_thread_in_store(
    store: &mut ChatHistoryStore,
    network: &str,
    identity_label: &str,
    thread_key: &str,
    thread_id: String,
) -> String {
    let context_index = ensure_chat_context_index(store, network, identity_label, thread_key);
    let context = store
        .contexts
        .get_mut(context_index)
        .expect("chat context should exist after upsert");
    context.active_thread_id = thread_id.clone();
    if !context.threads.iter().any(|thread| thread.thread_id == thread_id) {
        create_chat_thread_for_context(context, thread_id.clone());
    }
    thread_id
}

fn ensure_chat_thread_mut<'a>(
    store: &'a mut ChatHistoryStore,
    network: &str,
    identity_label: &str,
    thread_key: &str,
    thread_id: &str,
) -> &'a mut ChatThread {
    let context_index = ensure_chat_context_index(store, network, identity_label, thread_key);
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
    identity_label: &str,
    thread_key: &str,
) -> usize {
    store
        .contexts
        .iter()
        .position(|context| {
            context.network == network
                && context.identity_label == identity_label
                && context.thread_key == thread_key
        })
        .unwrap_or_else(|| {
            store.contexts.push(ChatContext {
                network: network.to_string(),
                identity_label: identity_label.to_string(),
                thread_key: thread_key.to_string(),
                active_thread_id: String::new(),
                threads: Vec::new(),
            });
            store.contexts.len().saturating_sub(1)
        })
}

fn create_chat_thread_for_context(
    context: &mut ChatContext,
    thread_id: String,
) -> &mut ChatThread {
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
    let chat_result_limit_display = chat_result_limit_display(preferences.chat_overall_top_k);
    let chat_per_memory_limit_display =
        chat_per_memory_limit_display(preferences.chat_per_memory_cap);
    let chat_candidate_pool_display =
        chat_candidate_pool_display(preferences.chat_candidate_pool_size);
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
                label: "Embedding".to_string(),
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
                        id: SETTINGS_ENTRY_CHAT_CANDIDATE_POOL_ID.to_string(),
                        label: "Chat candidate pool".to_string(),
                        value: chat_candidate_pool_display,
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
                        label: "Embedding".to_string(),
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

pub(crate) fn normalize_saved_tags(mut tags: Vec<String>) -> Vec<String> {
    tags = tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

pub fn normalize_user_preferences(mut preferences: UserPreferences) -> UserPreferences {
    preferences.saved_tags = normalize_saved_tags(preferences.saved_tags);
    preferences.manual_memory_ids = normalize_manual_memory_ids(preferences.manual_memory_ids);
    preferences.chat_overall_top_k = normalize_chat_overall_top_k(preferences.chat_overall_top_k);
    preferences.chat_per_memory_cap =
        normalize_chat_per_memory_cap(preferences.chat_per_memory_cap);
    preferences.chat_candidate_pool_size =
        normalize_chat_candidate_pool_size(preferences.chat_candidate_pool_size);
    preferences.chat_mmr_lambda = normalize_chat_mmr_lambda(preferences.chat_mmr_lambda);
    preferences
}

pub fn normalize_chat_overall_top_k(value: usize) -> usize {
    if CHAT_RESULT_LIMIT_OPTIONS.contains(&value) {
        value
    } else {
        DEFAULT_CHAT_OVERALL_TOP_K
    }
}

pub fn normalize_chat_per_memory_cap(value: usize) -> usize {
    if CHAT_PER_MEMORY_LIMIT_OPTIONS.contains(&value) {
        value
    } else {
        DEFAULT_CHAT_PER_MEMORY_CAP
    }
}

pub fn normalize_chat_candidate_pool_size(value: usize) -> usize {
    if CHAT_CANDIDATE_POOL_OPTIONS.contains(&value) {
        value
    } else {
        DEFAULT_CHAT_CANDIDATE_POOL_SIZE
    }
}

pub fn normalize_chat_mmr_lambda(value: u8) -> u8 {
    if CHAT_DIVERSITY_OPTIONS.contains(&value) {
        value
    } else {
        DEFAULT_CHAT_MMR_LAMBDA
    }
}

pub fn chat_result_limit_options() -> &'static [usize] {
    CHAT_RESULT_LIMIT_OPTIONS
}

pub fn chat_per_memory_limit_options() -> &'static [usize] {
    CHAT_PER_MEMORY_LIMIT_OPTIONS
}

pub fn chat_candidate_pool_options() -> &'static [usize] {
    CHAT_CANDIDATE_POOL_OPTIONS
}

pub fn chat_diversity_options() -> &'static [u8] {
    CHAT_DIVERSITY_OPTIONS
}

pub fn chat_result_limit_display(value: usize) -> String {
    format!("{value} docs")
}

pub fn chat_per_memory_limit_display(value: usize) -> String {
    format!("{value} per memory")
}

pub fn chat_candidate_pool_display(value: usize) -> String {
    format!("{value} candidates")
}

pub fn chat_diversity_display(value: u8) -> String {
    format!("{:.2}", f32::from(value) / 100.0)
}

fn saved_tags_display(preferences: &UserPreferences) -> String {
    if preferences.saved_tags.is_empty() {
        return NOT_SET.to_string();
    }

    preferences.saved_tags.join(", ")
}

#[cfg_attr(test, allow(dead_code))]
fn normalize_manual_memory_ids(memory_ids: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for memory_id in memory_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        if !unique.iter().any(|existing| existing == &memory_id) {
            unique.push(memory_id);
        }
    }
    unique
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
