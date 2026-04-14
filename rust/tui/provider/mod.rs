#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{Arc, mpsc},
    thread,
};

use super::adapter;
use super::bridge::{self, MemorySummary, SearchResultItem};
use super::chat_prompt::ActiveMemoryContext;
use super::settings::{self, PreferencesHealth};
use crate::{
    agent::{KeychainErrorCode, extract_keychain_error_code},
    create_domain::derive_create_cost,
    embedding::fetch_embedding,
    insert_service::{
        InsertRequest, parse_embedding_json, validate_insert_request_fields,
        validate_insert_request_for_submit,
    },
    preferences::{self, UserPreferences},
    shared::{
        cross_memory_search::{collect_searchable_memory_ids, fold_search_batches},
        memory_metadata::parse_memory_metadata,
    },
    tui::TuiAuth,
};
use kinic_core::{
    amount::{
        KinicAmountParseError, format_e8s_to_kinic_string_u128, parse_required_kinic_amount_to_e8s,
    },
    prefs_policy,
    principal::parse_required_principal,
    tag,
};
use serde::Deserialize;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tui_kit_runtime::{
    AccessControlAction, AccessControlMode, AccessControlRole, ChatScope, CoreAction, CoreEffect,
    CoreResult, CoreState, CreateCostState, DataProvider, FILE_MODE_ALLOWED_EXTENSIONS, InsertMode,
    LoadedCreateCost, MemorySelection, PaneFocus, PickerConfirmKind, PickerContext, PickerItem,
    PickerItemKind, PickerListMode, PickerState, ProviderOutput, ProviderSnapshot, SearchScope,
    SessionAccountOverview, SessionSettingsSnapshot, TransferModalMode,
    kinic_tabs::{
        KINIC_CREATE_TAB_ID, KINIC_INSERT_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID,
        KINIC_SETTINGS_TAB_ID,
    },
};

#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub auth: TuiAuth,
    pub use_mainnet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KinicRecord {
    pub id: String,
    pub title: String,
    pub group: String,
    pub summary: String,
    pub content_md: String,
    pub searchable_memory_id: Option<String>,
    pub source_memory_id: Option<String>,
}

impl KinicRecord {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        group: impl Into<String>,
        summary: impl Into<String>,
        content_md: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            group: group.into(),
            summary: summary.into(),
            content_md: content_md.into(),
            searchable_memory_id: None,
            source_memory_id: None,
        }
    }

    pub fn with_searchable_memory_id_option(mut self, memory_id: Option<String>) -> Self {
        self.searchable_memory_id = memory_id;
        self
    }

    pub fn with_source_memory_id(mut self, memory_id: impl Into<String>) -> Self {
        self.source_memory_id = Some(memory_id.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoriesMode {
    Browser,
    Results,
}

#[derive(Debug, Deserialize)]
struct SearchPayload {
    sentence: Option<String>,
    tag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveTagResult {
    Saved,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SaveTagOutcome {
    result: SaveTagResult,
    effect: CoreEffect,
}

const MAX_CONCURRENT_MEMORY_SEARCHES: usize = 10;
const MAX_CONCURRENT_MEMORY_DETAIL_PREFETCHES: usize = 4;
const ADD_MEMORY_ACTION_ID: &str = "kinic-action-add-memory";
const ALL_MEMORIES_CHAT_THREAD_KEY: &str = "all-memories";
#[cfg_attr(test, allow(dead_code))]
const MEMORY_SUMMARY_QUERY: &str = "Summarize the contents of this memory concisely. Explain the main topics, what kinds of information it contains, and what the memory appears to be for, in 3 to 5 sentences.";
const MEMORY_SUMMARY_LOADING_TEXT: &str = "Loading summary...";
const MEMORY_SUMMARY_UNAVAILABLE_TEXT: &str = "Summary unavailable.";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveChatThreadRef {
    thread_key: String,
    thread_id: String,
}

pub struct KinicProvider {
    all: Vec<KinicRecord>,
    query: String,
    tab_id: String,
    config: TuiConfig,
    session_overview: SessionAccountOverview,
    user_preferences: UserPreferences,
    preferences_health: PreferencesHealth,
    active_memory: Option<MemorySelection>,
    memory_summaries: Vec<MemorySummary>,
    memory_records: Vec<KinicRecord>,
    result_records: Vec<KinicRecord>,
    memories_mode: MemoriesMode,
    pending_initial_memories: Option<mpsc::Receiver<InitialMemoriesTaskOutput>>,
    initial_memories_in_flight: bool,
    pending_search: Option<PendingSearch>,
    next_search_request_id: u64,
    last_search_state: Option<LastSearchState>,
    create_cost_state: CreateCostState,
    create_cost_task: RequestTaskState<CreateCostTaskOutput>,
    create_submit_task: RequestTaskState<CreateSubmitTaskOutput>,
    chat_submit_task: RequestTaskState<ChatTaskOutput>,
    active_chat_thread: Option<ActiveChatThreadRef>,
    session_settings_task: RequestTaskState<SessionSettingsTaskOutput>,
    next_session_settings_request_id: u64,
    next_create_request_id: u64,
    next_chat_request_id: u64,
    insert_submit_task: RequestTaskState<InsertSubmitTaskOutput>,
    next_insert_request_id: u64,
    insert_expected_dim_memory_id: Option<String>,
    insert_expected_dim: Option<u64>,
    insert_expected_dim_loading: bool,
    insert_expected_dim_load_error: Option<String>,
    pending_insert_dim: Option<mpsc::Receiver<InsertDimTaskOutput>>,
    pending_access_submit: Option<mpsc::Receiver<AccessSubmitTaskOutput>>,
    access_submit_in_flight: bool,
    add_memory_validation_task: TaskState<AddMemoryValidationTaskOutput>,
    rename_submit_task: TaskState<RenameSubmitTaskOutput>,
    transfer_prerequisites_task: TaskState<TransferPrerequisitesTaskOutput>,
    transfer_submit_task: TaskState<TransferSubmitTaskOutput>,
    preferred_memory_after_refresh: Option<String>,
    memory_content_summaries: HashMap<String, String>,
    failed_memory_content_summaries: HashMap<String, String>,
    memory_summary_tasks: HashMap<String, RequestTaskState<MemorySummaryTaskOutput>>,
    next_memory_summary_request_id: u64,
    loaded_memory_details: HashSet<String>,
    pending_memory_detail: RequestTaskState<MemoryDetailTaskOutput>,
    next_memory_detail_request_id: u64,
    pending_memory_detail_memory_id: Option<String>,
    memory_detail_prefetch: MemoryDetailPrefetchState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchRequestContext {
    request_id: u64,
    query: String,
    scope: SearchScope,
    target_memory_ids: Vec<String>,
}

struct SearchTaskOutput {
    request_id: u64,
    query: String,
    scope: SearchScope,
    target_memory_ids: Vec<String>,
    result: Result<SearchBatchResult, String>,
}

/// In-flight memory search with explicit cancellation. Kept separate from
/// `RequestTaskState` because workers use `CancellationToken` and batching
/// differs from other request/response tasks.
struct PendingSearch {
    receiver: mpsc::Receiver<SearchTaskOutput>,
    cancellation: CancellationToken,
    context: SearchRequestContext,
}

type MemorySearchTaskResult = (String, anyhow::Result<Vec<SearchResultItem>>);
type MemorySearchJoinResult = Result<MemorySearchTaskResult, tokio::task::JoinError>;
type NextMemorySearchTask = Option<MemorySearchJoinResult>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LastSearchState {
    scope: SearchScope,
    target_memory_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct SearchBatchResult {
    items: Vec<SearchResultItem>,
    failed_memory_ids: Vec<String>,
    join_error_count: usize,
}

fn fold_live_search_results(
    target_count: usize,
    results: Vec<MemorySearchTaskResult>,
    join_errors: Vec<tokio::task::JoinError>,
) -> Result<SearchBatchResult, String> {
    let folded = fold_search_batches(
        target_count,
        results,
        join_errors
            .into_iter()
            .map(|error| error.to_string())
            .collect(),
        "Search failed before any memory returned results.",
    )?;
    Ok(SearchBatchResult {
        items: folded.items,
        failed_memory_ids: folded.failed_memory_ids,
        join_error_count: folded.join_error_count,
    })
}

struct InitialMemoriesTaskOutput {
    result: Result<Vec<MemorySummary>, String>,
}

struct CreateCostTaskOutput {
    request_id: u64,
    overview: SessionAccountOverview,
}

struct CreateSubmitTaskOutput {
    request_id: u64,
    result: Result<bridge::CreateMemorySuccess, bridge::CreateMemoryError>,
}

struct ChatTaskOutput {
    request_id: u64,
    history_thread_key: String,
    thread_id: String,
    result: Result<bridge::AskMemoriesOutput, String>,
}

struct MemorySummaryTaskOutput {
    request_id: u64,
    memory_id: String,
    result: Result<bridge::AskMemoriesOutput, String>,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedChatRequest {
    history_thread_key: String,
    thread_id: String,
    scope: ChatScope,
    target_memory_ids: Vec<String>,
    query: String,
    history: Vec<(String, String)>,
    active_memory_context: Option<ActiveMemoryContext>,
}

struct SessionSettingsTaskOutput {
    request_id: u64,
    overview: SessionAccountOverview,
}

struct TransferPrerequisitesTaskOutput {
    result: Result<(u128, u128), bridge::TransferKinicError>,
}

struct TransferSubmitTaskOutput {
    result: Result<bridge::TransferKinicSuccess, bridge::TransferKinicError>,
}

#[derive(Clone, Copy)]
struct DefaultMemorySelection<'a> {
    memory_records: &'a [KinicRecord],
    user_preferences: &'a UserPreferences,
}

impl<'a> DefaultMemorySelection<'a> {
    fn available_memory_ids(self) -> Vec<String> {
        self.memory_records
            .iter()
            .map(|record| record.id.clone())
            .collect()
    }

    fn selector_labels(self) -> Vec<String> {
        self.memory_records
            .iter()
            .map(|record| {
                let title = record.title.trim();
                if title.is_empty() {
                    record.id.clone()
                } else {
                    title.to_string()
                }
            })
            .collect()
    }

    fn preferred_initial_memory_id(self) -> Option<String> {
        let default_memory_id = self.user_preferences.default_memory_id.as_deref()?;
        self.memory_records
            .iter()
            .find(|record| record.id == default_memory_id)
            .map(|record| record.id.clone())
    }

    fn is_default_memory(self, memory_id: &str) -> bool {
        self.user_preferences.default_memory_id.as_deref() == Some(memory_id)
    }

    fn title_for_id(self, memory_id: &str) -> Option<String> {
        self.memory_records.iter().find_map(|record| {
            if record.id != memory_id {
                return None;
            }
            let title = record.title.trim();
            Some(if title.is_empty() {
                record.id.clone()
            } else {
                title.to_string()
            })
        })
    }
}

fn saved_tag_selection(preferences: &UserPreferences) -> Vec<String> {
    tag::normalize_saved_tags(preferences.saved_tags.clone())
}

fn add_action_label_for_context(context: PickerContext) -> Option<&'static str> {
    match context {
        PickerContext::InsertTag | PickerContext::TagManagement => Some("+ Add new tag"),
        PickerContext::DefaultMemory
        | PickerContext::InsertTarget
        | PickerContext::AddTag
        | PickerContext::ChatResultLimit
        | PickerContext::ChatPerMemoryLimit
        | PickerContext::ChatDiversity => None,
    }
}

fn picker_selected_id_for_context(
    context: PickerContext,
    state: &CoreState,
    active_memory: Option<&MemorySelection>,
    user_preferences: &UserPreferences,
) -> Option<String> {
    match context {
        PickerContext::DefaultMemory => state.saved_default_memory_id.clone(),
        PickerContext::InsertTarget => active_memory.map(|selection| selection.id.clone()),
        PickerContext::InsertTag => {
            let insert_tag = state.insert_tag.trim();
            (!insert_tag.is_empty()).then(|| insert_tag.to_string())
        }
        PickerContext::TagManagement | PickerContext::AddTag => None,
        PickerContext::ChatResultLimit => Some(user_preferences.chat_overall_top_k.to_string()),
        PickerContext::ChatPerMemoryLimit => Some(user_preferences.chat_per_memory_cap.to_string()),
        PickerContext::ChatDiversity => Some(user_preferences.chat_mmr_lambda.to_string()),
    }
}

fn picker_items_for_context(
    context: PickerContext,
    state: &CoreState,
    memory_selection: DefaultMemorySelection<'_>,
    user_preferences: &UserPreferences,
) -> Vec<PickerItem> {
    match context {
        PickerContext::DefaultMemory | PickerContext::InsertTarget => memory_selection
            .memory_records
            .iter()
            .map(|record| {
                PickerItem::option(
                    record.id.clone(),
                    memory_selection
                        .title_for_id(record.id.as_str())
                        .unwrap_or_else(|| record.id.clone()),
                    memory_selection.is_default_memory(record.id.as_str()),
                )
            })
            .collect(),
        PickerContext::InsertTag | PickerContext::TagManagement => {
            let current_insert_tag = state.insert_tag.trim();
            let mut items = saved_tag_selection(user_preferences)
                .into_iter()
                .map(|tag| {
                    let is_current_insert_tag =
                        !current_insert_tag.is_empty() && current_insert_tag == tag;
                    PickerItem::option(tag.clone(), tag, is_current_insert_tag)
                })
                .collect::<Vec<_>>();
            if let Some(label) = add_action_label_for_context(context) {
                items.push(PickerItem::add_action(label));
            }
            items
        }
        PickerContext::AddTag => match &state.picker {
            PickerState::Input { origin_context, .. } => origin_context
                .map(|origin_context| {
                    picker_items_for_context(
                        origin_context,
                        state,
                        memory_selection,
                        user_preferences,
                    )
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        },
        PickerContext::ChatResultLimit => prefs_policy::chat_result_limit_options()
            .iter()
            .map(|value| {
                PickerItem::option(
                    value.to_string(),
                    preferences::chat_result_limit_display(*value),
                    user_preferences.chat_overall_top_k == *value,
                )
            })
            .collect(),
        PickerContext::ChatPerMemoryLimit => prefs_policy::chat_per_memory_limit_options()
            .iter()
            .map(|value| {
                PickerItem::option(
                    value.to_string(),
                    preferences::chat_per_memory_limit_display(*value),
                    user_preferences.chat_per_memory_cap == *value,
                )
            })
            .collect(),
        PickerContext::ChatDiversity => prefs_policy::chat_diversity_options()
            .iter()
            .map(|value| {
                PickerItem::option(
                    value.to_string(),
                    preferences::chat_diversity_display(*value),
                    user_preferences.chat_mmr_lambda == *value,
                )
            })
            .collect(),
    }
}

struct DefaultMemoryController<'a> {
    user_preferences: &'a mut UserPreferences,
    preferences_health: &'a mut PreferencesHealth,
}

impl<'a> DefaultMemoryController<'a> {
    fn apply_reloaded_preferences(
        &mut self,
        updated_preferences: UserPreferences,
        reloaded_preferences: Result<UserPreferences, tui_kit_host::settings::SettingsError>,
    ) {
        *self.user_preferences = match reloaded_preferences {
            Ok(preferences) => {
                *self.preferences_health = PreferencesHealth::default();
                preferences
            }
            Err(error) => {
                *self.preferences_health = PreferencesHealth {
                    load_error: Some(error.to_string()),
                    save_error: None,
                };
                updated_preferences
            }
        };
    }

    fn set_default_memory_preference(&mut self, memory_id: String) -> CoreEffect {
        if self.user_preferences.default_memory_id.as_deref() == Some(memory_id.as_str()) {
            return CoreEffect::Notify(format!("Default memory already set to {memory_id}"));
        }

        let updated_preferences = UserPreferences {
            default_memory_id: Some(memory_id.clone()),
            saved_tags: self.user_preferences.saved_tags.clone(),
            manual_memory_ids: self.user_preferences.manual_memory_ids.clone(),
            chat_overall_top_k: self.user_preferences.chat_overall_top_k,
            chat_per_memory_cap: self.user_preferences.chat_per_memory_cap,
            chat_mmr_lambda: self.user_preferences.chat_mmr_lambda,
        };
        #[cfg(test)]
        let _settings_io_lock = settings_io_lock()
            .lock()
            .expect("settings io lock should be available");
        match save_user_preferences_for_apply(&updated_preferences) {
            Ok(()) => {
                let reloaded_preferences = reload_preferences_for_apply(&updated_preferences);
                self.apply_reloaded_preferences(updated_preferences, reloaded_preferences);
                CoreEffect::Notify(format!("Default memory set to {memory_id}"))
            }
            Err(error) => {
                self.preferences_health.save_error = Some(error.to_string());
                CoreEffect::Notify(format!("Default memory save failed: {error}"))
            }
        }
    }
}

fn picker_selected_index(
    items: &[PickerItem],
    selected_id: Option<&str>,
    selected_index: usize,
) -> usize {
    if items.is_empty() {
        return 0;
    }

    if let Some(selected_id) = selected_id
        && let Some(index) = items.iter().position(|item| item.id == selected_id)
    {
        return index;
    }

    selected_index.min(items.len().saturating_sub(1))
}

fn apply_reloaded_preferences(
    user_preferences: &mut UserPreferences,
    preferences_health: &mut PreferencesHealth,
    updated_preferences: UserPreferences,
    reloaded_preferences: Result<UserPreferences, tui_kit_host::settings::SettingsError>,
) {
    *user_preferences = match reloaded_preferences {
        Ok(preferences) => {
            *preferences_health = PreferencesHealth::default();
            preferences
        }
        Err(error) => {
            *preferences_health = PreferencesHealth {
                load_error: Some(error.to_string()),
                save_error: None,
            };
            updated_preferences
        }
    };
}

fn reload_preferences_for_apply(
    _updated_preferences: &UserPreferences,
) -> Result<UserPreferences, tui_kit_host::settings::SettingsError> {
    #[cfg(test)]
    {
        Ok(_updated_preferences.clone())
    }

    #[cfg(not(test))]
    {
        preferences::load_user_preferences()
    }
}

fn save_user_preferences_for_apply(
    preferences: &UserPreferences,
) -> Result<(), tui_kit_host::settings::SettingsError> {
    #[cfg(test)]
    if take_test_settings_save_override().is_some() {
        return Err(tui_kit_host::settings::SettingsError::NoConfigDir);
    }

    preferences::save_user_preferences(preferences)
}

#[cfg(test)]
fn settings_io_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
fn test_settings_save_override() -> &'static Mutex<Option<thread::ThreadId>> {
    static OVERRIDE: OnceLock<Mutex<Option<thread::ThreadId>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_next_test_settings_save_to_fail() {
    let mut guard = test_settings_save_override()
        .lock()
        .expect("test settings save override lock should be available");
    *guard = Some(thread::current().id());
}

#[cfg(test)]
fn take_test_settings_save_override() -> Option<()> {
    let mut guard = test_settings_save_override()
        .lock()
        .expect("test settings save override lock should be available");
    if guard
        .as_ref()
        .is_some_and(|thread_id| *thread_id == thread::current().id())
    {
        *guard = None;
        Some(())
    } else {
        None
    }
}

#[cfg(test)]
fn test_chat_submit_override() -> &'static Mutex<Option<Result<bridge::AskMemoriesOutput, String>>>
{
    static OVERRIDE: OnceLock<Mutex<Option<Result<bridge::AskMemoriesOutput, String>>>> =
        OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_next_test_chat_submit_result(result: Result<bridge::AskMemoriesOutput, String>) {
    let mut guard = test_chat_submit_override()
        .lock()
        .expect("test chat submit override lock should be available");
    *guard = Some(result);
}

#[cfg(test)]
fn take_test_chat_submit_result() -> Option<Result<bridge::AskMemoriesOutput, String>> {
    let mut guard = test_chat_submit_override()
        .lock()
        .expect("test chat submit override lock should be available");
    guard.take()
}

#[cfg(test)]
fn test_last_chat_request() -> &'static Mutex<Option<CapturedChatRequest>> {
    static LAST_REQUEST: OnceLock<Mutex<Option<CapturedChatRequest>>> = OnceLock::new();
    LAST_REQUEST.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_last_test_chat_request(request: CapturedChatRequest) {
    let mut guard = test_last_chat_request()
        .lock()
        .expect("last chat request lock should be available");
    *guard = Some(request);
}

#[cfg(test)]
fn take_last_test_chat_request() -> Option<CapturedChatRequest> {
    let mut guard = test_last_chat_request()
        .lock()
        .expect("last chat request lock should be available");
    guard.take()
}

#[cfg(test)]
fn test_memory_summary_override()
-> &'static Mutex<Option<Result<bridge::AskMemoriesOutput, String>>> {
    static OVERRIDE: OnceLock<Mutex<Option<Result<bridge::AskMemoriesOutput, String>>>> =
        OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_next_test_memory_summary_result(result: Result<bridge::AskMemoriesOutput, String>) {
    let mut guard = test_memory_summary_override()
        .lock()
        .expect("test memory summary override lock should be available");
    *guard = Some(result);
}

#[cfg(test)]
fn take_test_memory_summary_result() -> Option<Result<bridge::AskMemoriesOutput, String>> {
    let mut guard = test_memory_summary_override()
        .lock()
        .expect("test memory summary override lock should be available");
    guard.take()
}

struct InsertSubmitTaskOutput {
    request_id: u64,
    result: Result<bridge::InsertMemorySuccess, bridge::InsertMemoryError>,
}

fn insert_success_status(success: &bridge::InsertMemorySuccess) -> String {
    match &success.source_name {
        Some(source_name) => format!(
            "Inserted {} chunks from {} (tag: {}) into {}",
            success.inserted_count, source_name, success.tag, success.memory_id
        ),
        None => format!(
            "Inserted {} chunks (tag: {}) into {}",
            success.inserted_count, success.tag, success.memory_id
        ),
    }
}

struct InsertDimTaskOutput {
    memory_id: String,
    result: Result<u64, bridge::InsertMemoryError>,
}

struct MemoryDetailTaskOutput {
    request_id: u64,
    memory_id: String,
    result: Result<bridge::MemoryDetails, String>,
}

struct PrefetchMemoryDetailTaskOutput {
    memory_id: String,
    result: Result<bridge::MemoryDetails, String>,
}

struct AccessSubmitTaskOutput {
    memory_id: String,
    result: Result<(), String>,
}

struct AddMemoryValidationTaskOutput {
    memory_id: String,
    result: Result<String, String>,
}

struct RenameSubmitTaskOutput {
    memory_id: String,
    next_name: String,
    stored_name: Option<String>,
    result: Result<(), bridge::RenameMemoryError>,
}

struct RequestTaskState<T> {
    receiver: Option<mpsc::Receiver<T>>,
    in_flight: bool,
    request_id: Option<u64>,
}

impl<T> Default for RequestTaskState<T> {
    fn default() -> Self {
        Self {
            receiver: None,
            in_flight: false,
            request_id: None,
        }
    }
}

impl<T> RequestTaskState<T> {
    fn reset(&mut self) {
        self.receiver = None;
        self.in_flight = false;
        self.request_id = None;
    }

    fn finish(&mut self, output_request_id: u64) -> bool {
        self.receiver = None;
        self.in_flight = false;
        let is_current = self.request_id == Some(output_request_id);
        self.request_id = None;
        is_current
    }
}

struct TaskState<T> {
    receiver: Option<mpsc::Receiver<T>>,
    in_flight: bool,
}

#[derive(Default)]
struct MemoryDetailPrefetchState {
    sender: Option<mpsc::Sender<PrefetchMemoryDetailTaskOutput>>,
    receiver: Option<mpsc::Receiver<PrefetchMemoryDetailTaskOutput>>,
    queued_memory_ids: VecDeque<String>,
    in_flight_memory_ids: HashSet<String>,
}

impl MemoryDetailPrefetchState {
    fn reset(&mut self) {
        self.sender = None;
        self.receiver = None;
        self.queued_memory_ids.clear();
        self.in_flight_memory_ids.clear();
    }
}

impl<T> Default for TaskState<T> {
    fn default() -> Self {
        Self {
            receiver: None,
            in_flight: false,
        }
    }
}

impl<T> TaskState<T> {
    fn reset(&mut self) {
        self.receiver = None;
        self.in_flight = false;
    }
}

enum PendingTaskPoll<T> {
    Pending,
    Ready(T),
    Disconnected,
}

fn poll_pending_task<T>(receiver: &mpsc::Receiver<T>) -> PendingTaskPoll<T> {
    match receiver.try_recv() {
        Ok(output) => PendingTaskPoll::Ready(output),
        Err(mpsc::TryRecvError::Empty) => PendingTaskPoll::Pending,
        Err(mpsc::TryRecvError::Disconnected) => PendingTaskPoll::Disconnected,
    }
}

fn spawn_request_task<T, F>(
    next_request_id: &mut u64,
    task_state: &mut RequestTaskState<T>,
    worker: F,
) -> u64
where
    T: Send + 'static,
    F: FnOnce(u64, mpsc::Sender<T>) + Send + 'static,
{
    let request_id = *next_request_id;
    *next_request_id += 1;
    task_state.request_id = Some(request_id);
    task_state.in_flight = true;
    let (tx, rx) = mpsc::channel();
    task_state.receiver = Some(rx);
    thread::spawn(move || worker(request_id, tx));
    request_id
}

fn finish_request_task<T>(task_state: &mut RequestTaskState<T>, output_request_id: u64) -> bool {
    task_state.finish(output_request_id)
}

fn reset_request_task<T>(task_state: &mut RequestTaskState<T>) {
    task_state.reset();
}

fn spawn_task<T, F>(task_state: &mut TaskState<T>, worker: F)
where
    T: Send + 'static,
    F: FnOnce(mpsc::Sender<T>) + Send + 'static,
{
    task_state.in_flight = true;
    let (tx, rx) = mpsc::channel();
    task_state.receiver = Some(rx);
    thread::spawn(move || worker(tx));
}

fn finish_task<T>(task_state: &mut TaskState<T>) {
    task_state.reset();
}

#[cfg(test)]
type TestMemoryDetailResult = (String, Result<bridge::MemoryDetails, String>);

#[cfg(test)]
type TestMemoryDetailResults = Vec<TestMemoryDetailResult>;

#[cfg(test)]
fn test_memory_detail_results() -> &'static Mutex<TestMemoryDetailResults> {
    static RESULTS: OnceLock<Mutex<TestMemoryDetailResults>> = OnceLock::new();
    RESULTS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(test)]
fn push_test_memory_detail_result(memory_id: &str, result: Result<bridge::MemoryDetails, String>) {
    test_memory_detail_results()
        .lock()
        .expect("memory detail test results lock should be available")
        .push((memory_id.to_string(), result));
}

fn load_memory_details_task_result(
    use_mainnet: bool,
    auth: TuiAuth,
    memory_id: String,
) -> Result<bridge::MemoryDetails, String> {
    #[cfg(test)]
    {
        let mut guard = test_memory_detail_results()
            .lock()
            .expect("memory detail test results lock should be available");
        if let Some(index) = guard
            .iter()
            .position(|(candidate_id, _)| candidate_id == &memory_id)
        {
            let (_, result) = guard.remove(index);
            return result;
        }
    }

    let runtime = Runtime::new().expect("failed to create tokio runtime for memory detail load");
    runtime
        .block_on(bridge::load_memory_details(use_mainnet, auth, memory_id))
        .map_err(|error| error.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MemoryContentSelection<'a> {
    RenameMemory,
    User(&'a bridge::MemoryUser),
    AddUser,
    RemoveManualMemory,
}

impl KinicProvider {
    pub fn new(config: TuiConfig) -> Self {
        #[cfg(test)]
        let _settings_io_lock = settings_io_lock()
            .lock()
            .expect("settings io lock should be available");
        let (user_preferences, preferences_health) = match preferences::load_user_preferences() {
            Ok(preferences) => (preferences, PreferencesHealth::default()),
            Err(error) => (
                UserPreferences::default(),
                PreferencesHealth {
                    load_error: Some(error.to_string()),
                    save_error: None,
                },
            ),
        };
        let session_overview = SessionAccountOverview::new(settings::session_settings_snapshot(
            &config.auth,
            config.use_mainnet,
            None,
            crate::embedding::embedding_base_url(),
        ));

        Self {
            all: Vec::new(),
            query: String::new(),
            tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            config,
            session_overview,
            user_preferences,
            preferences_health,
            active_memory: None,
            memory_summaries: Vec::new(),
            memory_records: Vec::new(),
            result_records: Vec::new(),
            memories_mode: MemoriesMode::Browser,
            pending_initial_memories: None,
            initial_memories_in_flight: false,
            pending_search: None,
            next_search_request_id: 0,
            last_search_state: None,
            create_cost_state: CreateCostState::Hidden,
            create_cost_task: RequestTaskState::default(),
            create_submit_task: RequestTaskState::default(),
            chat_submit_task: RequestTaskState::default(),
            active_chat_thread: None,
            session_settings_task: RequestTaskState::default(),
            next_session_settings_request_id: 0,
            next_create_request_id: 0,
            next_chat_request_id: 0,
            insert_submit_task: RequestTaskState::default(),
            next_insert_request_id: 0,
            insert_expected_dim_memory_id: None,
            insert_expected_dim: None,
            insert_expected_dim_loading: false,
            insert_expected_dim_load_error: None,
            pending_insert_dim: None,
            pending_access_submit: None,
            access_submit_in_flight: false,
            add_memory_validation_task: TaskState::default(),
            rename_submit_task: TaskState::default(),
            transfer_prerequisites_task: TaskState::default(),
            transfer_submit_task: TaskState::default(),
            preferred_memory_after_refresh: None,
            memory_content_summaries: HashMap::new(),
            failed_memory_content_summaries: HashMap::new(),
            memory_summary_tasks: HashMap::new(),
            next_memory_summary_request_id: 0,
            loaded_memory_details: HashSet::new(),
            pending_memory_detail: RequestTaskState::default(),
            next_memory_detail_request_id: 0,
            pending_memory_detail_memory_id: None,
            memory_detail_prefetch: MemoryDetailPrefetchState::default(),
        }
    }

    fn initialize_live_memories(&mut self) {
        let _ = self.start_live_memories_load(None, false);
    }

    fn start_live_memories_load(
        &mut self,
        notify_message: Option<&str>,
        preserve_query: bool,
    ) -> Option<CoreEffect> {
        if self.initial_memories_in_flight {
            return None;
        }

        self.memories_mode = MemoriesMode::Browser;
        if !preserve_query {
            self.query.clear();
        }
        self.result_records.clear();
        self.invalidate_pending_search();

        self.all = vec![loading_memories_record()];
        self.memory_summaries.clear();
        self.memory_records.clear();
        self.memory_content_summaries.clear();
        self.failed_memory_content_summaries.clear();
        self.memory_summary_tasks.clear();
        self.loaded_memory_details.clear();
        reset_request_task(&mut self.pending_memory_detail);
        self.pending_memory_detail_memory_id = None;
        self.memory_detail_prefetch.reset();
        self.clear_active_memory();
        self.active_chat_thread = None;
        self.initial_memories_in_flight = true;
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let (tx, rx) = mpsc::channel();
        self.pending_initial_memories = Some(rx);

        thread::spawn(move || {
            let runtime =
                Runtime::new().expect("failed to create tokio runtime for initial memories load");
            let result = runtime
                .block_on(bridge::list_memories(use_mainnet, auth))
                .map_err(|error| error.to_string());
            let _ = tx.send(InitialMemoriesTaskOutput { result });
        });

        notify_message.map(|message| CoreEffect::Notify(message.to_string()))
    }

    fn is_memories_load_error_visible(&self) -> bool {
        self.memory_records.is_empty()
            && self.all.len() == 1
            && self.all[0].id == "kinic-live-error"
    }

    fn refresh_current_view(&mut self) -> Vec<CoreEffect> {
        match self.tab_id.as_str() {
            KINIC_CREATE_TAB_ID => self.start_create_cost_refresh().into_iter().collect(),
            KINIC_INSERT_TAB_ID => Vec::new(),
            KINIC_MEMORIES_TAB_ID => self
                .start_live_memories_load(None, true)
                .into_iter()
                .collect(),
            KINIC_SETTINGS_TAB_ID => self.start_session_settings_refresh().into_iter().collect(),
            _ => Vec::new(),
        }
    }

    fn current_records(&self) -> Vec<&KinicRecord> {
        if self.memories_mode == MemoriesMode::Browser && self.memory_records.is_empty() {
            return self.all.iter().collect();
        }

        let base = match self.memories_mode {
            MemoriesMode::Browser => &self.memory_records,
            MemoriesMode::Results => &self.result_records,
        };

        base.iter().collect()
    }

    fn visible_memory_records(&self) -> Vec<&KinicRecord> {
        if self.memories_mode != MemoriesMode::Browser {
            return Vec::new();
        }
        if self.memory_records.is_empty() {
            return Vec::new();
        }
        self.current_records()
    }

    fn sync_memory_browser_selection(&mut self) -> bool {
        if self.memories_mode != MemoriesMode::Browser {
            return false;
        }
        let previous_active_memory = self.active_memory.clone();

        let visible_ids = self
            .visible_memory_records()
            .into_iter()
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();

        if visible_ids.is_empty() {
            self.clear_active_memory();
            return self.active_memory != previous_active_memory;
        }

        if !self
            .active_memory_id()
            .is_some_and(|memory_id| visible_ids.iter().any(|id| id == memory_id))
            && let Some(memory_id) = visible_ids.first()
        {
            self.set_active_memory_by_id(memory_id.clone());
        }

        self.active_memory != previous_active_memory
    }

    /// Aligns provider active memory with [`CoreState::selected_index`] for browser list interactions.
    /// `all memories` chat still keeps the browser selection so detail/search/default-memory
    /// actions keep pointing at the visible list selection. In [`MemoriesMode::Results`], list
    /// selection follows a different path; chat scope sync is intentionally browser-first here.
    fn sync_active_memory_to_chat_scope_for_browser(&mut self, state: &CoreState) {
        if state.current_tab_id.as_str() != KINIC_MEMORIES_TAB_ID {
            return;
        }
        if self.memories_mode != MemoriesMode::Browser {
            return;
        }
        if let Some(id) = state
            .selected_index
            .and_then(|idx| state.list_items.get(idx))
            .filter(|item| {
                matches!(
                    &item.kind,
                    tui_kit_model::UiItemKind::Custom(kind) if kind == "memory"
                )
            })
            .map(|item| item.id.clone())
        {
            self.set_active_memory_by_id(id);
        }
    }

    fn active_memory_visible_index(&self) -> Option<usize> {
        let active_id = self.active_memory_id()?;
        self.visible_memory_records()
            .into_iter()
            .position(|record| record.id == active_id)
    }

    fn navigate_active_memory(&mut self, state: &CoreState, action: &CoreAction) {
        if !self.should_handle_memory_navigation(state) || self.is_add_memory_action_selected(state)
        {
            return;
        }

        let visible_records = self.visible_memory_records();
        if visible_records.is_empty() {
            return;
        }

        let target_index = if let Some(index) = state.selected_index
            && visible_records.get(index).is_some()
        {
            index
        } else {
            let visible_count = visible_records.len();
            let current = self.active_memory_visible_index().unwrap_or(0);
            let last = visible_count.saturating_sub(1);
            match action {
                CoreAction::MoveNext => {
                    if visible_count == 1 {
                        0
                    } else {
                        (current + 1) % visible_count
                    }
                }
                CoreAction::MovePrev => {
                    if visible_count == 1 {
                        0
                    } else if current == 0 {
                        last
                    } else {
                        current - 1
                    }
                }
                CoreAction::MoveHome => 0,
                CoreAction::MoveEnd => last,
                CoreAction::MovePageDown => (current + 10).min(last),
                CoreAction::MovePageUp => current.saturating_sub(10),
                _ => current,
            }
        };

        let Some(record) = visible_records.get(target_index) else {
            return;
        };
        self.set_active_memory_by_id(record.id.clone());
    }

    fn network_label(&self) -> &str {
        self.session_overview.session.network.as_str()
    }

    fn chat_history_principal_id(&self) -> Result<String, String> {
        self.config
            .auth
            .principal_text()
            .or_else(|_| {
                let principal_id = self.session_overview.session.principal_id.as_str();
                if principal_id.is_empty() || principal_id == "unavailable" {
                    Err(anyhow::anyhow!(
                        "Could not determine principal for chat history"
                    ))
                } else {
                    Ok(principal_id.to_string())
                }
            })
            .map_err(|error| error.to_string())
    }

    fn load_chat_history_messages(
        &mut self,
        history_thread_key: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let network = self.network_label().to_string();
        let principal_id = self.chat_history_principal_id()?;
        let identity_label = self.config.auth.identity_label().to_string();
        let active_thread = self.with_settings_io(move || {
            settings::load_or_create_active_chat_thread(
                network.as_str(),
                principal_id.as_str(),
                identity_label.as_str(),
                history_thread_key,
            )
        })?;
        self.active_chat_thread = Some(ActiveChatThreadRef {
            thread_key: history_thread_key.to_string(),
            thread_id: active_thread.thread_id,
        });
        Ok(active_thread.messages)
    }

    fn create_chat_thread(&mut self, history_thread_key: &str) -> Result<String, String> {
        let network = self.network_label().to_string();
        let principal_id = self.chat_history_principal_id()?;
        let identity_label = self.config.auth.identity_label().to_string();
        let thread_id = self.with_settings_io(move || {
            settings::create_chat_thread(
                network.as_str(),
                principal_id.as_str(),
                identity_label.as_str(),
                history_thread_key,
            )
        })?;
        self.active_chat_thread = Some(ActiveChatThreadRef {
            thread_key: history_thread_key.to_string(),
            thread_id: thread_id.clone(),
        });
        Ok(thread_id)
    }

    fn ensure_chat_thread_id(&mut self, history_thread_key: &str) -> Result<String, String> {
        if let Some(thread) = self.current_chat_thread(history_thread_key) {
            return Ok(thread.thread_id.clone());
        }
        let network = self.network_label().to_string();
        let principal_id = self.chat_history_principal_id()?;
        let identity_label = self.config.auth.identity_label().to_string();
        let active_thread = self.with_settings_io(move || {
            settings::load_or_create_active_chat_thread(
                network.as_str(),
                principal_id.as_str(),
                identity_label.as_str(),
                history_thread_key,
            )
        })?;
        let thread_id = active_thread.thread_id;
        self.active_chat_thread = Some(ActiveChatThreadRef {
            thread_key: history_thread_key.to_string(),
            thread_id: thread_id.clone(),
        });
        Ok(thread_id)
    }

    fn append_chat_history_message(
        &self,
        history_thread_key: &str,
        thread_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), String> {
        let network = self.network_label().to_string();
        let principal_id = self.chat_history_principal_id()?;
        let identity_label = self.config.auth.identity_label().to_string();
        let saved_at = settings::current_chat_saved_at();
        self.with_settings_io(move || {
            settings::append_chat_history_message(
                network.as_str(),
                principal_id.as_str(),
                identity_label.as_str(),
                history_thread_key,
                thread_id,
                role,
                content,
                saved_at,
            )
        })
    }

    fn with_settings_io<T, F>(&self, operation: F) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, tui_kit_host::settings::SettingsError>,
    {
        #[cfg(test)]
        let _settings_io_lock = settings_io_lock()
            .lock()
            .expect("settings io lock should be available");
        operation().map_err(|error| error.to_string())
    }

    fn chat_history_thread_key(&self, state: &CoreState) -> Option<String> {
        match state.chat_scope {
            ChatScope::All => Some(ALL_MEMORIES_CHAT_THREAD_KEY.to_string()),
            ChatScope::Selected => self.active_chat_scope_memory_id(state),
        }
    }

    fn current_chat_thread(&self, history_thread_key: &str) -> Option<&ActiveChatThreadRef> {
        self.active_chat_thread
            .as_ref()
            .filter(|thread| thread.thread_key == history_thread_key)
    }

    fn load_active_chat_history_effects(&mut self, state: &CoreState) -> Vec<CoreEffect> {
        let Some(history_thread_key) = self.chat_history_thread_key(state) else {
            self.active_chat_thread = None;
            return vec![
                CoreEffect::ReplaceChatMessages(Vec::new()),
                CoreEffect::SetChatLoading(false),
            ];
        };

        match self.load_chat_history_messages(history_thread_key.as_str()) {
            Ok(messages) => vec![
                CoreEffect::ReplaceChatMessages(messages),
                CoreEffect::SetChatLoading(false),
            ],
            Err(error) => vec![
                {
                    self.active_chat_thread = None;
                    CoreEffect::ReplaceChatMessages(Vec::new())
                },
                CoreEffect::SetChatLoading(false),
                CoreEffect::Notify(format!("Chat history load failed: {error}")),
            ],
        }
    }

    fn active_chat_scope_memory_id(&self, state: &CoreState) -> Option<String> {
        match state.chat_scope {
            ChatScope::All => None,
            ChatScope::Selected => self.active_memory_id().map(str::to_string),
        }
    }

    fn chat_scope_label(&self, state: &CoreState) -> Option<String> {
        let memory_id = self.active_chat_scope_memory_id(state)?;
        if self.active_memory_id() == Some(memory_id.as_str())
            && let Some(label) = self.active_memory_label().map(str::trim)
            && !label.is_empty()
        {
            return Some(label.to_string());
        }
        self.memory_records
            .iter()
            .find(|record| record.id == memory_id)
            .map(|record| {
                let title = record.title.trim();
                if title.is_empty() {
                    record.id.clone()
                } else {
                    title.to_string()
                }
            })
            .or(Some(memory_id))
    }

    fn chat_targets(&self, state: &CoreState) -> Result<Vec<bridge::ChatTarget>, String> {
        match state.chat_scope {
            ChatScope::All => {
                let targets =
                    self.memory_records
                        .iter()
                        .filter_map(|record| {
                            record.searchable_memory_id.as_ref().map(|memory_id| {
                                bridge::ChatTarget {
                                    memory_id: memory_id.clone(),
                                    memory_name: record.title.clone(),
                                }
                            })
                        })
                        .collect::<Vec<_>>();
                if targets.is_empty() {
                    Err("No searchable memories are available yet.".to_string())
                } else {
                    Ok(targets)
                }
            }
            ChatScope::Selected => {
                let active_memory_id = self
                    .active_chat_scope_memory_id(state)
                    .ok_or_else(|| "Select a memory before asking AI.".to_string())?;
                let record = self
                    .memory_records
                    .iter()
                    .find(|record| record.id == active_memory_id)
                    .ok_or_else(|| "Select a memory before asking AI.".to_string())?;
                let memory_id = record
                    .searchable_memory_id
                    .as_ref()
                    .ok_or_else(|| "The selected memory cannot be searched yet.".to_string())?;
                Ok(vec![bridge::ChatTarget {
                    memory_id: memory_id.clone(),
                    memory_name: record.title.clone(),
                }])
            }
        }
    }

    fn chat_retrieval_config(&self) -> bridge::ChatRetrievalConfig {
        bridge::ChatRetrievalConfig {
            overall_top_k: self.user_preferences.chat_overall_top_k,
            per_memory_cap: self.user_preferences.chat_per_memory_cap,
            mmr_lambda: f32::from(self.user_preferences.chat_mmr_lambda) / 100.0,
        }
    }

    fn active_memory_chat_context(&self, state: &CoreState) -> Option<ActiveMemoryContext> {
        if state.chat_scope != ChatScope::Selected {
            return None;
        }

        let active_memory_id = self.active_chat_scope_memory_id(state)?;
        let record = self
            .memory_records
            .iter()
            .find(|record| record.id == active_memory_id)?;
        let summary = self
            .memory_summaries
            .iter()
            .find(|summary| summary.id == active_memory_id)?;
        let parsed_detail = parse_memory_detail(summary.detail.as_str());
        let summary_text = self
            .memory_content_summaries
            .get(record.id.as_str())
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        Some(ActiveMemoryContext {
            memory_id: record.id.clone(),
            memory_name: resolved_memory_name(summary.name.as_str(), summary.detail.as_str()),
            description: parsed_detail.description,
            summary: summary_text,
        })
    }

    fn start_chat_submit(
        &mut self,
        state: &CoreState,
        history_thread_key: String,
        thread_id: String,
        targets: Vec<bridge::ChatTarget>,
        query: String,
        history: Vec<(String, String)>,
    ) -> CoreEffect {
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let scope = state.chat_scope;
        let retrieval_config = self.chat_retrieval_config();
        let active_memory_context = self.active_memory_chat_context(state);

        #[cfg(test)]
        set_last_test_chat_request(CapturedChatRequest {
            history_thread_key: history_thread_key.clone(),
            thread_id: thread_id.clone(),
            scope: state.chat_scope,
            target_memory_ids: targets
                .iter()
                .map(|target| target.memory_id.clone())
                .collect(),
            query: query.clone(),
            history: history.clone(),
            active_memory_context: active_memory_context.clone(),
        });

        spawn_request_task(
            &mut self.next_chat_request_id,
            &mut self.chat_submit_task,
            move |request_id, tx| {
                #[cfg(test)]
                if let Some(result) = take_test_chat_submit_result() {
                    let _ = tx.send(ChatTaskOutput {
                        request_id,
                        history_thread_key: history_thread_key.clone(),
                        thread_id: thread_id.clone(),
                        result,
                    });
                    return;
                }

                let runtime =
                    Runtime::new().expect("failed to create tokio runtime for chat submit");
                let result = runtime
                    .block_on(bridge::ask_memories(
                        use_mainnet,
                        auth,
                        bridge::AskMemoriesRequest {
                            scope,
                            targets,
                            query,
                            history,
                            retrieval_config,
                            active_memory_context,
                        },
                    ))
                    .map_err(|error| short_error(&error.to_string()));
                let _ = tx.send(ChatTaskOutput {
                    request_id,
                    history_thread_key,
                    thread_id,
                    result,
                });
            },
        );

        CoreEffect::Notify("Asking AI...".to_string())
    }

    fn prompt_history_messages(&self, state: &CoreState) -> Vec<(String, String)> {
        let mut recent = state
            .chat_messages
            .iter()
            .filter(|(role, content)| {
                matches!(role.as_str(), "user" | "assistant") && !content.trim().is_empty()
            })
            .cloned()
            .collect::<Vec<_>>();
        if recent
            .last()
            .is_some_and(|(role, _)| role.as_str() == "user")
        {
            recent.pop();
        }
        let start = recent.len().saturating_sub(8);
        recent.into_iter().skip(start).collect()
    }

    fn snapshot_output(&self, state: &CoreState, effects: Vec<CoreEffect>) -> ProviderOutput {
        ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        }
    }

    fn stale_request_output(&self, state: &CoreState) -> ProviderOutput {
        self.snapshot_output(state, Vec::new())
    }

    fn disconnected_request_output(&self, state: &CoreState, effect: CoreEffect) -> ProviderOutput {
        self.snapshot_output(state, vec![effect])
    }

    fn disconnected_task_output(&self, state: &CoreState, effect: CoreEffect) -> ProviderOutput {
        self.snapshot_output(state, vec![effect])
    }

    fn active_memory_summary(&self) -> Option<&MemorySummary> {
        let active_id = self.active_memory_id()?;
        self.memory_summaries
            .iter()
            .find(|summary| summary.id == active_id)
    }

    fn active_memory_record(&self) -> Option<&KinicRecord> {
        let selected_id = self.active_memory_id()?;
        self.memory_records
            .iter()
            .find(|record| record.id == selected_id)
    }

    fn active_memory_id(&self) -> Option<&str> {
        self.active_memory
            .as_ref()
            .map(|selection| selection.id.as_str())
    }

    fn active_memory_label(&self) -> Option<&str> {
        self.active_memory
            .as_ref()
            .map(|selection| selection.label.as_str())
    }

    fn memory_selection_for_id(&self, id: &str) -> MemorySelection {
        let label = self
            .memory_records
            .iter()
            .find(|record| record.id == id)
            .map(|record| record.title.trim())
            .filter(|title| !title.is_empty())
            .map(str::to_string)
            .or_else(|| self.default_memory_selection().title_for_id(id))
            .unwrap_or_else(|| id.to_string());

        MemorySelection {
            id: id.to_string(),
            label,
        }
    }

    fn set_active_memory_by_id(&mut self, id: impl Into<String>) -> bool {
        let id = id.into();
        self.set_active_memory(self.memory_selection_for_id(id.as_str()))
    }

    fn set_active_memory(&mut self, selection: MemorySelection) -> bool {
        if self.active_memory.as_ref() == Some(&selection) {
            return false;
        }
        self.active_memory = Some(selection);
        true
    }

    fn clear_active_memory(&mut self) -> bool {
        if self.active_memory.is_none() {
            return false;
        }
        self.active_memory = None;
        true
    }

    fn refresh_active_memory_label(&mut self, memory_id: &str) -> bool {
        if self.active_memory_id() != Some(memory_id) {
            return false;
        }
        self.set_active_memory_by_id(memory_id.to_string())
    }

    fn memory_summary_display_text(&self, memory_id: &str) -> Option<String> {
        if let Some(summary) = self.memory_content_summaries.get(memory_id) {
            return Some(summary.clone());
        }
        if self
            .memory_summary_tasks
            .get(memory_id)
            .is_some_and(|task| task.in_flight)
        {
            return Some(MEMORY_SUMMARY_LOADING_TEXT.to_string());
        }
        if self.failed_memory_content_summaries.contains_key(memory_id) {
            return Some(MEMORY_SUMMARY_UNAVAILABLE_TEXT.to_string());
        }
        None
    }

    fn invalidate_memory_summary(&mut self, memory_id: &str) {
        self.memory_content_summaries.remove(memory_id);
        self.failed_memory_content_summaries.remove(memory_id);
        self.memory_summary_tasks.remove(memory_id);
    }

    fn selected_content_for_record(
        &self,
        record: &KinicRecord,
        state: &CoreState,
    ) -> tui_kit_model::UiItemContent {
        let summary_text = if record.group == "memories" {
            self.memory_summary_display_text(record.id.as_str())
        } else {
            None
        };
        let mut content = adapter::to_content(record, summary_text.as_deref());
        if record.group == "memories" {
            self.apply_access_content(&mut content, state);
        }
        content
    }

    fn active_rename_target(&self, state: &CoreState) -> Option<(&MemorySummary, String)> {
        if self.tab_id != KINIC_MEMORIES_TAB_ID || self.memories_mode != MemoriesMode::Browser {
            return None;
        }
        if self.is_add_memory_action_selected(state) {
            return None;
        }
        let summary = self.active_memory_summary()?;
        summary.searchable_memory_id.as_ref().map(|_| {
            (
                summary,
                resolved_memory_name(summary.name.as_str(), summary.detail.as_str()),
            )
        })
    }

    fn active_memory_is_manual(&self) -> bool {
        let Some(active_id) = self.active_memory_id() else {
            return false;
        };
        self.user_preferences
            .manual_memory_ids
            .iter()
            .any(|memory_id| memory_id == active_id)
    }

    fn memory_content_selections<'a>(
        &'a self,
        state: &CoreState,
    ) -> Vec<MemoryContentSelection<'a>> {
        let Some(summary) = self.active_memory_summary() else {
            return vec![MemoryContentSelection::AddUser];
        };
        let mut selections = Vec::new();
        if self.active_rename_target(state).is_some() {
            selections.push(MemoryContentSelection::RenameMemory);
        }
        if let Some(users) = summary.users.as_ref() {
            selections.extend(users.iter().map(MemoryContentSelection::User));
        }
        selections.push(MemoryContentSelection::AddUser);
        if self.active_memory_is_manual() {
            selections.push(MemoryContentSelection::RemoveManualMemory);
        }
        selections
    }

    fn content_selection<'a>(&'a self, state: &CoreState) -> Option<MemoryContentSelection<'a>> {
        let selections = self.memory_content_selections(state);
        let selected_index = state
            .memory_content_action_index
            .min(selections.len().saturating_sub(1));
        selections.get(selected_index).cloned()
    }

    fn next_content_index(&self, state: &CoreState, delta: isize) -> usize {
        let selectable_len = self.memory_content_selections(state).len().max(1);
        let current = state
            .memory_content_action_index
            .min(selectable_len.saturating_sub(1)) as isize;
        (current + delta).rem_euclid(selectable_len as isize) as usize
    }

    fn access_role_from_user(user: &bridge::MemoryUser) -> AccessControlRole {
        match user.role.as_str() {
            "admin" => AccessControlRole::Admin,
            "writer" => AccessControlRole::Writer,
            _ => AccessControlRole::Reader,
        }
    }

    fn apply_access_content(&self, content: &mut tui_kit_model::UiItemContent, state: &CoreState) {
        let current_selection = self.content_selection(state);
        for section in &mut content.sections {
            for row in &mut section.rows {
                let label = row.label.trim_start().to_string();
                let selected = section.heading == "Overview"
                    && label == "Name"
                    && matches!(
                        current_selection,
                        Some(MemoryContentSelection::RenameMemory)
                    );
                row.label = marker_label(selected, label.as_str());
            }
        }

        let Some(section) = content
            .sections
            .iter_mut()
            .find(|section| section.heading == "Access")
        else {
            return;
        };
        let users = self
            .active_memory_summary()
            .and_then(|summary| summary.users.as_ref());
        section.body_lines = render_access_lines(users, current_selection.as_ref());

        content
            .sections
            .retain(|section| section.heading != "Actions");
        if self.active_memory_is_manual() {
            content.sections.push(tui_kit_model::UiSection {
                heading: "Actions".to_string(),
                rows: Vec::new(),
                body_lines: vec![marker_line(
                    matches!(
                        current_selection,
                        Some(MemoryContentSelection::RemoveManualMemory)
                    ),
                    "Remove from list",
                )],
            });
        }
    }

    fn effective_insert_memory_id(&self) -> Option<String> {
        self.active_memory_id().map(str::to_string)
    }

    fn reset_insert_dim(&mut self) {
        self.insert_expected_dim_memory_id = None;
        self.insert_expected_dim = None;
        self.insert_expected_dim_loading = false;
        self.insert_expected_dim_load_error = None;
        self.pending_insert_dim = None;
    }

    fn start_insert_dim_load(&mut self) {
        let Some(memory_id) = self.effective_insert_memory_id() else {
            return;
        };
        if self.insert_expected_dim_memory_id.as_deref() == Some(memory_id.as_str())
            && (self.insert_expected_dim.is_some() || self.insert_expected_dim_loading)
        {
            return;
        }

        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let (tx, rx) = mpsc::channel();
        self.insert_expected_dim_memory_id = Some(memory_id.clone());
        self.insert_expected_dim = None;
        self.insert_expected_dim_load_error = None;
        self.pending_insert_dim = Some(rx);
        self.insert_expected_dim_loading = true;

        thread::spawn(move || {
            let runtime =
                Runtime::new().expect("failed to create tokio runtime for insert dim load");
            let requested_memory_id = memory_id.clone();
            let result = runtime.block_on(bridge::load_memory_dim(
                use_mainnet,
                auth,
                requested_memory_id,
            ));
            let _ = tx.send(InsertDimTaskOutput { memory_id, result });
        });
    }

    fn refresh_memory_records_from_summaries(&mut self) {
        self.normalize_memory_summaries();
        self.memory_records = self
            .memory_summaries
            .iter()
            .cloned()
            .map(record_from_memory_summary)
            .collect();
        self.all = self.memory_records.clone();
    }

    fn normalize_memory_summaries(&mut self) {
        let mut seen_ids = HashSet::new();
        self.memory_summaries
            .retain(|summary| seen_ids.insert(summary.id.clone()));

        for memory_id in &self.user_preferences.manual_memory_ids {
            if seen_ids.contains(memory_id) {
                continue;
            }
            self.memory_summaries
                .push(manual_memory_summary(memory_id.as_str()));
            seen_ids.insert(memory_id.clone());
        }
    }

    fn memory_detail_in_flight(&self, memory_id: &str) -> bool {
        self.pending_memory_detail_memory_id.as_deref() == Some(memory_id)
            || self
                .memory_detail_prefetch
                .in_flight_memory_ids
                .contains(memory_id)
    }

    fn ensure_memory_detail_prefetch_channel(&mut self) {
        if self.memory_detail_prefetch.sender.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.memory_detail_prefetch.sender = Some(tx);
        self.memory_detail_prefetch.receiver = Some(rx);
    }

    fn spawn_memory_detail_prefetch_worker(&mut self, memory_id: String) {
        let Some(tx) = self.memory_detail_prefetch.sender.clone() else {
            return;
        };
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        self.memory_detail_prefetch
            .in_flight_memory_ids
            .insert(memory_id.clone());
        thread::spawn(move || {
            let result = load_memory_details_task_result(use_mainnet, auth, memory_id.clone());
            let _ = tx.send(PrefetchMemoryDetailTaskOutput { memory_id, result });
        });
    }

    fn pump_memory_detail_prefetch_workers(&mut self) {
        while self.memory_detail_prefetch.in_flight_memory_ids.len()
            < MAX_CONCURRENT_MEMORY_DETAIL_PREFETCHES
        {
            let Some(memory_id) = self.memory_detail_prefetch.queued_memory_ids.pop_front() else {
                break;
            };
            if self.loaded_memory_details.contains(memory_id.as_str())
                || self.memory_detail_in_flight(memory_id.as_str())
            {
                continue;
            }
            self.spawn_memory_detail_prefetch_worker(memory_id);
        }
    }

    fn enqueue_memory_detail_prefetch(&mut self, memory_ids: impl IntoIterator<Item = String>) {
        self.ensure_memory_detail_prefetch_channel();
        for memory_id in memory_ids {
            if self.loaded_memory_details.contains(memory_id.as_str())
                || self.memory_detail_in_flight(memory_id.as_str())
                || self
                    .memory_detail_prefetch
                    .queued_memory_ids
                    .iter()
                    .any(|queued_memory_id| queued_memory_id == &memory_id)
            {
                continue;
            }
            self.memory_detail_prefetch
                .queued_memory_ids
                .push_back(memory_id);
        }
        self.pump_memory_detail_prefetch_workers();
    }

    fn start_memory_detail_prefetch_for_records(&mut self) {
        let memory_ids = self
            .memory_records
            .iter()
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();
        self.enqueue_memory_detail_prefetch(memory_ids);
    }

    fn apply_memory_detail_result(
        &mut self,
        memory_id: String,
        result: Result<bridge::MemoryDetails, String>,
        notify: bool,
    ) -> Vec<CoreEffect> {
        let mut effects = Vec::new();
        match result {
            Ok(details) => {
                if let Some(summary) = self
                    .memory_summaries
                    .iter_mut()
                    .find(|summary| summary.id == memory_id)
                {
                    summary.name = parse_memory_metadata(details.metadata_name.as_str())
                        .map(|_| details.metadata_name)
                        .unwrap_or(details.display_name);
                    summary.version = details.version;
                    summary.dim = details.dim;
                    summary.owners = Some(details.owners);
                    summary.stable_memory_size = details.stable_memory_size;
                    summary.cycle_amount = details.cycle_amount;
                    summary.users = Some(details.users);
                    self.loaded_memory_details.insert(memory_id.clone());
                    self.refresh_memory_records_from_summaries();
                    self.refresh_active_memory_label(memory_id.as_str());
                }
                if notify && let Some(message) = details.users_load_error.as_ref() {
                    effects.push(CoreEffect::Notify(format!(
                        "Could not load memory access list: {message}"
                    )));
                }
            }
            Err(message) => {
                if notify {
                    effects.push(CoreEffect::Notify(format!(
                        "Could not load memory details: {message}"
                    )));
                }
            }
        }
        effects
    }

    fn start_active_memory_detail_load(&mut self) {
        if self.memories_mode != MemoriesMode::Browser {
            return;
        }
        let Some(memory_id) = self.active_memory_id().map(str::to_string) else {
            return;
        };
        if self.loaded_memory_details.contains(memory_id.as_str()) {
            return;
        }
        if let Some(index) = self
            .memory_detail_prefetch
            .queued_memory_ids
            .iter()
            .position(|queued_memory_id| queued_memory_id == &memory_id)
        {
            self.memory_detail_prefetch.queued_memory_ids.remove(index);
        }
        if self
            .memory_detail_prefetch
            .in_flight_memory_ids
            .contains(memory_id.as_str())
        {
            return;
        }
        if self.pending_memory_detail_memory_id.as_deref() == Some(memory_id.as_str()) {
            return;
        }

        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        self.pending_memory_detail_memory_id = Some(memory_id.clone());
        spawn_request_task(
            &mut self.next_memory_detail_request_id,
            &mut self.pending_memory_detail,
            move |request_id, tx| {
                let result = load_memory_details_task_result(use_mainnet, auth, memory_id.clone());
                let _ = tx.send(MemoryDetailTaskOutput {
                    request_id,
                    memory_id,
                    result,
                });
            },
        );
    }

    fn start_selected_memory_summary_load(&mut self, force_reload: bool) {
        if self.memories_mode != MemoriesMode::Browser {
            return;
        }
        let Some(record) = self.active_memory_record() else {
            return;
        };
        #[cfg(test)]
        if record.searchable_memory_id.is_none() {
            return;
        }
        #[cfg(not(test))]
        let Some(searchable_memory_id) = record.searchable_memory_id.clone() else {
            return;
        };
        let memory_id = record.id.clone();
        if !force_reload {
            if self
                .memory_content_summaries
                .contains_key(memory_id.as_str())
            {
                return;
            }
            if self
                .failed_memory_content_summaries
                .contains_key(memory_id.as_str())
            {
                return;
            }
            if self
                .memory_summary_tasks
                .get(memory_id.as_str())
                .is_some_and(|task| task.in_flight)
            {
                return;
            }
        }

        #[cfg(test)]
        let Some(result) = take_test_memory_summary_result() else {
            return;
        };
        #[cfg(not(test))]
        let auth = self.config.auth.clone();
        #[cfg(not(test))]
        let use_mainnet = self.config.use_mainnet;
        #[cfg(not(test))]
        let retrieval_config = self.chat_retrieval_config();
        #[cfg(not(test))]
        let target = bridge::ChatTarget {
            memory_id: searchable_memory_id,
            memory_name: record.title.clone(),
        };

        self.failed_memory_content_summaries
            .remove(memory_id.as_str());
        let task_state = self
            .memory_summary_tasks
            .entry(memory_id.clone())
            .or_default();

        spawn_request_task(
            &mut self.next_memory_summary_request_id,
            task_state,
            move |request_id, tx| {
                #[cfg(test)]
                {
                    let _ = tx.send(MemorySummaryTaskOutput {
                        request_id,
                        memory_id,
                        result,
                    });
                }

                #[cfg(not(test))]
                let result = Runtime::new()
                    .expect("failed to create tokio runtime for memory summary")
                    .block_on(bridge::ask_memories(
                        use_mainnet,
                        auth,
                        bridge::AskMemoriesRequest {
                            scope: ChatScope::Selected,
                            targets: vec![target],
                            query: MEMORY_SUMMARY_QUERY.to_string(),
                            history: Vec::new(),
                            retrieval_config,
                            active_memory_context: None,
                        },
                    ))
                    .map_err(|error| short_error(&error.to_string()));

                #[cfg(not(test))]
                let _ = tx.send(MemorySummaryTaskOutput {
                    request_id,
                    memory_id,
                    result,
                });
            },
        );
    }

    fn should_handle_memory_navigation(&self, state: &CoreState) -> bool {
        state.current_tab_id == KINIC_MEMORIES_TAB_ID
            && self.tab_id == KINIC_MEMORIES_TAB_ID
            && self.memories_mode == MemoriesMode::Browser
    }

    fn is_add_memory_action_selected(&self, state: &CoreState) -> bool {
        self.should_show_add_memory_action(state)
            && state.selected_index == Some(self.current_records().len())
    }

    fn should_show_add_memory_action(&self, state: &CoreState) -> bool {
        state.current_tab_id == KINIC_MEMORIES_TAB_ID && self.memories_mode == MemoriesMode::Browser
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.current_records();
        let default_memory = self.default_memory_selection();
        let default_memory_items = default_memory.available_memory_ids();
        let default_memory_labels = default_memory.selector_labels();
        let insert_memory_placeholder = self.insert_memory_placeholder_label();
        let picker = match &state.picker {
            PickerState::Closed => PickerState::Closed,
            PickerState::Input {
                context,
                origin_context,
                value,
            } => PickerState::Input {
                context: *context,
                origin_context: *origin_context,
                value: value.clone(),
            },
            PickerState::List {
                context,
                selected_index,
                selected_id,
                mode,
                ..
            } => {
                let items = picker_items_for_context(
                    *context,
                    state,
                    default_memory,
                    &self.user_preferences,
                );
                let preferred_selected_id = selected_id.clone().or_else(|| {
                    picker_selected_id_for_context(
                        *context,
                        state,
                        self.active_memory.as_ref(),
                        &self.user_preferences,
                    )
                });
                let resolved_index = picker_selected_index(
                    &items,
                    preferred_selected_id.as_deref(),
                    *selected_index,
                );
                let resolved_selected_id =
                    items.get(resolved_index).and_then(|item| match item.kind {
                        PickerItemKind::Option => Some(item.id.clone()),
                        PickerItemKind::AddAction => None,
                    });
                let resolved_mode = match mode {
                    PickerListMode::Browsing => PickerListMode::Browsing,
                    PickerListMode::Confirm {
                        kind: PickerConfirmKind::DeleteTag { tag_id },
                    } if items.iter().any(|item| item.id == *tag_id) => PickerListMode::Confirm {
                        kind: PickerConfirmKind::DeleteTag {
                            tag_id: tag_id.clone(),
                        },
                    },
                    PickerListMode::Confirm { .. } => PickerListMode::Browsing,
                };
                PickerState::List {
                    context: *context,
                    items,
                    selected_index: resolved_index,
                    selected_id: resolved_selected_id,
                    mode: resolved_mode,
                }
            }
        };
        let mut items = filtered
            .iter()
            .map(|record| {
                let mut summary = adapter::to_summary(record);
                if record.group == "memories" && default_memory.is_default_memory(&record.id) {
                    summary.leading_marker = Some("★".to_string());
                }
                summary
            })
            .collect::<Vec<_>>();
        if self.should_show_add_memory_action(state) {
            items.push(adapter::to_summary(&add_memory_action_record()));
        }
        let selected_content = if state.current_tab_id == KINIC_SETTINGS_TAB_ID {
            None
        } else if self.memories_mode == MemoriesMode::Browser {
            if self.active_memory.is_none() && self.memory_records.is_empty() {
                filtered
                    .first()
                    .copied()
                    .map(|record| self.selected_content_for_record(record, state))
            } else {
                self.active_memory_record()
                    .map(|record| self.selected_content_for_record(record, state))
            }
        } else {
            let sel = state.selected_index.unwrap_or(0);
            filtered
                .get(sel)
                .map(|record| adapter::to_content(record, None))
        };
        let selected_index = if state.current_tab_id == KINIC_SETTINGS_TAB_ID {
            None
        } else if self.is_add_memory_action_selected(state) {
            Some(filtered.len())
        } else if self.memories_mode == MemoriesMode::Browser {
            self.active_memory_visible_index()
                .or_else(|| (!filtered.is_empty()).then_some(0))
        } else {
            let current = state.selected_index.unwrap_or(0);
            (!filtered.is_empty()).then_some(current.min(filtered.len().saturating_sub(1)))
        };
        let insert_current_dim = self.insert_current_dim(state);
        let insert_validation_message = self.insert_validation_message(state);

        ProviderSnapshot {
            items,
            selected_index,
            selected_content,
            selected_context: None,
            total_count: filtered.len(),
            status_message: Some(self.status_message(state, filtered.len())),
            selected_memory: self.active_memory.clone(),
            chat_scope_label: self.chat_scope_label(state),
            create_cost_state: self.create_cost_state.clone(),
            create_submit_state: state.create_submit_state.clone(),
            settings: settings::build_settings_snapshot(
                &self.session_overview,
                &self.user_preferences,
                &default_memory_items,
                &default_memory_labels,
                &self.preferences_health,
            ),
            picker,
            saved_default_memory_id: self.user_preferences.default_memory_id.clone(),
            insert_memory_placeholder,
            insert_expected_dim: self
                .effective_insert_memory_id()
                .filter(|memory_id| {
                    self.insert_expected_dim_memory_id.as_deref() == Some(memory_id.as_str())
                })
                .and(self.insert_expected_dim),
            insert_expected_dim_loading: self.insert_expected_dim_loading
                && self.effective_insert_memory_id().is_some_and(|memory_id| {
                    self.insert_expected_dim_memory_id.as_deref() == Some(memory_id.as_str())
                }),
            insert_current_dim,
            insert_validation_message,
        }
    }

    fn insert_current_dim(&self, state: &CoreState) -> Option<String> {
        if !matches!(state.insert_mode, InsertMode::ManualEmbedding)
            || state.insert_embedding.trim().is_empty()
        {
            return None;
        }

        Some(
            match parse_embedding_json(state.insert_embedding.as_str()) {
                Ok(values) => values.len().to_string(),
                Err(_) => "invalid".to_string(),
            },
        )
    }

    fn insert_validation_message(&self, state: &CoreState) -> Option<String> {
        if !matches!(state.insert_mode, InsertMode::ManualEmbedding)
            || state.insert_embedding.trim().is_empty()
        {
            return None;
        }

        let selected_memory_id = self.effective_insert_memory_id()?;
        if self.insert_expected_dim_memory_id.as_deref() != Some(selected_memory_id.as_str()) {
            return None;
        }
        if self.insert_expected_dim_loading {
            return Some("Loading expected embedding dimension for this memory…".to_string());
        }
        if let Some(message) = self.insert_expected_dim_load_error.as_ref() {
            return Some(format!("Could not load expected dimension: {message}"));
        }
        let expected_dim = self.insert_expected_dim?;
        let provided_dim = match parse_embedding_json(state.insert_embedding.as_str()) {
            Ok(values) => values.len() as u64,
            Err(_) => {
                return Some(
                    "Embedding must be a JSON array of floats, e.g. [0.1, 0.2]".to_string(),
                );
            }
        };

        (provided_dim != expected_dim).then(|| {
            format!(
                "Embedding dimension mismatch. Received {provided_dim} values, expected {expected_dim}."
            )
        })
    }

    fn build_snapshot_with_picker(
        &self,
        state: &CoreState,
        picker: PickerState,
    ) -> ProviderSnapshot {
        let mut snapshot = self.build_snapshot(state);
        snapshot.picker = picker;
        snapshot
    }

    fn insert_memory_placeholder_label(&self) -> Option<String> {
        if self.active_memory.is_some() {
            return None;
        }
        let default_memory_id = self.user_preferences.default_memory_id.as_deref()?;
        Some(
            self.default_memory_selection()
                .title_for_id(default_memory_id)
                .unwrap_or_else(|| default_memory_id.to_string()),
        )
    }

    fn default_memory_selection(&self) -> DefaultMemorySelection<'_> {
        DefaultMemorySelection {
            memory_records: &self.memory_records,
            user_preferences: &self.user_preferences,
        }
    }

    fn default_memory_controller(&mut self) -> DefaultMemoryController<'_> {
        DefaultMemoryController {
            user_preferences: &mut self.user_preferences,
            preferences_health: &mut self.preferences_health,
        }
    }

    fn save_tags_to_preferences(&mut self, tag: String) -> SaveTagOutcome {
        let normalized_tag = tag.trim().to_string();
        if normalized_tag.is_empty() {
            return SaveTagOutcome {
                result: SaveTagResult::Failed,
                effect: CoreEffect::Notify("Tag cannot be empty.".to_string()),
            };
        }

        let mut updated_preferences = self.user_preferences.clone();
        updated_preferences.saved_tags.push(normalized_tag.clone());
        updated_preferences.saved_tags = tag::normalize_saved_tags(updated_preferences.saved_tags);

        #[cfg(test)]
        let _settings_io_lock = settings_io_lock()
            .lock()
            .expect("settings io lock should be available");
        match save_user_preferences_for_apply(&updated_preferences) {
            Ok(()) => {
                let reloaded_preferences = reload_preferences_for_apply(&updated_preferences);
                apply_reloaded_preferences(
                    &mut self.user_preferences,
                    &mut self.preferences_health,
                    updated_preferences,
                    reloaded_preferences,
                );
                SaveTagOutcome {
                    result: SaveTagResult::Saved,
                    effect: CoreEffect::Notify(format!("Saved tag {normalized_tag}")),
                }
            }
            Err(error) => {
                self.preferences_health.save_error = Some(error.to_string());
                SaveTagOutcome {
                    result: SaveTagResult::Failed,
                    effect: CoreEffect::Notify(format!("Tag save failed: {error}")),
                }
            }
        }
    }

    fn delete_tag_from_preferences(&mut self, tag: &str) -> CoreEffect {
        let normalized_tag = tag.trim().to_string();
        if normalized_tag.is_empty() {
            return CoreEffect::Notify("Tag cannot be empty.".to_string());
        }

        let mut updated_preferences = self.user_preferences.clone();
        updated_preferences
            .saved_tags
            .retain(|saved| saved != &normalized_tag);
        updated_preferences.saved_tags = tag::normalize_saved_tags(updated_preferences.saved_tags);

        #[cfg(test)]
        let _settings_io_lock = settings_io_lock()
            .lock()
            .expect("settings io lock should be available");
        match save_user_preferences_for_apply(&updated_preferences) {
            Ok(()) => {
                let reloaded_preferences = reload_preferences_for_apply(&updated_preferences);
                apply_reloaded_preferences(
                    &mut self.user_preferences,
                    &mut self.preferences_health,
                    updated_preferences,
                    reloaded_preferences,
                );
                CoreEffect::Notify(format!("Deleted tag {normalized_tag}"))
            }
            Err(error) => {
                self.preferences_health.save_error = Some(error.to_string());
                CoreEffect::Notify(format!("Tag delete failed: {error}"))
            }
        }
    }

    fn picker_confirm_effect(
        &mut self,
        context: PickerContext,
        kind: &PickerConfirmKind,
    ) -> Option<CoreEffect> {
        match (context, kind) {
            (PickerContext::TagManagement, PickerConfirmKind::DeleteTag { tag_id }) => {
                Some(self.delete_tag_from_preferences(tag_id))
            }
            _ => None,
        }
    }

    fn picker_option_submit_effects(
        &mut self,
        context: PickerContext,
        item: &PickerItem,
    ) -> Vec<CoreEffect> {
        match context {
            PickerContext::DefaultMemory => {
                let selection = self.memory_selection_for_id(item.id.as_str());
                self.set_active_memory(selection);
                vec![
                    self.default_memory_controller()
                        .set_default_memory_preference(item.id.clone()),
                ]
            }
            PickerContext::InsertTarget => {
                let selection = MemorySelection {
                    id: item.id.clone(),
                    label: item.label.clone(),
                };
                self.set_active_memory(selection);
                vec![CoreEffect::Notify(format!(
                    "Selected target memory {}",
                    item.label
                ))]
            }
            PickerContext::InsertTag => Vec::new(),
            PickerContext::TagManagement => vec![
                CoreEffect::SetInsertTag(item.id.clone()),
                CoreEffect::Notify(format!("Selected tag {} for insert", item.id)),
            ],
            PickerContext::AddTag => Vec::new(),
            PickerContext::ChatResultLimit => item
                .id
                .parse::<usize>()
                .map(|value| vec![self.set_chat_result_limit_preference(value)])
                .unwrap_or_else(|_| {
                    vec![CoreEffect::Notify("Invalid chat result limit.".to_string())]
                }),
            PickerContext::ChatPerMemoryLimit => item
                .id
                .parse::<usize>()
                .map(|value| vec![self.set_chat_per_memory_limit_preference(value)])
                .unwrap_or_else(|_| {
                    vec![CoreEffect::Notify("Invalid per-memory limit.".to_string())]
                }),
            PickerContext::ChatDiversity => item
                .id
                .parse::<u8>()
                .map(|value| vec![self.set_chat_diversity_preference(value)])
                .unwrap_or_else(|_| {
                    vec![CoreEffect::Notify("Invalid chat diversity.".to_string())]
                }),
        }
    }

    fn start_session_settings_refresh(&mut self) -> Option<CoreEffect> {
        if self.session_settings_task.in_flight {
            return None;
        }

        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        spawn_request_task(
            &mut self.next_session_settings_request_id,
            &mut self.session_settings_task,
            move |request_id, tx| {
                let runtime =
                    Runtime::new().expect("failed to create tokio runtime for settings refresh");
                let overview =
                    runtime.block_on(bridge::load_session_account_overview(use_mainnet, auth));
                let _ = tx.send(SessionSettingsTaskOutput {
                    request_id,
                    overview,
                });
            },
        );

        None
    }

    fn start_create_cost_refresh(&mut self) -> Option<CoreEffect> {
        if self.create_cost_task.in_flight {
            return None;
        }

        self.create_cost_state = CreateCostState::Loading;
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        spawn_request_task(
            &mut self.next_create_request_id,
            &mut self.create_cost_task,
            move |request_id, tx| {
                let runtime =
                    Runtime::new().expect("failed to create tokio runtime for create cost refresh");
                let overview =
                    runtime.block_on(bridge::load_session_account_overview(use_mainnet, auth));
                let _ = tx.send(CreateCostTaskOutput {
                    request_id,
                    overview,
                });
            },
        );

        None
    }

    fn start_create_submit(&mut self, name: String, description: String) -> CoreEffect {
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        spawn_request_task(
            &mut self.next_create_request_id,
            &mut self.create_submit_task,
            move |request_id, tx| {
                let runtime =
                    Runtime::new().expect("failed to create tokio runtime for create submit");
                let result =
                    runtime.block_on(bridge::create_memory(use_mainnet, auth, name, description));
                let _ = tx.send(CreateSubmitTaskOutput { request_id, result });
            },
        );

        CoreEffect::Notify("Creating memory...".to_string())
    }

    fn start_insert_submit(&mut self, request: InsertRequest) -> CoreEffect {
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        spawn_request_task(
            &mut self.next_insert_request_id,
            &mut self.insert_submit_task,
            move |request_id, tx| {
                let runtime =
                    Runtime::new().expect("failed to create tokio runtime for insert submit");
                let result = runtime.block_on(bridge::run_insert(use_mainnet, auth, request));
                let _ = tx.send(InsertSubmitTaskOutput { request_id, result });
            },
        );

        CoreEffect::Notify("Submitting insert request...".to_string())
    }

    fn start_access_submit(
        &mut self,
        memory_id: String,
        action: AccessControlAction,
        principal_id: String,
        role: AccessControlRole,
    ) -> CoreEffect {
        self.access_submit_in_flight = true;
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let (tx, rx) = mpsc::channel();
        self.pending_access_submit = Some(rx);

        thread::spawn(move || {
            let runtime = Runtime::new().expect("failed to create tokio runtime for access submit");
            let result = runtime
                .block_on(bridge::manage_memory_access(
                    use_mainnet,
                    auth,
                    memory_id.clone(),
                    action,
                    principal_id,
                    role,
                ))
                .map_err(|error| error.to_string());
            let _ = tx.send(AccessSubmitTaskOutput { memory_id, result });
        });

        CoreEffect::Notify("Applying access change...".to_string())
    }

    fn start_add_memory_validation(&mut self, memory_id: String) -> Result<CoreEffect, String> {
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        spawn_task(&mut self.add_memory_validation_task, move |tx| {
            let runtime =
                Runtime::new().expect("failed to create tokio runtime for add memory validation");
            let result = runtime
                .block_on(bridge::validate_manual_memory_access(
                    use_mainnet,
                    auth,
                    memory_id.clone(),
                ))
                .map_err(|error| error.to_string());
            let _ = tx.send(AddMemoryValidationTaskOutput { memory_id, result });
        });

        Ok(CoreEffect::Notify(
            "Checking memory access via get_name()...".to_string(),
        ))
    }

    fn start_rename_submit(&mut self, memory_id: String, next_name: String) -> CoreEffect {
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        spawn_task(&mut self.rename_submit_task, move |tx| {
            let runtime = Runtime::new().expect("failed to create tokio runtime for rename submit");
            let result = runtime.block_on(bridge::rename_memory(
                use_mainnet,
                auth,
                memory_id.clone(),
                next_name.clone(),
            ));
            let (stored_name, result) = match result {
                Ok(output) => (Some(output.stored_name), Ok(())),
                Err(error) => (None, Err(error)),
            };
            let _ = tx.send(RenameSubmitTaskOutput {
                memory_id,
                next_name,
                stored_name,
                result,
            });
        });

        CoreEffect::Notify("Renaming memory...".to_string())
    }

    fn save_manual_memory_to_preferences(&mut self, memory_id: &str) -> Result<(), String> {
        if self
            .user_preferences
            .manual_memory_ids
            .iter()
            .any(|existing| existing == memory_id)
        {
            return Ok(());
        }
        self.update_user_preferences(|preferences| {
            preferences.manual_memory_ids.push(memory_id.to_string());
        })
    }

    fn remove_manual_memory_from_preferences(&mut self, memory_id: &str) -> Result<(), String> {
        self.update_user_preferences(|preferences| {
            preferences
                .manual_memory_ids
                .retain(|existing| existing != memory_id);
            if preferences.default_memory_id.as_deref() == Some(memory_id) {
                preferences.default_memory_id = None;
            }
        })
    }

    fn set_chat_result_limit_preference(&mut self, value: usize) -> CoreEffect {
        if self.user_preferences.chat_overall_top_k == value {
            return CoreEffect::Notify(format!(
                "Chat result limit already set to {}",
                preferences::chat_result_limit_display(value)
            ));
        }
        match self.update_user_preferences(|preferences| {
            preferences.chat_overall_top_k = value;
        }) {
            Ok(()) => CoreEffect::Notify(format!(
                "Chat result limit set to {}",
                preferences::chat_result_limit_display(value)
            )),
            Err(error) => CoreEffect::Notify(format!("Chat result limit save failed: {error}")),
        }
    }

    fn set_chat_per_memory_limit_preference(&mut self, value: usize) -> CoreEffect {
        if self.user_preferences.chat_per_memory_cap == value {
            return CoreEffect::Notify(format!(
                "Per-memory limit already set to {}",
                preferences::chat_per_memory_limit_display(value)
            ));
        }
        match self.update_user_preferences(|preferences| {
            preferences.chat_per_memory_cap = value;
        }) {
            Ok(()) => CoreEffect::Notify(format!(
                "Per-memory limit set to {}",
                preferences::chat_per_memory_limit_display(value)
            )),
            Err(error) => CoreEffect::Notify(format!("Per-memory limit save failed: {error}")),
        }
    }

    fn set_chat_diversity_preference(&mut self, value: u8) -> CoreEffect {
        if self.user_preferences.chat_mmr_lambda == value {
            return CoreEffect::Notify(format!(
                "Chat diversity already set to {}",
                preferences::chat_diversity_display(value)
            ));
        }
        match self.update_user_preferences(|preferences| {
            preferences.chat_mmr_lambda = value;
        }) {
            Ok(()) => CoreEffect::Notify(format!(
                "Chat diversity set to {}",
                preferences::chat_diversity_display(value)
            )),
            Err(error) => CoreEffect::Notify(format!("Chat diversity save failed: {error}")),
        }
    }

    fn update_user_preferences<F>(&mut self, update: F) -> Result<(), String>
    where
        F: FnOnce(&mut UserPreferences),
    {
        let mut updated_preferences = self.user_preferences.clone();
        update(&mut updated_preferences);

        #[cfg(test)]
        let _settings_io_lock = settings_io_lock()
            .lock()
            .expect("settings io lock should be available");
        match save_user_preferences_for_apply(&updated_preferences) {
            Ok(()) => {
                let reloaded_preferences = reload_preferences_for_apply(&updated_preferences);
                apply_reloaded_preferences(
                    &mut self.user_preferences,
                    &mut self.preferences_health,
                    updated_preferences,
                    reloaded_preferences,
                );
                Ok(())
            }
            Err(error) => {
                self.preferences_health.save_error = Some(error.to_string());
                Err(error.to_string())
            }
        }
    }

    fn next_memory_after_removal(&self, removed_memory_id: &str) -> Option<String> {
        let visible_ids = self
            .visible_memory_records()
            .into_iter()
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();
        let removed_index = visible_ids
            .iter()
            .position(|memory_id| memory_id == removed_memory_id)?;
        visible_ids.get(removed_index + 1).cloned().or_else(|| {
            removed_index
                .checked_sub(1)
                .and_then(|index| visible_ids.get(index).cloned())
        })
    }

    fn remove_manual_memory_locally(&mut self, memory_id: &str) {
        self.memory_summaries
            .retain(|summary| summary.id != memory_id);
        self.memory_records.retain(|record| record.id != memory_id);
        self.all.retain(|record| record.id != memory_id);
        self.invalidate_memory_summary(memory_id);
        self.loaded_memory_details.remove(memory_id);
        if self.pending_memory_detail_memory_id.as_deref() == Some(memory_id) {
            reset_request_task(&mut self.pending_memory_detail);
            self.pending_memory_detail_memory_id = None;
        }
        self.memory_detail_prefetch
            .queued_memory_ids
            .retain(|queued_memory_id| queued_memory_id != memory_id);
        self.memory_detail_prefetch
            .in_flight_memory_ids
            .remove(memory_id);
        self.refresh_memory_records_from_summaries();
    }

    fn start_transfer_prerequisites_load(&mut self) {
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        spawn_task(&mut self.transfer_prerequisites_task, move |tx| {
            let runtime =
                Runtime::new().expect("failed to create tokio runtime for transfer prerequisites");
            let result = runtime.block_on(bridge::load_transfer_prerequisites(use_mainnet, auth));
            let _ = tx.send(TransferPrerequisitesTaskOutput { result });
        });
    }

    fn cached_transfer_prerequisites(&self) -> Option<(u128, u128)> {
        Some((
            self.session_overview.balance_base_units?,
            self.session_overview.fee_base_units?,
        ))
    }

    fn start_transfer_submit(
        &mut self,
        recipient_principal: String,
        amount_base_units: u128,
        fee_base_units: u128,
    ) -> CoreEffect {
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        spawn_task(&mut self.transfer_submit_task, move |tx| {
            let runtime =
                Runtime::new().expect("failed to create tokio runtime for transfer submit");
            let result = runtime.block_on(bridge::transfer_kinic(
                use_mainnet,
                auth,
                recipient_principal,
                amount_base_units,
                fee_base_units,
            ));
            let _ = tx.send(TransferSubmitTaskOutput { result });
        });

        CoreEffect::Notify("Submitting transfer...".to_string())
    }

    fn build_insert_request(&self, state: &CoreState) -> InsertRequest {
        let memory_id = self.effective_insert_memory_id().unwrap_or_default();
        let tag = state.insert_tag.trim().to_string();
        let file_path = resolved_insert_file_path(state);

        match state.insert_mode {
            InsertMode::File => match file_path {
                Some(path) if insert_file_path_is_pdf(path.as_path()) => InsertRequest::Pdf {
                    memory_id,
                    tag,
                    file_path: path,
                },
                Some(path) => InsertRequest::Normal {
                    memory_id,
                    tag,
                    text: None,
                    file_path: Some(path),
                },
                None => InsertRequest::Normal {
                    memory_id,
                    tag,
                    text: None,
                    file_path: None,
                },
            },
            InsertMode::InlineText => InsertRequest::Normal {
                memory_id,
                tag,
                text: (!state.insert_text.trim().is_empty()).then(|| state.insert_text.clone()),
                file_path: None,
            },
            InsertMode::ManualEmbedding => InsertRequest::Raw {
                memory_id,
                tag,
                text: state.insert_text.clone(),
                embedding_json: state.insert_embedding.clone(),
            },
        }
    }

    fn status_message(&self, state: &CoreState, visible_count: usize) -> String {
        if self.tab_id == KINIC_INSERT_TAB_ID {
            return "Choose mode, target memory, and payload, then press Enter to submit."
                .to_string();
        }
        if self.tab_id == KINIC_CREATE_TAB_ID {
            return "Create a new memory canister from this form.".to_string();
        }
        if self.tab_id == KINIC_SETTINGS_TAB_ID {
            return "Review session details and default memory settings here.".to_string();
        }
        if self.tab_id == KINIC_MARKET_TAB_ID {
            return "Market is not implemented yet.".to_string();
        }
        let base = match self.memories_mode {
            MemoriesMode::Browser => self.browser_status_message(state),
            MemoriesMode::Results => match self.last_search_state.as_ref() {
                Some(last) if last.scope == SearchScope::All => format!(
                    "{visible_count} search results across {} memories",
                    last.target_memory_ids.len()
                ),
                Some(last) => {
                    let memory_id = last
                        .target_memory_ids
                        .first()
                        .map(String::as_str)
                        .unwrap_or("selected");
                    format!("{visible_count} search results in {memory_id}")
                }
                None => format!("{visible_count} search results"),
            },
        };

        if self.initial_memories_in_flight {
            return "Loading memories...".to_string();
        }
        if self.is_memories_load_error_visible() {
            return "Memories unavailable | Ctrl-R retries loading".to_string();
        }

        if let Some(error) = &self.preferences_health.save_error {
            return format!("{base} | preferences save failed: {error}");
        }
        if let Some(error) = &self.preferences_health.load_error {
            return format!("{base} | preferences load failed: {error}");
        }
        base
    }

    fn browser_status_message(&self, state: &CoreState) -> String {
        match state.focus {
            PaneFocus::Search => {
                let scope = match state.search_scope {
                    SearchScope::All => "all memories",
                    SearchScope::Selected => "selected memory",
                };
                format!("Search scope {scope}")
            }
            _ => "Browse memories".to_string(),
        }
    }

    fn invalidate_pending_search(&mut self) {
        if let Some(pending_search) = self.pending_search.take() {
            pending_search.cancellation.cancel();
        }
    }

    fn validate_insert_state(&self, state: &CoreState) -> Result<(), String> {
        match state.insert_mode {
            InsertMode::File => {
                let file_path = resolved_insert_file_path(state)
                    .ok_or_else(|| "File path is required for file insert.".to_string())?;
                validate_supported_file_mode_path(file_path.as_path())?;
            }
            InsertMode::InlineText => {
                if state.insert_text.trim().is_empty() {
                    return Err("Text is required for inline text insert.".to_string());
                }
            }
            InsertMode::ManualEmbedding => {}
        }

        let request = self.build_insert_request(state);
        validate_insert_request_fields(&request).map_err(|error| error.to_string())?;
        validate_insert_request_for_submit(&request).map_err(|error| error.to_string())
    }

    fn validate_insert_expected_dim(&self, request: &InsertRequest) -> Result<(), String> {
        let InsertRequest::Raw { embedding_json, .. } = request else {
            return Ok(());
        };
        let Some(selected_memory_id) = self.effective_insert_memory_id() else {
            return Ok(());
        };
        if self.insert_expected_dim_memory_id.as_deref() != Some(selected_memory_id.as_str()) {
            return Ok(());
        }
        if self.insert_expected_dim_loading {
            return Err(
                "Embedding dimension is still loading for this memory. Wait or press Ctrl-R."
                    .to_string(),
            );
        }
        if let Some(message) = self.insert_expected_dim_load_error.as_ref() {
            return Err(format!(
                "Could not load expected embedding dimension: {message} Try switching away from Insert and back, or press Ctrl-R."
            ));
        }
        let Some(expected_dim) = self.insert_expected_dim else {
            return Err(
                "Embedding dimension is not available yet. Open the Insert tab or press Ctrl-R."
                    .to_string(),
            );
        };

        let provided_dim = parse_embedding_json(embedding_json)
            .map_err(|error| error.to_string())?
            .len() as u64;
        if provided_dim == expected_dim {
            return Ok(());
        }

        Err(format!(
            "Embedding dimension mismatch. Received {provided_dim} values, expected {expected_dim}."
        ))
    }

    fn poll_insert_dim_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_insert_dim.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.reset_insert_dim();
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: Vec::new(),
                });
            }
        };

        self.pending_insert_dim = None;
        if self.insert_expected_dim_memory_id.as_deref() != Some(output.memory_id.as_str()) {
            self.insert_expected_dim_loading = false;
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }
        self.insert_expected_dim_loading = false;

        match output.result {
            Ok(dim) => {
                self.insert_expected_dim_load_error = None;
                self.insert_expected_dim = Some(dim);
            }
            Err(err) => {
                self.insert_expected_dim = None;
                self.insert_expected_dim_load_error = Some(format_insert_submit_error(&err));
            }
        }

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects: Vec::new(),
        })
    }

    fn poll_access_submit_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_access_submit.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_access_submit = None;
                self.access_submit_in_flight = false;
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::AccessFormError(Some(
                        "Access update failed.".to_string(),
                    ))],
                });
            }
        };

        self.pending_access_submit = None;
        self.access_submit_in_flight = false;

        let mut effects = Vec::new();
        match output.result {
            Ok(()) => {
                effects.push(CoreEffect::CloseAccessControl);
                effects.push(CoreEffect::Notify("Access updated.".to_string()));
                self.loaded_memory_details.remove(&output.memory_id);
                if self.active_memory_id() == Some(output.memory_id.as_str()) {
                    self.start_active_memory_detail_load();
                }
            }
            Err(error) => {
                effects.push(CoreEffect::AccessFormError(Some(short_error(&error))));
            }
        }

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }

    fn poll_add_memory_validation_background(
        &mut self,
        state: &CoreState,
    ) -> Option<ProviderOutput> {
        let receiver = self.add_memory_validation_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                finish_task(&mut self.add_memory_validation_task);
                return Some(self.disconnected_task_output(
                    state,
                    CoreEffect::AddMemoryFormError(Some(
                        "Manual memory validation failed.".to_string(),
                    )),
                ));
            }
        };

        finish_task(&mut self.add_memory_validation_task);

        let mut effects = Vec::new();
        match output.result {
            Ok(memory_name) => {
                match self.save_manual_memory_to_preferences(output.memory_id.as_str()) {
                    Ok(()) => {
                        self.preferred_memory_after_refresh = Some(output.memory_id.clone());
                        effects.push(CoreEffect::CloseAddMemory);
                        effects.push(CoreEffect::Notify(if memory_name.trim().is_empty() {
                            format!("Added memory {}.", output.memory_id)
                        } else {
                            format!("Added memory {} ({memory_name}).", output.memory_id)
                        }));
                        if let Some(effect) = self.start_live_memories_load(None, true) {
                            effects.push(effect);
                        }
                    }
                    Err(error) => {
                        effects.push(CoreEffect::AddMemoryFormError(Some(format!(
                            "Manual memory save failed: {error}"
                        ))));
                    }
                }
            }
            Err(error) => {
                effects.push(CoreEffect::AddMemoryFormError(Some(short_error(&error))));
            }
        }

        Some(self.snapshot_output(state, effects))
    }

    fn poll_rename_submit_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.rename_submit_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                finish_task(&mut self.rename_submit_task);
                return Some(self.disconnected_task_output(
                    state,
                    CoreEffect::RenameFormError(Some("Rename request failed.".to_string())),
                ));
            }
        };

        finish_task(&mut self.rename_submit_task);

        let mut effects = Vec::new();
        match output.result {
            Ok(()) => {
                if let Some(summary) = self
                    .memory_summaries
                    .iter_mut()
                    .find(|summary| summary.id == output.memory_id)
                {
                    summary.name = output
                        .stored_name
                        .clone()
                        .unwrap_or_else(|| output.next_name.clone());
                }
                self.refresh_memory_records_from_summaries();
                self.refresh_active_memory_label(output.memory_id.as_str());
                self.loaded_memory_details.remove(&output.memory_id);
                if self.active_memory_id() == Some(output.memory_id.as_str()) {
                    self.start_active_memory_detail_load();
                }
                effects.push(CoreEffect::CloseRenameMemory);
                effects.push(CoreEffect::Notify(format!(
                    "Renamed memory to {}.",
                    output.next_name
                )));
            }
            Err(error) => {
                let message = match error {
                    bridge::RenameMemoryError::ResolveAgentFactory(message)
                    | bridge::RenameMemoryError::BuildAgent(message)
                    | bridge::RenameMemoryError::ParseMemoryId(message)
                    | bridge::RenameMemoryError::Rename(message) => message,
                };
                effects.push(CoreEffect::RenameFormError(Some(message)));
            }
        }

        Some(self.snapshot_output(state, effects))
    }

    fn poll_memory_detail_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_memory_detail.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                reset_request_task(&mut self.pending_memory_detail);
                self.pending_memory_detail_memory_id = None;
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: Vec::new(),
                });
            }
        };

        let is_current_request =
            finish_request_task(&mut self.pending_memory_detail, output.request_id);
        let is_current = is_current_request
            && self.pending_memory_detail_memory_id.as_deref() == Some(output.memory_id.as_str());
        self.pending_memory_detail_memory_id = None;
        if !is_current {
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }

        let effects = self.apply_memory_detail_result(output.memory_id, output.result, true);

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }

    fn poll_memory_detail_prefetch_background(
        &mut self,
        state: &CoreState,
    ) -> Option<ProviderOutput> {
        let receiver = self.memory_detail_prefetch.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                self.memory_detail_prefetch.reset();
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: Vec::new(),
                });
            }
        };

        self.memory_detail_prefetch
            .in_flight_memory_ids
            .remove(output.memory_id.as_str());
        self.pump_memory_detail_prefetch_workers();
        let effects = self.apply_memory_detail_result(output.memory_id, output.result, false);

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }

    fn apply_session_overview(&mut self, overview: SessionAccountOverview) {
        let same_session_context =
            has_same_session_context(&self.session_overview.session, &overview.session);
        let mut session = overview.session;
        if same_session_context && overview.principal_error.is_some() {
            session.principal_id = self.session_overview.session.principal_id.clone();
        }
        self.session_overview = SessionAccountOverview {
            session,
            balance_base_units: overview.balance_base_units,
            fee_base_units: overview.fee_base_units,
            price_base_units: overview.price_base_units,
            principal_error: overview.principal_error,
            balance_error: overview.balance_error,
            fee_error: overview.fee_error,
            price_error: overview.price_error,
        };
    }

    fn search_context(
        request_id: u64,
        query: String,
        scope: SearchScope,
        target_memory_ids: Vec<String>,
    ) -> SearchRequestContext {
        SearchRequestContext {
            request_id,
            query,
            scope,
            target_memory_ids,
        }
    }

    fn validate_access_submit(
        &self,
        state: &CoreState,
    ) -> Result<(String, AccessControlAction, String, AccessControlRole), String> {
        let memory_id = state.access_control.memory_id.trim();
        if memory_id.is_empty() {
            return Err("Select a memory before managing access.".to_string());
        }

        let principal_id = state.access_control.principal_id.trim();
        if principal_id.is_empty() {
            return Err("Principal ID is required.".to_string());
        }

        let action = state.access_control.action;
        let role = state.access_control.role;
        let requested_role = match action {
            AccessControlAction::Remove => None,
            AccessControlAction::Add | AccessControlAction::Change => Some(match role {
                AccessControlRole::Admin => crate::shared::access::MemoryRole::Admin,
                AccessControlRole::Writer => crate::shared::access::MemoryRole::Writer,
                AccessControlRole::Reader => crate::shared::access::MemoryRole::Reader,
            }),
        };
        crate::shared::access::validate_access_control_target(
            principal_id,
            crate::clients::LAUNCHER_CANISTER,
            requested_role,
        )
        .map_err(|error| {
            let message = error.to_string();
            if message == "launcher canister access cannot be modified" {
                "Launcher canister access cannot be modified.".to_string()
            } else if message.starts_with("invalid principal text:") {
                format!("Invalid principal text: {principal_id}")
            } else {
                message
            }
        })?;

        Ok((
            memory_id.to_string(),
            action,
            principal_id.to_string(),
            role,
        ))
    }

    fn validate_add_memory_submit(&self, state: &CoreState) -> Result<String, String> {
        let memory_id = state.add_memory.value.trim();
        if memory_id.is_empty() {
            return Err("Memory canister id is required.".to_string());
        }
        parse_required_principal(memory_id)
            .map_err(|_| format!("Invalid principal text: {memory_id}"))?;
        if self
            .user_preferences
            .manual_memory_ids
            .iter()
            .any(|existing| existing == memory_id)
            || self
                .memory_records
                .iter()
                .any(|record| record.id == memory_id)
        {
            return Err("Memory is already in the list.".to_string());
        }
        Ok(memory_id.to_string())
    }

    fn validate_rename_submit(&self, state: &CoreState) -> Result<(String, String), String> {
        let memory_id = state.rename_memory.memory_id.trim();
        if memory_id.is_empty() {
            return Err("Select a memory before renaming.".to_string());
        }
        parse_required_principal(memory_id)
            .map_err(|_| format!("Invalid principal text: {memory_id}"))?;

        let next_name = state.rename_memory.form.value.trim();
        if next_name.is_empty() {
            return Err("Memory name is required.".to_string());
        }

        Ok((memory_id.to_string(), next_name.to_string()))
    }

    fn validate_transfer_submit(&self, state: &CoreState) -> Result<(String, u128, u128), String> {
        let principal_id = state.transfer_modal.principal_id.trim();
        if principal_id.is_empty() {
            return Err("Recipient principal is required.".to_string());
        }
        parse_required_principal(principal_id)
            .map_err(|_| format!("Invalid principal text: {principal_id}"))?;

        let amount_text = state.transfer_modal.amount.trim();
        if amount_text.is_empty() {
            return Err("Amount is required.".to_string());
        }
        let amount_base_units = parse_required_kinic_amount_to_e8s(amount_text)
            .map_err(format_transfer_amount_parse_error)?;
        if amount_base_units == 0 {
            return Err("Amount must be greater than zero.".to_string());
        }

        let fee_base_units = state
            .transfer_modal
            .fee_base_units
            .ok_or_else(|| "Transfer fee is unavailable.".to_string())?;
        let balance_base_units = state
            .transfer_modal
            .available_balance_base_units
            .ok_or_else(|| "Balance is unavailable.".to_string())?;
        if balance_base_units <= fee_base_units {
            return Err("Available balance does not cover the transfer fee.".to_string());
        }
        let total = amount_base_units
            .checked_add(fee_base_units)
            .ok_or_else(|| "Transfer total exceeds supported range.".to_string())?;
        if total > balance_base_units {
            return Err(format!(
                "Amount exceeds spendable balance. Max sendable is {} KINIC.",
                format_e8s_to_kinic_string_u128(balance_base_units.saturating_sub(fee_base_units))
            ));
        }

        Ok((principal_id.to_string(), amount_base_units, fee_base_units))
    }

    fn matches_pending_search(&self, output: &SearchTaskOutput) -> bool {
        self.pending_search.as_ref().is_some_and(|pending_search| {
            let context = &pending_search.context;
            context.request_id == output.request_id
                && context.query == output.query
                && context.scope == output.scope
                && context.target_memory_ids == output.target_memory_ids
        })
    }

    fn active_memory_id_for_default_selection(&self, state: &CoreState) -> Option<String> {
        match self.memories_mode {
            MemoriesMode::Browser => {
                if self.is_add_memory_action_selected(state) {
                    None
                } else {
                    self.active_memory_id().map(str::to_string)
                }
            }
            MemoriesMode::Results => self
                .result_records
                .get(state.selected_index.unwrap_or(0))
                .and_then(|record| record.source_memory_id.clone()),
        }
    }

    fn search_target_memory_ids(&self, scope: SearchScope) -> Result<Vec<String>, String> {
        match scope {
            SearchScope::All => collect_searchable_memory_ids(
                self.memory_records
                    .iter()
                    .map(|record| record.searchable_memory_id.clone()),
                "No searchable memories are available yet.",
            ),
            SearchScope::Selected => {
                let active_memory_id = self.active_memory_id().ok_or_else(|| {
                    "Select a memory in the list before running search.".to_string()
                })?;
                let record = self
                    .memory_records
                    .iter()
                    .find(|record| record.id == active_memory_id)
                    .ok_or_else(|| {
                        "Select a memory in the list before running search.".to_string()
                    })?;
                let memory_id = record.searchable_memory_id.clone().ok_or_else(|| {
                    "The selected memory is not searchable yet. Wait until setup finishes."
                        .to_string()
                })?;
                Ok(vec![memory_id])
            }
        }
    }

    fn run_live_search(&mut self, scope: SearchScope) -> Option<CoreEffect> {
        let auth = self.config.auth.clone();
        let target_memory_ids = match self.search_target_memory_ids(scope) {
            Ok(targets) => targets,
            Err(message) => return Some(CoreEffect::Notify(message)),
        };
        let query = self.query.trim().to_string();
        if query.is_empty() {
            self.memories_mode = MemoriesMode::Browser;
            self.result_records.clear();
            self.invalidate_pending_search();
            self.last_search_state = None;
            return Some(CoreEffect::Notify(
                "Cleared search query and returned to memories.".to_string(),
            ));
        }

        let use_mainnet = self.config.use_mainnet;
        self.invalidate_pending_search();
        let request_id = self.next_search_request_id;
        self.next_search_request_id += 1;
        let (tx, rx) = mpsc::channel();
        let cancellation = CancellationToken::new();
        let worker_cancellation = cancellation.clone();
        self.pending_search = Some(PendingSearch {
            receiver: rx,
            cancellation,
            context: Self::search_context(
                request_id,
                query.clone(),
                scope,
                target_memory_ids.clone(),
            ),
        });

        thread::spawn(move || {
            let runtime = Runtime::new().expect("failed to create tokio runtime for search");
            let output_target_memory_ids = target_memory_ids.clone();
            let output_query = query.clone();
            let result = runtime.block_on(async move {
                if worker_cancellation.is_cancelled() {
                    return None;
                }
                let embedding_result: Result<Vec<f32>, String> = tokio::select! {
                    _ = worker_cancellation.cancelled() => return None,
                    result = fetch_embedding(&query) => result.map_err(|error| error.to_string()),
                };
                let embedding = match embedding_result {
                    Ok(embedding) => embedding,
                    Err(error) => return Some(Err(error)),
                };
                if worker_cancellation.is_cancelled() {
                    return None;
                }
                let agent = match bridge::build_search_agent(use_mainnet, auth.clone())
                    .await
                    .map_err(|error| error.to_string())
                {
                    Ok(agent) => agent,
                    Err(error) => return Some(Err(error)),
                };
                let concurrency_limit = usize::min(
                    MAX_CONCURRENT_MEMORY_SEARCHES,
                    target_memory_ids.len().max(1),
                );
                let semaphore = Arc::new(Semaphore::new(concurrency_limit));
                let mut tasks = tokio::task::JoinSet::new();
                for memory_id in target_memory_ids.clone() {
                    let agent = agent.clone();
                    let embedding = embedding.clone();
                    let semaphore = semaphore.clone();
                    tasks.spawn(async move {
                        let permit = semaphore
                            .acquire_owned()
                            .await
                            .expect("search semaphore should remain open");
                        let result =
                            bridge::search_memory_with_agent(agent, memory_id.clone(), embedding)
                                .await;
                        drop(permit);
                        (memory_id, result)
                    });
                }

                let mut batch_results = Vec::new();
                let mut join_errors = Vec::new();
                while !tasks.is_empty() {
                    let task: NextMemorySearchTask = tokio::select! {
                        _ = worker_cancellation.cancelled() => {
                            tasks.abort_all();
                            while tasks.join_next().await.is_some() {}
                            return None;
                        }
                        task = tasks.join_next() => task,
                    };
                    let Some(task) = task else {
                        break;
                    };
                    match task {
                        Ok(result) => batch_results.push(result),
                        Err(error) => join_errors.push(error),
                    }
                }

                Some(fold_live_search_results(
                    target_memory_ids.len(),
                    batch_results,
                    join_errors,
                ))
            });
            if let Some(result) = result {
                let _ = tx.send(SearchTaskOutput {
                    request_id,
                    query: output_query,
                    scope,
                    target_memory_ids: output_target_memory_ids,
                    result,
                });
            }
        });

        Some(CoreEffect::Notify("Searching...".to_string()))
    }

    fn poll_search_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = &self.pending_search.as_mut()?.receiver;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_search = None;
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::Notify(
                        "Search worker disconnected unexpectedly.".to_string(),
                    )],
                });
            }
        };

        let is_current_search = self.matches_pending_search(&output);
        self.invalidate_pending_search();

        if !is_current_search {
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }

        let effects = match output.result {
            Ok(results) => {
                let failed_count = results.failed_memory_ids.len() + results.join_error_count;
                let mut items = results.items;
                items.sort_by(|left, right| {
                    right
                        .score
                        .partial_cmp(&left.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                self.result_records = items
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| record_from_search_result(index, item))
                    .collect();
                self.memories_mode = MemoriesMode::Results;
                self.last_search_state = Some(LastSearchState {
                    scope: output.scope,
                    target_memory_ids: output.target_memory_ids.clone(),
                });
                let mut effects = vec![CoreEffect::SelectFirstListItem];
                if state.current_tab_id == KINIC_MEMORIES_TAB_ID {
                    effects.push(CoreEffect::FocusPane(PaneFocus::Items));
                }
                let success_message = match output.scope {
                    SearchScope::All => format!(
                        "Loaded {} search results across {} memories",
                        self.result_records.len(),
                        output.target_memory_ids.len()
                    ),
                    SearchScope::Selected => format!(
                        "Loaded {} search results for {}",
                        self.result_records.len(),
                        output
                            .target_memory_ids
                            .first()
                            .map(String::as_str)
                            .unwrap_or("selected memory")
                    ),
                };
                if failed_count == 0 {
                    effects.push(CoreEffect::Notify(success_message));
                } else {
                    effects.push(CoreEffect::Notify(format!(
                        "{success_message}; {failed_count} memory search(es) failed"
                    )));
                }
                effects
            }
            Err(error) => {
                self.result_records.clear();
                self.memories_mode = MemoriesMode::Browser;
                self.last_search_state = None;
                vec![CoreEffect::Notify(format!("Search failed: {error}"))]
            }
        };

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }

    fn poll_initial_memories_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_initial_memories.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_initial_memories = None;
                self.initial_memories_in_flight = false;
                self.all = vec![load_error_record(
                    "Initial memories worker disconnected unexpectedly.".to_string(),
                )];
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::Notify(
                        "Initial memories load failed unexpectedly.".to_string(),
                    )],
                });
            }
        };

        self.pending_initial_memories = None;
        self.initial_memories_in_flight = false;
        match output.result {
            Ok(memories) => {
                let previous_active_memory = self.active_memory.clone();
                self.memory_summaries = memories;
                self.refresh_memory_records_from_summaries();
                let initial_memory_id = self
                    .preferred_memory_after_refresh
                    .take()
                    .filter(|memory_id| {
                        self.memory_records
                            .iter()
                            .any(|record| record.id == *memory_id)
                    })
                    .or_else(|| {
                        self.default_memory_selection()
                            .preferred_initial_memory_id()
                    })
                    .or_else(|| self.memory_records.first().map(|record| record.id.clone()));
                if self.active_memory_id().is_none()
                    || self.active_memory_id().is_some_and(|memory_id| {
                        !self
                            .memory_records
                            .iter()
                            .any(|record| record.id == memory_id)
                    })
                {
                    if let Some(memory_id) = initial_memory_id {
                        self.set_active_memory_by_id(memory_id);
                    } else {
                        self.clear_active_memory();
                    }
                } else if let Some(memory_id) = self.active_memory_id().map(str::to_string) {
                    self.set_active_memory_by_id(memory_id);
                }
                self.last_search_state = None;
                self.start_memory_detail_prefetch_for_records();
                self.start_active_memory_detail_load();
                self.start_selected_memory_summary_load(false);
                self.start_insert_dim_load();
                let mut effects = Vec::new();
                if self.active_memory != previous_active_memory {
                    self.invalidate_pending_search();
                    effects.extend(self.load_active_chat_history_effects(state));
                }
                Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects,
                })
            }
            Err(error) => {
                let previous_active_memory = self.active_memory.clone();
                self.memory_records.clear();
                self.result_records.clear();
                self.memories_mode = MemoriesMode::Browser;
                let notify_message = format_live_load_failure_message(&error);
                self.all = vec![load_error_record(error)];
                self.clear_active_memory();
                self.last_search_state = None;
                let mut effects = vec![CoreEffect::Notify(notify_message)];
                if self.active_memory != previous_active_memory {
                    self.invalidate_pending_search();
                    effects.extend(self.load_active_chat_history_effects(state));
                }
                Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects,
                })
            }
        }
    }

    fn poll_create_cost_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.create_cost_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                reset_request_task(&mut self.create_cost_task);
                self.create_cost_state = CreateCostState::Error(vec![
                    "Account info refresh worker disconnected.".to_string(),
                ]);
                return Some(self.disconnected_request_output(
                    state,
                    CoreEffect::Notify("Account info refresh failed unexpectedly.".to_string()),
                ));
            }
        };

        let is_current = finish_request_task(&mut self.create_cost_task, output.request_id);
        if !is_current {
            return Some(self.stale_request_output(state));
        }

        let issues = output.overview.account_issue_messages();
        let details = derive_create_cost(
            output.overview.session.principal_id.as_str(),
            output.overview.balance_base_units,
            output.overview.price_base_units.as_ref(),
            output.overview.fee_base_units,
        );
        let next_state = if output.overview.principal_error.is_none() {
            CreateCostState::Loaded(Box::new(LoadedCreateCost {
                overview: output.overview.clone(),
                details,
            }))
        } else if issues.is_empty() {
            CreateCostState::Error(vec![
                "Could not load account overview. Cause: Account overview is unavailable."
                    .to_string(),
            ])
        } else {
            CreateCostState::Error(issues)
        };
        self.apply_session_overview(output.overview);
        self.create_cost_state = next_state;

        Some(self.snapshot_output(state, Vec::new()))
    }

    fn poll_chat_submit_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.chat_submit_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                reset_request_task(&mut self.chat_submit_task);
                return Some(self.snapshot_output(
                    state,
                    vec![
                        CoreEffect::SetChatLoading(false),
                        CoreEffect::Notify(
                            "Ask AI failed: background worker disconnected.".to_string(),
                        ),
                    ],
                ));
            }
        };

        let is_current = finish_request_task(&mut self.chat_submit_task, output.request_id);
        if !is_current {
            return Some(self.stale_request_output(state));
        }

        let mut effects = vec![CoreEffect::SetChatLoading(false)];
        match output.result {
            Ok(success) => {
                let failed_count = success.failed_memory_ids.len() + success.join_error_count;
                let response = success.response;
                if let Err(error) = self.append_chat_history_message(
                    output.history_thread_key.as_str(),
                    output.thread_id.as_str(),
                    "assistant",
                    &response,
                ) {
                    effects.push(CoreEffect::Notify(format!(
                        "Chat history save failed: {error}"
                    )));
                }
                if self
                    .current_chat_thread(output.history_thread_key.as_str())
                    .is_some_and(|thread| thread.thread_id == output.thread_id)
                {
                    effects.push(CoreEffect::AppendChatMessage {
                        role: "assistant".to_string(),
                        content: response,
                    });
                }
                if failed_count > 0 {
                    effects.push(CoreEffect::Notify(format!(
                        "AI response ready; {failed_count} memory search(es) failed"
                    )));
                }
            }
            Err(error) => {
                effects.push(CoreEffect::Notify(format!("Ask AI failed: {error}")));
            }
        }

        Some(self.snapshot_output(state, effects))
    }

    fn poll_memory_summary_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        enum MemorySummaryPollState {
            Pending,
            Ready(MemorySummaryTaskOutput, bool),
            Disconnected,
        }

        let memory_ids = self
            .memory_summary_tasks
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for memory_id in memory_ids {
            let poll_state = {
                let Some(task_state) = self.memory_summary_tasks.get_mut(memory_id.as_str()) else {
                    continue;
                };
                let Some(receiver) = task_state.receiver.as_ref() else {
                    continue;
                };
                match poll_pending_task(receiver) {
                    PendingTaskPoll::Pending => MemorySummaryPollState::Pending,
                    PendingTaskPoll::Ready(output) => {
                        let output_request_id = output.request_id;
                        MemorySummaryPollState::Ready(
                            output,
                            finish_request_task(task_state, output_request_id),
                        )
                    }
                    PendingTaskPoll::Disconnected => {
                        reset_request_task(task_state);
                        MemorySummaryPollState::Disconnected
                    }
                }
            };

            match poll_state {
                MemorySummaryPollState::Pending => continue,
                MemorySummaryPollState::Disconnected => {
                    self.memory_summary_tasks.remove(memory_id.as_str());
                    return Some(self.snapshot_output(state, Vec::new()));
                }
                MemorySummaryPollState::Ready(output, is_current) => {
                    self.memory_summary_tasks.remove(memory_id.as_str());
                    match output.result {
                        Ok(success) => {
                            self.memory_content_summaries
                                .insert(output.memory_id.clone(), success.response);
                            self.failed_memory_content_summaries
                                .remove(output.memory_id.as_str());
                        }
                        Err(error) => {
                            self.memory_content_summaries
                                .remove(output.memory_id.as_str());
                            self.failed_memory_content_summaries
                                .insert(output.memory_id, error);
                        }
                    }
                    if !is_current {
                        return Some(self.stale_request_output(state));
                    }
                    return Some(self.snapshot_output(state, Vec::new()));
                }
            }
        }

        None
    }

    fn poll_create_submit_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.create_submit_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                reset_request_task(&mut self.create_submit_task);
                return Some(self.disconnected_request_output(
                    state,
                    CoreEffect::CreateFormError(Some(
                        "Create request failed: background worker disconnected.".to_string(),
                    )),
                ));
            }
        };

        let is_current = finish_request_task(&mut self.create_submit_task, output.request_id);
        if !is_current {
            return Some(self.stale_request_output(state));
        }

        let previous_active_memory = self.active_memory.clone();
        let mut effects = Vec::new();
        match output.result {
            Ok(success) => {
                if let Some(memories) = success.memories {
                    self.memory_summaries = memories;
                    self.loaded_memory_details.clear();
                    self.memory_detail_prefetch.reset();
                    self.refresh_memory_records_from_summaries();
                    if let Some(index) = self.memory_records.iter().position(|r| r.id == success.id)
                    {
                        let record = self.memory_records.remove(index);
                        self.memory_records.insert(0, record.clone());
                        self.all = self.memory_records.clone();
                        if let Some(index) = self
                            .memory_summaries
                            .iter()
                            .position(|summary| summary.id == success.id)
                        {
                            let summary = self.memory_summaries.remove(index);
                            self.memory_summaries.insert(0, summary);
                        }
                    }
                }
                self.set_active_memory_by_id(success.id.clone());
                self.memories_mode = MemoriesMode::Browser;
                self.result_records.clear();
                self.invalidate_pending_search();
                self.last_search_state = None;
                self.start_memory_detail_prefetch_for_records();
                self.start_active_memory_detail_load();
                self.start_selected_memory_summary_load(false);
                let _ = self.start_create_cost_refresh();
                effects.extend(self.set_tab(KINIC_MEMORIES_TAB_ID));
                effects.push(CoreEffect::SelectFirstListItem);
                effects.push(CoreEffect::ResetCreateFormAndSetTab {
                    tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                });
                effects.push(CoreEffect::FocusPane(PaneFocus::Items));
                let status = if let Some(warning) = success.refresh_warning {
                    format!("Created memory {}. {}", success.id, warning)
                } else {
                    format!("Created memory {}", success.id)
                };
                effects.push(CoreEffect::Notify(status));
            }
            Err(error) => {
                effects.push(CoreEffect::CreateFormError(Some(
                    format_create_submit_error(&error),
                )));
            }
        }
        if self.active_memory != previous_active_memory {
            effects.push(CoreEffect::SetMemoryContentActionIndex(0));
            effects.extend(self.load_active_chat_history_effects(state));
        }

        Some(self.snapshot_output(state, effects))
    }

    fn poll_session_settings_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.session_settings_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                reset_request_task(&mut self.session_settings_task);
                return Some(self.disconnected_request_output(
                    state,
                    CoreEffect::Notify("Session settings refresh failed unexpectedly.".to_string()),
                ));
            }
        };

        let is_current = finish_request_task(&mut self.session_settings_task, output.request_id);
        if !is_current {
            return Some(self.stale_request_output(state));
        }

        let failure_message = output.overview.session_settings_refresh_failure_message();
        let notify_message = output.overview.session_settings_refresh_notify_message();
        self.apply_session_overview(output.overview);
        let effects = failure_message
            .or_else(|| (notify_message != "Session settings refreshed.").then_some(notify_message))
            .map(CoreEffect::Notify)
            .into_iter()
            .collect();

        Some(self.snapshot_output(state, effects))
    }

    fn poll_transfer_prerequisites_background(
        &mut self,
        state: &CoreState,
    ) -> Option<ProviderOutput> {
        let receiver = self.transfer_prerequisites_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                finish_task(&mut self.transfer_prerequisites_task);
                return Some(self.disconnected_task_output(
                    state,
                    CoreEffect::TransferFormError(Some(
                        "Transfer form failed to load unexpectedly.".to_string(),
                    )),
                ));
            }
        };

        finish_task(&mut self.transfer_prerequisites_task);
        let effects = match output.result {
            Ok((balance_base_units, fee_base_units)) => vec![CoreEffect::OpenTransferModal {
                fee_base_units,
                available_balance_base_units: balance_base_units,
            }],
            Err(error) => vec![CoreEffect::TransferFormError(Some(format_transfer_error(
                &error,
            )))],
        };

        Some(self.snapshot_output(state, effects))
    }

    fn poll_transfer_submit_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.transfer_submit_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                finish_task(&mut self.transfer_submit_task);
                return Some(self.disconnected_task_output(
                    state,
                    CoreEffect::TransferFormError(Some(
                        "Transfer request failed: background worker disconnected.".to_string(),
                    )),
                ));
            }
        };

        finish_task(&mut self.transfer_submit_task);
        let mut effects = Vec::new();
        match output.result {
            Ok(success) => {
                effects.push(CoreEffect::CloseTransferModal);
                effects.push(CoreEffect::Notify(format!(
                    "Transferred KINIC successfully. Ledger block {}.",
                    success.block_index
                )));
                if let Some(effect) = self.start_session_settings_refresh() {
                    effects.push(effect);
                }
            }
            Err(error) => {
                effects.push(CoreEffect::TransferFormError(Some(format_transfer_error(
                    &error,
                ))));
            }
        }

        Some(self.snapshot_output(state, effects))
    }

    fn poll_insert_submit_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.insert_submit_task.receiver.as_ref()?;
        let output = match poll_pending_task(receiver) {
            PendingTaskPoll::Pending => return None,
            PendingTaskPoll::Ready(output) => output,
            PendingTaskPoll::Disconnected => {
                reset_request_task(&mut self.insert_submit_task);
                return Some(self.disconnected_request_output(
                    state,
                    CoreEffect::InsertFormError(Some(
                        "Insert request failed: background worker disconnected.".to_string(),
                    )),
                ));
            }
        };

        let is_current = finish_request_task(&mut self.insert_submit_task, output.request_id);
        if !is_current {
            return Some(self.stale_request_output(state));
        }

        let effects = match output.result {
            Ok(success) => {
                self.invalidate_memory_summary(success.memory_id.as_str());
                if self.active_memory_id() == Some(success.memory_id.as_str()) {
                    self.start_selected_memory_summary_load(true);
                }
                vec![
                    CoreEffect::InsertFormError(None),
                    CoreEffect::ResetInsertFormForRepeat,
                    CoreEffect::NotifyPersistent(insert_success_status(&success)),
                ]
            }
            Err(error) => vec![CoreEffect::InsertFormError(Some(
                format_insert_submit_error(&error),
            ))],
        };

        Some(self.snapshot_output(state, effects))
    }

    fn reset_memories_browser(&mut self) {
        self.memories_mode = MemoriesMode::Browser;
        self.result_records.clear();
        self.invalidate_pending_search();
        self.last_search_state = None;
    }

    fn set_tab(&mut self, tab_id: &str) -> Vec<CoreEffect> {
        self.tab_id = tab_id.to_string();
        self.invalidate_pending_search();

        match tab_id {
            KINIC_MEMORIES_TAB_ID => {
                self.reset_memories_browser();
                Vec::new()
            }
            KINIC_INSERT_TAB_ID => Vec::new(),
            KINIC_CREATE_TAB_ID => {
                let mut effects = Vec::new();
                if let Some(effect) = self.start_create_cost_refresh() {
                    effects.push(effect);
                }
                effects
            }
            KINIC_MARKET_TAB_ID => {
                vec![CoreEffect::Notify(
                    "Market is not implemented yet.".to_string(),
                )]
            }
            KINIC_SETTINGS_TAB_ID => self.start_session_settings_refresh().into_iter().collect(),
            _ => vec![CoreEffect::Notify(format!("Switched kinic tab: {tab_id}"))],
        }
    }
}

fn normalize_insert_file_path_input(path: &str) -> &str {
    let trimmed = path.trim();
    if let Some(inner) = trimmed
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
    {
        return inner;
    }
    if let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        return inner;
    }
    trimmed
}

fn validate_supported_file_mode_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("File path is required for file insert.".to_string());
    }

    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return Err(format!(
            "File path must use a supported {} extension.",
            allowed_extension_list(FILE_MODE_ALLOWED_EXTENSIONS)
        ));
    };

    if FILE_MODE_ALLOWED_EXTENSIONS
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed))
    {
        return Ok(());
    }

    Err(format!(
        "File path must use a supported {} extension.",
        allowed_extension_list(FILE_MODE_ALLOWED_EXTENSIONS)
    ))
}

fn allowed_extension_list(allowed_extensions: &[&str]) -> String {
    allowed_extensions
        .iter()
        .map(|extension| format!(".{extension}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn insert_file_path_is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

fn resolved_insert_file_path(state: &CoreState) -> Option<std::path::PathBuf> {
    state.insert_selected_file_path.clone().or_else(|| {
        let normalized = normalize_insert_file_path_input(state.insert_file_path_input.trim());
        (!normalized.is_empty()).then(|| std::path::PathBuf::from(normalized))
    })
}

impl DataProvider for KinicProvider {
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
        self.initialize_live_memories();
        let state = CoreState::default();
        Ok(self.build_snapshot(&state))
    }

    fn handle_action(
        &mut self,
        action: &CoreAction,
        state: &CoreState,
    ) -> CoreResult<ProviderOutput> {
        let previous_active_memory = self.active_memory.clone();
        let mut effects = Vec::new();
        let mut close_picker_after_submit = false;
        match action {
            CoreAction::SetQuery(q) => {
                self.query = q.clone();
                let _ = self.sync_memory_browser_selection();
                self.invalidate_pending_search();
                if self.tab_id == KINIC_MEMORIES_TAB_ID && q.is_empty() {
                    self.reset_memories_browser();
                }
            }
            CoreAction::SearchInput(c) => {
                self.query.push(*c);
                let _ = self.sync_memory_browser_selection();
                self.invalidate_pending_search();
            }
            CoreAction::SearchBackspace => {
                self.query.pop();
                let _ = self.sync_memory_browser_selection();
                self.invalidate_pending_search();
                if self.query.is_empty() {
                    self.reset_memories_browser();
                }
            }
            CoreAction::SearchScopePrev => {
                self.invalidate_pending_search();
            }
            CoreAction::SearchScopeNext => {
                self.invalidate_pending_search();
            }
            CoreAction::ChatScopePrev | CoreAction::ChatScopeNext | CoreAction::ChatScopeAll => {
                self.sync_active_memory_to_chat_scope_for_browser(state);
                effects.extend(self.load_active_chat_history_effects(state));
            }
            CoreAction::ChatNewThread => {
                if self.chat_submit_task.in_flight {
                    effects.push(CoreEffect::Notify(
                        "Wait for the current AI response before starting a new thread."
                            .to_string(),
                    ));
                } else if let Some(history_thread_key) = self.chat_history_thread_key(state) {
                    match self.create_chat_thread(history_thread_key.as_str()) {
                        Ok(_) => {
                            effects.push(CoreEffect::ReplaceChatMessages(Vec::new()));
                            effects.push(CoreEffect::SetChatLoading(false));
                            effects
                                .push(CoreEffect::Notify("Started a new chat thread.".to_string()));
                        }
                        Err(error) => effects.push(CoreEffect::Notify(format!(
                            "Chat thread create failed: {error}"
                        ))),
                    }
                } else {
                    effects.push(CoreEffect::Notify(
                        "Select a memory before starting a new thread.".to_string(),
                    ));
                }
            }
            CoreAction::SearchSubmit => {
                if let Some(effect) = self.run_live_search(state.search_scope) {
                    effects.push(effect);
                }
            }
            CoreAction::MoveNext if self.should_handle_memory_navigation(state) => {
                self.navigate_active_memory(state, action)
            }
            CoreAction::MovePrev if self.should_handle_memory_navigation(state) => {
                self.navigate_active_memory(state, action)
            }
            CoreAction::MoveHome if self.should_handle_memory_navigation(state) => {
                self.navigate_active_memory(state, action)
            }
            CoreAction::MoveEnd if self.should_handle_memory_navigation(state) => {
                self.navigate_active_memory(state, action)
            }
            CoreAction::MovePageDown if self.should_handle_memory_navigation(state) => {
                self.navigate_active_memory(state, action)
            }
            CoreAction::MovePageUp if self.should_handle_memory_navigation(state) => {
                self.navigate_active_memory(state, action)
            }
            CoreAction::OpenSelected => {
                if self.is_add_memory_action_selected(state) {
                    effects.push(CoreEffect::OpenAddMemory);
                    effects.push(CoreEffect::FocusPane(PaneFocus::Items));
                }
            }
            CoreAction::SetTab(id) => {
                effects.extend(self.set_tab(id.0.as_str()));
                if id.0.as_str() == KINIC_INSERT_TAB_ID {
                    self.start_insert_dim_load();
                }
            }
            CoreAction::ChatSubmit => {
                if self.chat_submit_task.in_flight {
                    effects.push(CoreEffect::Notify(
                        "Ask AI request already running.".to_string(),
                    ));
                } else {
                    let history_thread_key = self.chat_history_thread_key(state);
                    match (history_thread_key, self.chat_targets(state)) {
                        (Some(history_thread_key), Ok(targets)) => {
                            let thread_id =
                                match self.ensure_chat_thread_id(history_thread_key.as_str()) {
                                    Ok(thread_id) => thread_id,
                                    Err(error) => {
                                        effects.push(CoreEffect::SetChatLoading(false));
                                        effects.push(CoreEffect::Notify(format!(
                                            "Chat thread load failed: {error}"
                                        )));
                                        return Ok(self.snapshot_output(state, effects));
                                    }
                                };
                            let submitted_query = state
                                .chat_messages
                                .last()
                                .map(|(_, content)| content.clone())
                                .unwrap_or_default();
                            let prompt_history = self.prompt_history_messages(state);
                            if let Some((role, content)) = state.chat_messages.last()
                                && role == "user"
                                && let Err(error) = self.append_chat_history_message(
                                    history_thread_key.as_str(),
                                    thread_id.as_str(),
                                    role,
                                    content,
                                )
                            {
                                effects.push(CoreEffect::Notify(format!(
                                    "Chat history save failed: {error}"
                                )));
                            }
                            effects.push(self.start_chat_submit(
                                state,
                                history_thread_key,
                                thread_id,
                                targets,
                                submitted_query,
                                prompt_history,
                            ));
                        }
                        (None, _) => {
                            effects.push(CoreEffect::SetChatLoading(false));
                            effects.push(CoreEffect::Notify(
                                "Select a memory before asking AI.".to_string(),
                            ));
                        }
                        (_, Err(error)) => {
                            effects.push(CoreEffect::SetChatLoading(false));
                            effects.push(CoreEffect::Notify(error));
                        }
                    }
                }
            }
            CoreAction::CreateSubmit => {
                let name = state.create_name.trim().to_string();
                let description = state.create_description.trim().to_string();
                if name.is_empty() || description.is_empty() {
                    effects.push(CoreEffect::CreateFormError(Some(
                        "Name and description are required.".to_string(),
                    )));
                } else if self.create_submit_task.in_flight {
                    effects.push(CoreEffect::Notify(
                        "Create request already running.".to_string(),
                    ));
                } else {
                    effects.push(self.start_create_submit(name, description));
                }
            }
            CoreAction::InsertSubmit => {
                let request = self.build_insert_request(state);
                if let Err(error) = self.validate_insert_state(state) {
                    effects.push(CoreEffect::InsertFormError(Some(error)));
                } else if let Err(error) = self.validate_insert_expected_dim(&request) {
                    effects.push(CoreEffect::InsertFormError(Some(error)));
                } else if self.insert_submit_task.in_flight {
                    effects.push(CoreEffect::Notify(
                        "Insert request already running.".to_string(),
                    ));
                } else {
                    let request = self.build_insert_request(state);
                    effects.push(self.start_insert_submit(request));
                }
            }
            CoreAction::CreateRefresh => {
                if let Some(effect) = self.start_create_cost_refresh() {
                    effects.push(effect);
                }
            }
            CoreAction::InsertOpenFileDialog => {}
            CoreAction::RefreshCurrentView => {
                effects.extend(self.refresh_current_view());
            }
            CoreAction::ToggleSettings => {
                if let Some(effect) = self.start_session_settings_refresh() {
                    effects.push(effect);
                }
            }
            CoreAction::OpenPicker(_)
            | CoreAction::ClosePicker
            | CoreAction::MovePickerNext
            | CoreAction::MovePickerPrev
            | CoreAction::DeleteSelectedPickerItem
            | CoreAction::PickerInput(_)
            | CoreAction::PickerBackspace => {}
            CoreAction::MemoryContentMoveNext => {
                let next_index = self.next_content_index(state, 1);
                effects.push(CoreEffect::SetMemoryContentActionIndex(next_index));
                let mut preview_state = state.clone();
                preview_state.memory_content_action_index = next_index;
                return Ok(ProviderOutput {
                    snapshot: Some(self.build_snapshot(&preview_state)),
                    effects,
                });
            }
            CoreAction::MemoryContentMovePrev => {
                let next_index = self.next_content_index(state, -1);
                effects.push(CoreEffect::SetMemoryContentActionIndex(next_index));
                let mut preview_state = state.clone();
                preview_state.memory_content_action_index = next_index;
                return Ok(ProviderOutput {
                    snapshot: Some(self.build_snapshot(&preview_state)),
                    effects,
                });
            }
            CoreAction::MemoryContentJumpNext => {
                let selections = self.memory_content_selections(state);
                let current = state
                    .memory_content_action_index
                    .min(selections.len().saturating_sub(1));
                if current + 1 >= selections.len().max(1) {
                    let next_focus = if state.chat_open {
                        PaneFocus::Extra
                    } else {
                        PaneFocus::Search
                    };
                    effects.push(CoreEffect::FocusPane(next_focus));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                }
                let next_index = current + 1;
                effects.push(CoreEffect::SetMemoryContentActionIndex(next_index));
                let mut preview_state = state.clone();
                preview_state.memory_content_action_index = next_index;
                return Ok(ProviderOutput {
                    snapshot: Some(self.build_snapshot(&preview_state)),
                    effects,
                });
            }
            CoreAction::MemoryContentJumpPrev => {
                if state.memory_content_action_index == 0 {
                    effects.push(CoreEffect::FocusPane(PaneFocus::Items));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                }
                let next_index = state.memory_content_action_index - 1;
                effects.push(CoreEffect::SetMemoryContentActionIndex(next_index));
                let mut preview_state = state.clone();
                preview_state.memory_content_action_index = next_index;
                return Ok(ProviderOutput {
                    snapshot: Some(self.build_snapshot(&preview_state)),
                    effects,
                });
            }
            CoreAction::MemoryContentOpenSelected => {
                let Some(memory_id) = self.active_memory_id().map(str::to_string) else {
                    effects.push(CoreEffect::Notify(
                        "Select a memory before running this action.".to_string(),
                    ));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                };
                match self.content_selection(state) {
                    Some(MemoryContentSelection::RenameMemory) => {
                        let Some((summary, current_name)) = self.active_rename_target(state) else {
                            effects.push(CoreEffect::Notify(
                                "Select a memory before renaming.".to_string(),
                            ));
                            return Ok(ProviderOutput {
                                snapshot: Some(self.build_snapshot(state)),
                                effects,
                            });
                        };
                        effects.push(CoreEffect::OpenRenameMemory {
                            memory_id: summary.id.clone(),
                            current_name,
                        });
                    }
                    Some(MemoryContentSelection::User(user)) => {
                        effects.push(CoreEffect::OpenAccessAction {
                            memory_id,
                            principal_id: user.principal_id.clone(),
                            role: Self::access_role_from_user(user),
                        });
                    }
                    Some(MemoryContentSelection::AddUser) | None => {
                        effects.push(CoreEffect::OpenAccessAdd { memory_id });
                    }
                    Some(MemoryContentSelection::RemoveManualMemory) => {
                        effects.push(CoreEffect::OpenRemoveMemory);
                    }
                }
            }
            CoreAction::CloseAccessControl => {
                effects.push(CoreEffect::CloseAccessControl);
            }
            CoreAction::AccessNextField => {}
            CoreAction::AccessPrevField => {}
            CoreAction::AccessInput(_) => {}
            CoreAction::AccessBackspace => {}
            CoreAction::AccessCycleAction => {}
            CoreAction::AccessCycleActionPrev => {}
            CoreAction::AccessCycleRole => {}
            CoreAction::AccessCycleRolePrev => {}
            CoreAction::AccessSubmit => {
                if self.access_submit_in_flight {
                    effects.push(CoreEffect::Notify(
                        "Access request already running.".to_string(),
                    ));
                } else {
                    match state.access_control.mode {
                        AccessControlMode::Action => {
                            effects.push(CoreEffect::OpenAccessConfirm {
                                memory_id: state.access_control.memory_id.clone(),
                                principal_id: state.access_control.principal_id.clone(),
                                action: state.access_control.action,
                                role: state.access_control.role,
                            });
                        }
                        AccessControlMode::Add | AccessControlMode::Confirm => {
                            if state.access_control.mode == AccessControlMode::Confirm
                                && !state.access_control.confirm_yes
                            {
                                effects.push(CoreEffect::CloseAccessControl);
                                return Ok(ProviderOutput {
                                    snapshot: Some(self.build_snapshot(state)),
                                    effects,
                                });
                            }
                            match self.validate_access_submit(state) {
                                Ok((memory_id, action, principal_id, role)) => {
                                    effects.push(self.start_access_submit(
                                        memory_id,
                                        action,
                                        principal_id,
                                        role,
                                    ));
                                }
                                Err(error) => {
                                    effects.push(CoreEffect::AccessFormError(Some(error)));
                                }
                            }
                        }
                        AccessControlMode::None => {}
                    }
                }
            }
            CoreAction::OpenAddMemory => {
                effects.push(CoreEffect::OpenAddMemory);
            }
            CoreAction::CloseAddMemory => {
                effects.push(CoreEffect::CloseAddMemory);
            }
            CoreAction::AddMemoryInput(_) => {}
            CoreAction::AddMemoryBackspace => {}
            CoreAction::AddMemorySubmit => {
                if self.add_memory_validation_task.in_flight {
                    effects.push(CoreEffect::Notify(
                        "Manual memory validation already running.".to_string(),
                    ));
                } else {
                    match self.validate_add_memory_submit(state) {
                        Ok(memory_id) => match self.start_add_memory_validation(memory_id) {
                            Ok(effect) => effects.push(effect),
                            Err(error) => {
                                effects.push(CoreEffect::AddMemoryFormError(Some(error)));
                            }
                        },
                        Err(error) => {
                            effects.push(CoreEffect::AddMemoryFormError(Some(error)));
                        }
                    }
                }
            }
            CoreAction::OpenRemoveMemory => {
                if !self.active_memory_is_manual() {
                    effects.push(CoreEffect::Notify(
                        "Only manually added memories can be removed from this list.".to_string(),
                    ));
                } else {
                    effects.push(CoreEffect::OpenRemoveMemory);
                }
            }
            CoreAction::CloseRemoveMemory => {
                effects.push(CoreEffect::CloseRemoveMemory);
            }
            CoreAction::RemoveMemoryToggleConfirm => {}
            CoreAction::RemoveMemorySubmit => {
                if !state.remove_memory.open {
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                }
                if !state.remove_memory.confirm_yes {
                    effects.push(CoreEffect::CloseRemoveMemory);
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                }

                let Some(memory_id) = self.active_memory_id().map(str::to_string) else {
                    effects.push(CoreEffect::RemoveMemoryFormError(Some(
                        "Select a memory before removing it.".to_string(),
                    )));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                };
                if !self.active_memory_is_manual() {
                    effects.push(CoreEffect::RemoveMemoryFormError(Some(
                        "Only manually added memories can be removed from this list.".to_string(),
                    )));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                }

                let next_memory_id = self.next_memory_after_removal(memory_id.as_str());
                match self.remove_manual_memory_from_preferences(memory_id.as_str()) {
                    Ok(()) => {
                        self.remove_manual_memory_locally(memory_id.as_str());
                        if let Some(memory_id) = next_memory_id {
                            self.set_active_memory_by_id(memory_id);
                        } else {
                            self.clear_active_memory();
                        }
                        let _ = self.sync_memory_browser_selection();
                        self.invalidate_pending_search();
                        self.start_active_memory_detail_load();
                        effects.push(CoreEffect::CloseRemoveMemory);
                        effects.push(CoreEffect::SetMemoryContentActionIndex(0));
                        effects.push(CoreEffect::Notify(format!(
                            "Removed manual memory {memory_id} from this list"
                        )));
                    }
                    Err(error) => {
                        effects.push(CoreEffect::RemoveMemoryFormError(Some(format!(
                            "Manual memory removal failed: {error}"
                        ))));
                    }
                }
            }
            CoreAction::OpenRenameMemory => {
                let Some((summary, current_name)) = self.active_rename_target(state) else {
                    effects.push(CoreEffect::Notify(
                        "Select a memory before renaming.".to_string(),
                    ));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                };
                effects.push(CoreEffect::OpenRenameMemory {
                    memory_id: summary.id.clone(),
                    current_name,
                });
            }
            CoreAction::CloseRenameMemory => {
                effects.push(CoreEffect::CloseRenameMemory);
            }
            CoreAction::RenameMemoryInput(_) => {}
            CoreAction::RenameMemoryBackspace => {}
            CoreAction::RenameMemoryNextField => {}
            CoreAction::RenameMemoryPrevField => {}
            CoreAction::RenameMemorySubmit => {
                if self.rename_submit_task.in_flight {
                    effects.push(CoreEffect::Notify(
                        "Rename request already running.".to_string(),
                    ));
                } else {
                    match self.validate_rename_submit(state) {
                        Ok((memory_id, next_name)) => {
                            effects.push(self.start_rename_submit(memory_id, next_name));
                        }
                        Err(error) => {
                            effects.push(CoreEffect::RenameFormError(Some(error)));
                        }
                    }
                }
            }
            CoreAction::OpenTransferModal => {
                if let Some((available_balance_base_units, fee_base_units)) =
                    self.cached_transfer_prerequisites()
                {
                    effects.push(CoreEffect::OpenTransferModal {
                        fee_base_units,
                        available_balance_base_units,
                    });
                } else if !self.transfer_prerequisites_task.in_flight {
                    self.start_transfer_prerequisites_load();
                }
            }
            CoreAction::CloseTransferModal => {
                effects.push(CoreEffect::CloseTransferModal);
            }
            CoreAction::TransferInput(_)
            | CoreAction::TransferBackspace
            | CoreAction::TransferNextField
            | CoreAction::TransferPrevField
            | CoreAction::TransferApplyMax
            | CoreAction::TransferConfirmToggle => {}
            CoreAction::TransferSubmit => {
                if self.transfer_submit_task.in_flight {
                    effects.push(CoreEffect::Notify(
                        "Transfer request already running.".to_string(),
                    ));
                } else if state.transfer_modal.mode == TransferModalMode::Confirm {
                    if !state.transfer_modal.confirm_yes {
                        effects.push(CoreEffect::CloseTransferModal);
                    } else {
                        match self.validate_transfer_submit(state) {
                            Ok((recipient_principal, amount_base_units, fee_base_units)) => {
                                effects.push(self.start_transfer_submit(
                                    recipient_principal,
                                    amount_base_units,
                                    fee_base_units,
                                ));
                            }
                            Err(error) => {
                                effects.push(CoreEffect::TransferFormError(Some(error)));
                            }
                        }
                    }
                } else {
                    match self.validate_transfer_submit(state) {
                        Ok(_) => {
                            effects.push(CoreEffect::OpenTransferConfirm);
                        }
                        Err(error) => {
                            effects.push(CoreEffect::TransferFormError(Some(error)));
                        }
                    }
                }
            }
            CoreAction::ScrollContentPageDown => {}
            CoreAction::ScrollContentPageUp => {}
            CoreAction::ScrollContentHome => {}
            CoreAction::ScrollContentEnd => {}
            CoreAction::SubmitPicker => match &state.picker {
                PickerState::Closed => {}
                PickerState::Input {
                    context: PickerContext::AddTag,
                    value,
                    ..
                } => {
                    let outcome = self.save_tags_to_preferences(value.clone());
                    if outcome.result == SaveTagResult::Saved {
                        close_picker_after_submit = true;
                    }
                    effects.push(outcome.effect);
                }
                PickerState::Input { .. } => {}
                PickerState::List {
                    context,
                    items,
                    selected_index,
                    mode,
                    ..
                } => {
                    if let PickerListMode::Confirm { kind } = mode {
                        if let Some(effect) = self.picker_confirm_effect(*context, kind) {
                            effects.push(effect);
                        }
                        return Ok(ProviderOutput {
                            snapshot: Some(self.build_snapshot(state)),
                            effects,
                        });
                    }

                    let Some(item) = items.get(*selected_index) else {
                        effects.push(CoreEffect::Notify("No options available yet.".to_string()));
                        return Ok(ProviderOutput {
                            snapshot: Some(self.build_snapshot(state)),
                            effects,
                        });
                    };

                    if item.kind == PickerItemKind::AddAction {
                        return Ok(ProviderOutput {
                            snapshot: Some(self.build_snapshot(state)),
                            effects,
                        });
                    }

                    effects.extend(self.picker_option_submit_effects(*context, item));
                }
            },
            CoreAction::SetDefaultMemoryFromSelection => {
                let Some(memory_id) = self.active_memory_id_for_default_selection(state) else {
                    effects.push(CoreEffect::Notify(
                        "Select a memory before setting the default.".to_string(),
                    ));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                };
                effects.push(
                    self.default_memory_controller()
                        .set_default_memory_preference(memory_id),
                );
            }
            _ => {}
        }

        let snapshot = match &state.picker {
            PickerState::Input {
                context: PickerContext::AddTag,
                value,
                ..
            } if matches!(action, CoreAction::SubmitPicker)
                && close_picker_after_submit
                && !value.trim().is_empty() =>
            {
                self.build_snapshot_with_picker(state, PickerState::Closed)
            }
            PickerState::List { context, .. }
                if matches!(action, CoreAction::SubmitPicker)
                    && matches!(
                        context,
                        PickerContext::DefaultMemory
                            | PickerContext::InsertTarget
                            | PickerContext::InsertTag
                            | PickerContext::TagManagement
                            | PickerContext::ChatResultLimit
                            | PickerContext::ChatPerMemoryLimit
                            | PickerContext::ChatDiversity
                    ) =>
            {
                match &state.picker {
                    PickerState::List {
                        context,
                        mode: PickerListMode::Confirm { .. },
                        ..
                    } if *context == PickerContext::TagManagement => self.build_snapshot(state),
                    _ => self.build_snapshot_with_picker(state, PickerState::Closed),
                }
            }
            _ => self.build_snapshot(state),
        };
        let chat_scope_follows_active_memory = state.chat_scope == ChatScope::Selected
            && !matches!(
                action,
                CoreAction::ChatScopePrev | CoreAction::ChatScopeNext | CoreAction::ChatScopeAll
            );
        if self.active_memory != previous_active_memory {
            self.invalidate_pending_search();
            self.start_active_memory_detail_load();
            self.start_selected_memory_summary_load(false);
            effects.push(CoreEffect::SetMemoryContentActionIndex(0));
            if chat_scope_follows_active_memory {
                effects.extend(self.load_active_chat_history_effects(state));
            }
        }

        Ok(ProviderOutput {
            snapshot: Some(snapshot),
            effects,
        })
    }

    fn poll_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        self.poll_initial_memories_background(state)
            .or_else(|| self.poll_memory_summary_background(state))
            .or_else(|| self.poll_memory_detail_background(state))
            .or_else(|| self.poll_memory_detail_prefetch_background(state))
            .or_else(|| self.poll_chat_submit_background(state))
            .or_else(|| self.poll_create_submit_background(state))
            .or_else(|| self.poll_access_submit_background(state))
            .or_else(|| self.poll_add_memory_validation_background(state))
            .or_else(|| self.poll_rename_submit_background(state))
            .or_else(|| self.poll_transfer_submit_background(state))
            .or_else(|| self.poll_transfer_prerequisites_background(state))
            .or_else(|| self.poll_insert_dim_background(state))
            .or_else(|| self.poll_memory_detail_background(state))
            .or_else(|| self.poll_insert_submit_background(state))
            .or_else(|| self.poll_create_cost_background(state))
            .or_else(|| self.poll_session_settings_background(state))
            .or_else(|| self.poll_search_background(state))
    }
}

fn format_transfer_amount_parse_error(error: KinicAmountParseError) -> String {
    match error {
        KinicAmountParseError::Empty => "Amount is required.".to_string(),
        KinicAmountParseError::Negative => "Amount must be a positive decimal number.".to_string(),
        KinicAmountParseError::TooManyParts => {
            "Amount must be a decimal number with up to 8 fraction digits.".to_string()
        }
        KinicAmountParseError::NonDigit => "Amount must contain digits only.".to_string(),
        KinicAmountParseError::TooManyFractionDigits => {
            "Amount supports up to 8 decimal places.".to_string()
        }
        KinicAmountParseError::Overflow => "Amount exceeds supported range.".to_string(),
    }
}

fn format_transfer_error(error: &bridge::TransferKinicError) -> String {
    match error {
        bridge::TransferKinicError::ResolveAgentFactory(message) => {
            format!("Transfer setup failed: {message}")
        }
        bridge::TransferKinicError::BuildAgent(message) => {
            format!("Transfer agent failed: {message}")
        }
        bridge::TransferKinicError::ParsePrincipal(message) => {
            format!("Recipient principal failed: {message}")
        }
        bridge::TransferKinicError::LoadBalance(message) => {
            format!("Transfer balance lookup failed: {message}")
        }
        bridge::TransferKinicError::LoadFee(message) => {
            format!("Transfer fee lookup failed: {message}")
        }
        bridge::TransferKinicError::Transfer(message) => {
            format!("Transfer failed: {message}")
        }
    }
}

fn loading_memories_record() -> KinicRecord {
    KinicRecord::new(
        "kinic-live-loading",
        "Loading memories...",
        "memories",
        "Waiting for launcher response.",
        "## Loading Memories\n\nThe TUI started successfully and is waiting for the launcher to respond.\n",
    )
}

fn add_memory_action_record() -> KinicRecord {
    KinicRecord::new(
        ADD_MEMORY_ACTION_ID,
        "+ Add Existing Memory Canister",
        "action",
        "Add an existing memory canister to this local list.",
        "## Add Existing Memory Canister\n\nOpen this action to register an existing memory canister by id.\n",
    )
}

fn manual_memory_summary(id: &str) -> MemorySummary {
    MemorySummary {
        id: id.to_string(),
        status: "manually added".to_string(),
        detail: "Manually added memory. Loading details...".to_string(),
        searchable_memory_id: Some(id.to_string()),
        name: "unknown".to_string(),
        version: "unknown".to_string(),
        dim: None,
        owners: None,
        stable_memory_size: None,
        cycle_amount: None,
        users: None,
    }
}

fn marker_prefix(selected: bool) -> &'static str {
    if selected { ">" } else { " " }
}

fn marker_label(selected: bool, label: &str) -> String {
    format!("{} {}", marker_prefix(selected), label)
}

fn marker_line(selected: bool, label: &str) -> String {
    format!("{} {}", marker_prefix(selected), label)
}

fn render_access_lines(
    users: Option<&Vec<bridge::MemoryUser>>,
    current_selection: Option<&MemoryContentSelection<'_>>,
) -> Vec<String> {
    let mut lines = Vec::new();
    let user_list = users.cloned().unwrap_or_default();

    match users {
        None => lines.push("unavailable".to_string()),
        Some(users) if users.is_empty() => lines.push("none".to_string()),
        Some(_) => {}
    }

    for user in &user_list {
        let selected = matches!(
            current_selection,
            Some(MemoryContentSelection::User(candidate)) if *candidate == user
        );
        lines.push(marker_line(
            selected,
            format!(
                "{}   {}",
                adapter::short_id(user.principal_id.as_str()),
                user.role
            )
            .as_str(),
        ));
    }

    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines.push(marker_line(
        matches!(current_selection, Some(MemoryContentSelection::AddUser)),
        "+ Add User",
    ));
    lines
}

fn short_error(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_string()
}

fn format_live_load_failure_message(error: &str) -> String {
    format!("Unable to load memories. Cause: {}", short_error(error))
}

fn load_error_record(error: String) -> KinicRecord {
    let subtitle = keychain_context_note(error.as_str())
        .unwrap_or("Check your identity or network configuration.");
    KinicRecord::new(
        "kinic-live-error",
        "Unable to load memories",
        "memories",
        subtitle,
        format!("## Live Load Error\n\n{error}"),
    )
}

fn keychain_context_note(error: &str) -> Option<&'static str> {
    match extract_keychain_error_code(error) {
        Some(KeychainErrorCode::LookupFailed) => {
            Some("Check the macOS Keychain entry and whether approval was delayed or interrupted.")
        }
        Some(KeychainErrorCode::AccessDenied | KeychainErrorCode::InteractionNotAllowed) => {
            Some("Check the macOS Keychain prompt and the selected identity entry.")
        }
        Some(KeychainErrorCode::KeychainError) => {
            Some("Check the macOS Keychain entry and local security settings.")
        }
        None => None,
    }
}

fn record_from_memory_summary(memory: MemorySummary) -> KinicRecord {
    let detail = parse_memory_detail(memory.detail.as_str());
    let metadata_name = parse_memory_metadata(memory.name.as_str());
    let resolved_name = detail
        .name
        .as_deref()
        .or(metadata_name
            .as_ref()
            .and_then(|value| value.name.as_deref()))
        .map(str::to_string);
    let resolved_description = detail
        .description
        .as_deref()
        .or(metadata_name
            .as_ref()
            .and_then(|value| value.description.as_deref()))
        .map(str::to_string);
    let users_section = render_memory_users_markdown(&memory.users);
    let dim = memory
        .dim
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let cycle_amount = memory
        .cycle_amount
        .map(format_cycle_amount)
        .unwrap_or_else(|| "unknown".to_string());
    let stable_memory_size = memory
        .stable_memory_size
        .map(format_count)
        .unwrap_or_else(|| "unknown".to_string());
    let description_line = resolved_description
        .as_deref()
        .map(|description| format_multiline_metadata_field("Description", description))
        .unwrap_or_default();
    let display_name = display_memory_name(memory.name.as_str(), resolved_name.as_deref());
    let summary = format!("Id: {}\nStatus: {}", memory.id, memory.status);
    KinicRecord::new(
        memory.id.clone(),
        display_name,
        "memories",
        summary,
        format!(
            "## Memory\n\n- Id: `{}`\n- Status: `{}`\n- Name: `{}`\n{}- Version: `{}`\n- Dimension: `{}`\n- Stable Memory Size: `{}`\n- Cycle Amount: `{}`\n\n### Search\nSelect this item, then type a query and press Enter in the search box.\n\n### Users\n{}\n",
            memory.id,
            memory.status,
            display_memory_name(memory.name.as_str(), resolved_name.as_deref()),
            description_line,
            memory.version,
            dim,
            stable_memory_size,
            cycle_amount,
            users_section
        ),
    )
    .with_searchable_memory_id_option(memory.searchable_memory_id)
}

fn display_memory_name(name: &str, detail_name: Option<&str>) -> String {
    if let Some(detail_name) = detail_name {
        return detail_name.to_string();
    }
    match name.trim() {
        "" | "unknown" => "unknown".to_string(),
        _ => name.to_string(),
    }
}

fn resolved_memory_name(name: &str, detail: &str) -> String {
    let detail_name = parse_memory_detail(detail).name;
    let metadata_name = parse_memory_metadata(name).and_then(|metadata| metadata.name);
    display_memory_name(name, detail_name.as_deref().or(metadata_name.as_deref()))
}

fn format_cycle_amount(value: u64) -> String {
    const TRILLION: u128 = 1_000_000_000_000;
    const SCALE: u128 = 1_000;

    let raw = value as u128;
    let mut whole = raw / TRILLION;
    let mut fractional = ((raw % TRILLION) * SCALE + (TRILLION / 2)) / TRILLION;
    if fractional == SCALE {
        whole += 1;
        fractional = 0;
    }

    format!("{whole}.{:03}T", fractional)
}

fn format_count(value: u32) -> String {
    let digits = value.to_string();
    let len = digits.len();
    let mut formatted = String::with_capacity(len + (len.saturating_sub(1) / 3));
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (len - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(ch);
    }
    formatted
}

fn format_multiline_metadata_field(label: &str, value: &str) -> String {
    let mut rendered = format!("- {label}:\n");
    for line in value.lines() {
        rendered.push_str("  ");
        rendered.push_str(line);
        rendered.push('\n');
    }
    rendered
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedMemoryDetail {
    name: Option<String>,
    description: Option<String>,
}

fn parse_memory_detail(detail: &str) -> ParsedMemoryDetail {
    let trimmed = detail.trim();
    if trimmed.is_empty() || is_memory_boilerplate_detail(trimmed) {
        return ParsedMemoryDetail {
            name: None,
            description: None,
        };
    }

    if let Some(metadata) = parse_memory_metadata(trimmed) {
        return ParsedMemoryDetail {
            name: metadata.name,
            description: metadata.description,
        };
    }

    ParsedMemoryDetail {
        name: None,
        description: None,
    }
}

fn is_memory_boilerplate_detail(detail: &str) -> bool {
    matches!(
        detail,
        "Memory is ready for search and writes." | "Launcher is setting up this memory."
    )
}

fn render_memory_users_markdown(users: &Option<Vec<bridge::MemoryUser>>) -> String {
    let Some(users) = users.as_ref() else {
        return "Users unavailable.".to_string();
    };
    if users.is_empty() {
        return "No users found.".to_string();
    }

    users
        .iter()
        .map(|user| format!("- User: `{}` | {}", user.principal_id, user.role))
        .collect::<Vec<_>>()
        .join("\n")
}

fn record_from_search_result(index: usize, item: SearchResultItem) -> KinicRecord {
    let parsed = parse_search_payload(&item.payload);
    let sentence = parsed
        .as_ref()
        .and_then(|payload| payload.sentence.as_deref())
        .map(decode_payload_text);
    let title = sentence
        .as_ref()
        .map(|text| search_result_list_title(text, index))
        .unwrap_or_else(|| search_result_title(&item.payload, index));
    let score = format!("{:.4}", item.score);
    let tag = parsed
        .as_ref()
        .and_then(|payload| payload.tag.as_deref())
        .unwrap_or("search-result");
    let detail_body = sentence.unwrap_or_else(|| item.payload.clone());
    KinicRecord::new(
        format!("{}-result-{}", item.memory_id, index + 1),
        title,
        "search-result",
        format!("Score: {score} | Tag: {tag}"),
        format!(
            "## Search Hit\n\n- Memory: `{}`\n- Score: `{score}`\n- Tag: `{tag}`\n\n### Sentence\n{}\n\n### Raw Payload\n{}\n",
            item.memory_id, detail_body, item.payload
        ),
    )
    .with_source_memory_id(item.memory_id)
}

fn search_result_title(payload: &str, index: usize) -> String {
    payload
        .lines()
        .map(clean_payload_line)
        .find(|line| !line.is_empty())
        .map(truncate_title)
        .unwrap_or_else(|| format!("Hit #{:02}", index + 1))
}

fn search_result_list_title(payload: &str, index: usize) -> String {
    let single_line = payload
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    search_result_title(&single_line, index)
}

fn clean_payload_line(line: &str) -> String {
    let trimmed = line.trim();
    let stripped = trimmed
        .trim_start_matches('#')
        .trim_start_matches('-')
        .trim_start_matches('*')
        .trim()
        .trim_matches('`')
        .trim();
    stripped.to_string()
}

fn truncate_title(mut title: String) -> String {
    const MAX_CHARS: usize = 72;
    if title.chars().count() <= MAX_CHARS {
        return title;
    }
    title = title.chars().take(MAX_CHARS - 1).collect::<String>();
    format!("{title}…")
}

fn parse_search_payload(payload: &str) -> Option<SearchPayload> {
    serde_json::from_str::<SearchPayload>(payload).ok()
}

fn decode_payload_text(text: &str) -> String {
    text.replace("\\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

fn has_same_session_context(
    current: &SessionSettingsSnapshot,
    next: &SessionSettingsSnapshot,
) -> bool {
    current.auth_mode == next.auth_mode
        && current.identity_name == next.identity_name
        && current.network == next.network
}

fn format_create_submit_error(error: &bridge::CreateMemoryError) -> String {
    match error {
        bridge::CreateMemoryError::Principal(reason) => {
            format!("Could not derive principal for create. Cause: {reason}")
        }
        bridge::CreateMemoryError::Balance(reason) => {
            format!("Could not fetch current balance. Cause: {reason}")
        }
        bridge::CreateMemoryError::Price(reason) => {
            format!("Could not fetch create price. Cause: {reason}")
        }
        bridge::CreateMemoryError::Fee(reason) => {
            format!("Could not fetch ledger fee. Cause: {reason}")
        }
        bridge::CreateMemoryError::InsufficientBalance {
            required_total_kinic,
            required_total_base_units,
            shortfall_kinic,
            shortfall_base_units,
        } => format!(
            "Insufficient balance. Need {required_total_kinic} KINIC ({required_total_base_units} e8s) total, short by {shortfall_kinic} KINIC ({shortfall_base_units} e8s)."
        ),
        bridge::CreateMemoryError::Approve(reason) => {
            format!("Approve step failed. Cause: {reason}")
        }
        bridge::CreateMemoryError::Deploy(reason) => {
            format!("Deploy step failed. Cause: {reason}")
        }
    }
}

fn format_insert_submit_error(error: &bridge::InsertMemoryError) -> String {
    match error {
        bridge::InsertMemoryError::ResolveAgentFactory(reason) => {
            format!("Could not resolve agent configuration. Cause: {reason}")
        }
        bridge::InsertMemoryError::BuildAgent(reason) => {
            format!("Could not build agent. Cause: {reason}")
        }
        bridge::InsertMemoryError::ParseMemoryId(reason) => {
            format!("Could not resolve memory canister. Cause: {reason}")
        }
        bridge::InsertMemoryError::Execute(reason) => {
            format!("Insert failed. Cause: {reason}")
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
