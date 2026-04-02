#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, mpsc},
    thread,
};

use super::adapter;
use super::bridge::{self, MemorySummary, SearchResultItem};
use super::settings::{self, PreferencesHealth, UserPreferences};
use crate::{
    create_domain::derive_create_cost,
    embedding::fetch_embedding,
    insert_service::{
        InsertRequest, parse_embedding_json, validate_insert_request_fields,
        validate_insert_request_for_submit,
    },
    tui::TuiAuth,
};
use ic_agent::export::Principal;
use serde::Deserialize;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tui_kit_runtime::{
    AccessControlAction, AccessControlMode, AccessControlRole, CoreAction, CoreEffect, CoreResult,
    CoreState, CreateCostState, DataProvider, FILE_MODE_ALLOWED_EXTENSIONS, InsertMode,
    LoadedCreateCost, PaneFocus, PickerConfirmKind, PickerContext, PickerItem, PickerItemKind,
    PickerListMode, PickerState, ProviderOutput, ProviderSnapshot, SearchScope,
    SessionAccountOverview, SessionSettingsSnapshot,
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

pub struct KinicProvider {
    all: Vec<KinicRecord>,
    query: String,
    tab_id: String,
    config: TuiConfig,
    session_overview: SessionAccountOverview,
    user_preferences: UserPreferences,
    preferences_health: PreferencesHealth,
    active_memory_id: Option<String>,
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
    pending_create_cost: Option<mpsc::Receiver<CreateCostTaskOutput>>,
    pending_create_cost_request_id: Option<u64>,
    create_cost_in_flight: bool,
    pending_create_submit: Option<mpsc::Receiver<CreateSubmitTaskOutput>>,
    pending_create_submit_request_id: Option<u64>,
    create_submit_in_flight: bool,
    pending_session_settings: Option<mpsc::Receiver<SessionSettingsTaskOutput>>,
    pending_session_settings_request_id: Option<u64>,
    session_settings_in_flight: bool,
    next_session_settings_request_id: u64,
    next_create_request_id: u64,
    pending_insert_submit: Option<mpsc::Receiver<InsertSubmitTaskOutput>>,
    pending_insert_submit_request_id: Option<u64>,
    insert_submit_in_flight: bool,
    next_insert_request_id: u64,
    insert_expected_dim_memory_id: Option<String>,
    insert_expected_dim: Option<u64>,
    insert_expected_dim_loading: bool,
    pending_insert_dim: Option<mpsc::Receiver<InsertDimTaskOutput>>,
    pending_access_submit: Option<mpsc::Receiver<AccessSubmitTaskOutput>>,
    access_submit_in_flight: bool,
    loaded_memory_details: HashSet<String>,
    pending_memory_detail: Option<mpsc::Receiver<MemoryDetailTaskOutput>>,
    pending_memory_detail_memory_id: Option<String>,
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

struct SessionSettingsTaskOutput {
    request_id: u64,
    overview: SessionAccountOverview,
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
    settings::normalize_saved_tags(preferences.saved_tags.clone())
}

fn add_action_label_for_context(context: PickerContext) -> Option<&'static str> {
    match context {
        PickerContext::InsertTag | PickerContext::TagManagement => Some("+ Add new tag"),
        PickerContext::DefaultMemory | PickerContext::InsertTarget | PickerContext::AddTag => None,
    }
}

fn picker_selected_id_for_context(context: PickerContext, state: &CoreState) -> Option<String> {
    match context {
        PickerContext::DefaultMemory => state.saved_default_memory_id.clone(),
        PickerContext::InsertTarget => {
            let insert_memory_id = state.insert_memory_id.trim();
            (!insert_memory_id.is_empty()).then(|| insert_memory_id.to_string())
        }
        PickerContext::InsertTag => {
            let insert_tag = state.insert_tag.trim();
            (!insert_tag.is_empty()).then(|| insert_tag.to_string())
        }
        PickerContext::TagManagement | PickerContext::AddTag => None,
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
        settings::load_user_preferences()
    }
}

fn save_user_preferences_for_apply(
    preferences: &UserPreferences,
) -> Result<(), tui_kit_host::settings::SettingsError> {
    #[cfg(test)]
    if take_test_settings_save_override().is_some() {
        return Err(tui_kit_host::settings::SettingsError::NoConfigDir);
    }

    settings::save_user_preferences(preferences)
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

struct InsertSubmitTaskOutput {
    request_id: u64,
    result: Result<bridge::InsertMemorySuccess, bridge::InsertMemoryError>,
}

fn insert_success_status(success: &bridge::InsertMemorySuccess) -> String {
    format!(
        "Inserted {} chunks (tag: {}) into {}",
        success.inserted_count, success.tag, success.memory_id
    )
}

struct InsertDimTaskOutput {
    memory_id: String,
    result: Result<u64, bridge::InsertMemoryError>,
}

struct MemoryDetailTaskOutput {
    memory_id: String,
    result: Result<bridge::MemoryDetails, String>,
}

struct AccessSubmitTaskOutput {
    memory_id: String,
    result: Result<(), String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AccessSelection<'a> {
    User(&'a bridge::MemoryUser),
    AddUser,
}
impl KinicProvider {
    pub fn new(config: TuiConfig) -> Self {
        #[cfg(test)]
        let _settings_io_lock = settings_io_lock()
            .lock()
            .expect("settings io lock should be available");
        let (user_preferences, preferences_health) = match settings::load_user_preferences() {
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
            active_memory_id: None,
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
            pending_create_cost: None,
            pending_create_cost_request_id: None,
            create_cost_in_flight: false,
            pending_create_submit: None,
            pending_create_submit_request_id: None,
            create_submit_in_flight: false,
            pending_session_settings: None,
            pending_session_settings_request_id: None,
            session_settings_in_flight: false,
            next_session_settings_request_id: 0,
            next_create_request_id: 0,
            pending_insert_submit: None,
            pending_insert_submit_request_id: None,
            insert_submit_in_flight: false,
            next_insert_request_id: 0,
            insert_expected_dim_memory_id: None,
            insert_expected_dim: None,
            insert_expected_dim_loading: false,
            pending_insert_dim: None,
            pending_access_submit: None,
            access_submit_in_flight: false,
            loaded_memory_details: HashSet::new(),
            pending_memory_detail: None,
            pending_memory_detail_memory_id: None,
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
        self.loaded_memory_details.clear();
        self.pending_memory_detail = None;
        self.pending_memory_detail_memory_id = None;
        self.active_memory_id = None;
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

        if self.memories_mode == MemoriesMode::Results || self.query.is_empty() {
            return base.iter().collect();
        }

        let q = self.query.to_lowercase();
        base.iter()
            .filter(|r| {
                r.title.to_lowercase().contains(&q)
                    || r.summary.to_lowercase().contains(&q)
                    || r.group.to_lowercase().contains(&q)
                    || r.id.to_lowercase().contains(&q)
            })
            .collect()
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

    fn sync_active_memory_to_visible_records(&mut self) {
        if self.memories_mode != MemoriesMode::Browser {
            return;
        }

        let previous_active_memory_id = self.active_memory_id.clone();
        if self.query.is_empty() {
            if self.active_memory_id.is_none() {
                self.active_memory_id = self.memory_records.first().map(|record| record.id.clone());
            }
            if self.active_memory_id != previous_active_memory_id {
                self.invalidate_pending_search();
                self.start_active_memory_detail_load();
            }
            return;
        }

        let visible_ids = self
            .visible_memory_records()
            .into_iter()
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();

        if visible_ids.is_empty() {
            self.active_memory_id = None;
            if self.active_memory_id != previous_active_memory_id {
                self.invalidate_pending_search();
            }
            return;
        }

        if self
            .active_memory_id
            .as_ref()
            .is_some_and(|active_id| visible_ids.iter().any(|id| id == active_id))
        {
            return;
        }

        self.active_memory_id = visible_ids.first().cloned();
        self.invalidate_pending_search();
        self.start_active_memory_detail_load();
    }

    fn active_visible_memory_record(&self) -> Option<&KinicRecord> {
        let active_id = self.active_memory_id.as_deref()?;
        self.visible_memory_records()
            .into_iter()
            .find(|record| record.id == active_id)
    }

    fn active_visible_memory_index(&self) -> Option<usize> {
        let active_id = self.active_memory_id.as_deref()?;
        self.visible_memory_records()
            .into_iter()
            .position(|record| record.id == active_id)
    }

    fn visible_memory_count(&self) -> usize {
        self.visible_memory_records().len()
    }

    fn move_active_memory(&mut self, delta: isize) {
        if self.memories_mode != MemoriesMode::Browser || self.visible_memory_count() == 0 {
            return;
        }

        let visible_records = self.visible_memory_records();
        let current = self.active_visible_memory_index().unwrap_or(0) as isize;
        let last = visible_records.len().saturating_sub(1) as isize;
        let next = (current + delta).clamp(0, last) as usize;
        self.active_memory_id = Some(visible_records[next].id.clone());
        self.invalidate_pending_search();
        self.start_active_memory_detail_load();
    }

    fn set_active_memory(&mut self, index: usize) {
        if self.memories_mode != MemoriesMode::Browser {
            return;
        }
        let visible_records = self.visible_memory_records();
        let Some(record) = visible_records.get(index) else {
            return;
        };
        self.active_memory_id = Some(record.id.clone());
        self.invalidate_pending_search();
        self.start_active_memory_detail_load();
    }

    fn active_memory_summary(&self) -> Option<&MemorySummary> {
        let active_id = self.active_memory_id.as_deref()?;
        self.memory_summaries
            .iter()
            .find(|summary| summary.id == active_id)
    }

    fn access_selection<'a>(&'a self, state: &CoreState) -> Option<AccessSelection<'a>> {
        let summary = self.active_memory_summary()?;
        let user_count = summary.users.as_ref().map_or(0, Vec::len);
        if state.access_list_index >= user_count {
            return Some(AccessSelection::AddUser);
        }
        summary
            .users
            .as_ref()
            .and_then(|users| users.get(state.access_list_index))
            .map(AccessSelection::User)
    }

    fn next_access_index(&self, state: &CoreState, delta: isize) -> usize {
        let selectable_len = self
            .active_memory_summary()
            .map(|summary| summary.users.as_ref().map_or(0, Vec::len) + 1)
            .unwrap_or(1);
        let current = state
            .access_list_index
            .min(selectable_len.saturating_sub(1)) as isize;
        (current + delta).clamp(0, selectable_len.saturating_sub(1) as isize) as usize
    }

    fn access_role_from_user(user: &bridge::MemoryUser) -> AccessControlRole {
        match user.role.as_str() {
            "admin" => AccessControlRole::Admin,
            "writer" => AccessControlRole::Writer,
            _ => AccessControlRole::Reader,
        }
    }

    fn apply_access_content(&self, content: &mut tui_kit_model::UiItemContent, state: &CoreState) {
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
        section.body_lines = render_access_lines(users, state.access_list_index);
    }

    fn selected_insert_memory_id(&self, state: &CoreState) -> Option<String> {
        let insert_memory_id = state.insert_memory_id.trim();
        if !insert_memory_id.is_empty() {
            return Some(insert_memory_id.to_string());
        }

        let default_memory_id = self.user_preferences.default_memory_id.as_deref()?;
        self.memory_records
            .iter()
            .find(|record| record.id == default_memory_id)
            .map(|record| record.id.clone())
    }

    fn reset_insert_dim(&mut self) {
        self.insert_expected_dim_memory_id = None;
        self.insert_expected_dim = None;
        self.insert_expected_dim_loading = false;
        self.pending_insert_dim = None;
    }

    fn start_insert_dim_load(&mut self, state: &CoreState) {
        let Some(memory_id) = self.selected_insert_memory_id(state) else {
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
        self.memory_records = self
            .memory_summaries
            .iter()
            .cloned()
            .map(record_from_memory_summary)
            .collect();
        self.all = self.memory_records.clone();
    }

    fn start_active_memory_detail_load(&mut self) {
        if self.memories_mode != MemoriesMode::Browser {
            return;
        }
        let Some(memory_id) = self.active_memory_id.clone() else {
            return;
        };
        if self.loaded_memory_details.contains(memory_id.as_str()) {
            return;
        }
        if self.pending_memory_detail_memory_id.as_deref() == Some(memory_id.as_str()) {
            return;
        }

        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let (tx, rx) = mpsc::channel();
        self.pending_memory_detail = Some(rx);
        self.pending_memory_detail_memory_id = Some(memory_id.clone());

        thread::spawn(move || {
            let runtime =
                Runtime::new().expect("failed to create tokio runtime for memory detail load");
            let result = runtime
                .block_on(bridge::load_memory_details(
                    use_mainnet,
                    auth,
                    memory_id.clone(),
                ))
                .map_err(|error| error.to_string());
            let _ = tx.send(MemoryDetailTaskOutput { memory_id, result });
        });
    }

    fn should_handle_memory_navigation(&self, state: &CoreState) -> bool {
        state.current_tab_id == KINIC_MEMORIES_TAB_ID
            && self.tab_id == KINIC_MEMORIES_TAB_ID
            && self.memories_mode == MemoriesMode::Browser
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.current_records();
        let default_memory = self.default_memory_selection();
        let default_memory_items = default_memory.available_memory_ids();
        let default_memory_labels = default_memory.selector_labels();
        let insert_memory_placeholder = self.insert_memory_placeholder_label(state);
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
                let preferred_selected_id = selected_id
                    .clone()
                    .or_else(|| picker_selected_id_for_context(*context, state));
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
        let items = filtered
            .iter()
            .map(|record| {
                let mut summary = adapter::to_summary(record);
                if record.group == "memories" && default_memory.is_default_memory(&record.id) {
                    summary.leading_marker = Some("★".to_string());
                }
                summary
            })
            .collect::<Vec<_>>();
        let selected_content = if state.current_tab_id == KINIC_SETTINGS_TAB_ID {
            None
        } else if self.memories_mode == MemoriesMode::Browser {
            if self.memory_records.is_empty() {
                filtered.first().copied().map(adapter::to_content)
            } else {
                self.active_visible_memory_record().map(|record| {
                    let mut content = adapter::to_content(record);
                    if record.group == "memories" {
                        self.apply_access_content(&mut content, state);
                    }
                    content
                })
            }
        } else {
            let sel = state.selected_index.unwrap_or(0);
            filtered.get(sel).map(|r| adapter::to_content(r))
        };
        let insert_current_dim = self.insert_current_dim(state);
        let insert_validation_message = self.insert_validation_message(state);

        ProviderSnapshot {
            items,
            selected_content,
            selected_context: None,
            total_count: filtered.len(),
            status_message: Some(self.status_message(state, filtered.len())),
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
                .selected_insert_memory_id(state)
                .filter(|memory_id| {
                    self.insert_expected_dim_memory_id.as_deref() == Some(memory_id.as_str())
                })
                .and(self.insert_expected_dim),
            insert_expected_dim_loading: self.insert_expected_dim_loading
                && self
                    .selected_insert_memory_id(state)
                    .is_some_and(|memory_id| {
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
            || self.insert_expected_dim_loading
            || state.insert_embedding.trim().is_empty()
        {
            return None;
        }

        let selected_memory_id = self.selected_insert_memory_id(state)?;
        if self.insert_expected_dim_memory_id.as_deref() != Some(selected_memory_id.as_str()) {
            return None;
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

    fn insert_memory_placeholder_label(&self, state: &CoreState) -> Option<String> {
        if !state.insert_memory_id.trim().is_empty() {
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
        updated_preferences.saved_tags =
            settings::normalize_saved_tags(updated_preferences.saved_tags);

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
        updated_preferences.saved_tags =
            settings::normalize_saved_tags(updated_preferences.saved_tags);

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
            PickerContext::DefaultMemory => vec![
                self.default_memory_controller()
                    .set_default_memory_preference(item.id.clone()),
            ],
            PickerContext::InsertTarget => vec![
                CoreEffect::SetInsertMemoryId(item.id.clone()),
                CoreEffect::Notify(format!("Selected target memory {}", item.id)),
            ],
            PickerContext::InsertTag => Vec::new(),
            PickerContext::TagManagement => vec![
                CoreEffect::SetInsertTag(item.id.clone()),
                CoreEffect::Notify(format!("Selected tag {} for insert", item.id)),
            ],
            PickerContext::AddTag => Vec::new(),
        }
    }

    fn start_session_settings_refresh(&mut self) -> Option<CoreEffect> {
        if self.session_settings_in_flight {
            return None;
        }

        let request_id = self.next_session_settings_request_id;
        self.next_session_settings_request_id += 1;
        self.pending_session_settings_request_id = Some(request_id);
        self.session_settings_in_flight = true;
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let (tx, rx) = mpsc::channel();
        self.pending_session_settings = Some(rx);

        thread::spawn(move || {
            let runtime =
                Runtime::new().expect("failed to create tokio runtime for settings refresh");
            let overview =
                runtime.block_on(bridge::load_session_account_overview(use_mainnet, auth));
            let _ = tx.send(SessionSettingsTaskOutput {
                request_id,
                overview,
            });
        });

        None
    }

    fn start_create_cost_refresh(&mut self) -> Option<CoreEffect> {
        if self.create_cost_in_flight {
            return None;
        }

        let request_id = self.next_create_request_id;
        self.next_create_request_id += 1;
        self.pending_create_cost_request_id = Some(request_id);
        self.create_cost_state = CreateCostState::Loading;
        self.create_cost_in_flight = true;
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let (tx, rx) = mpsc::channel();
        self.pending_create_cost = Some(rx);

        thread::spawn(move || {
            let runtime =
                Runtime::new().expect("failed to create tokio runtime for create cost refresh");
            let overview =
                runtime.block_on(bridge::load_session_account_overview(use_mainnet, auth));
            let _ = tx.send(CreateCostTaskOutput {
                request_id,
                overview,
            });
        });

        None
    }

    fn start_create_submit(&mut self, name: String, description: String) -> CoreEffect {
        let request_id = self.next_create_request_id;
        self.next_create_request_id += 1;
        self.pending_create_submit_request_id = Some(request_id);
        self.create_submit_in_flight = true;
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let (tx, rx) = mpsc::channel();
        self.pending_create_submit = Some(rx);

        thread::spawn(move || {
            let runtime = Runtime::new().expect("failed to create tokio runtime for create submit");
            let result =
                runtime.block_on(bridge::create_memory(use_mainnet, auth, name, description));
            let _ = tx.send(CreateSubmitTaskOutput { request_id, result });
        });

        CoreEffect::Notify("Creating memory...".to_string())
    }

    fn start_insert_submit(&mut self, request: InsertRequest) -> CoreEffect {
        let request_id = self.next_insert_request_id;
        self.next_insert_request_id += 1;
        self.pending_insert_submit_request_id = Some(request_id);
        self.insert_submit_in_flight = true;
        let auth = self.config.auth.clone();
        let use_mainnet = self.config.use_mainnet;
        let (tx, rx) = mpsc::channel();
        self.pending_insert_submit = Some(rx);

        thread::spawn(move || {
            let runtime = Runtime::new().expect("failed to create tokio runtime for insert submit");
            let result = runtime.block_on(bridge::run_insert(use_mainnet, auth, request));
            let _ = tx.send(InsertSubmitTaskOutput { request_id, result });
        });

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

    fn build_insert_request(&self, state: &CoreState) -> InsertRequest {
        let memory_id = state.insert_memory_id.trim().to_string();
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
            MemoriesMode::Browser => {
                let scope = match state.search_scope {
                    SearchScope::All => "all memories",
                    SearchScope::Selected => "selected memory",
                };
                match self.active_memory_id.as_deref() {
                    Some(memory_id) => format!("Target {memory_id} | Search scope {scope}"),
                    None => format!("Search scope {scope}"),
                }
            }
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

    fn invalidate_pending_create_cost(&mut self) {
        self.pending_create_cost_request_id = None;
    }

    fn invalidate_pending_create_submit(&mut self) {
        self.pending_create_submit_request_id = None;
    }

    fn invalidate_pending_session_settings(&mut self) {
        self.pending_session_settings_request_id = None;
    }

    fn invalidate_pending_insert_submit(&mut self) {
        self.pending_insert_submit_request_id = None;
    }

    fn validate_insert_expected_dim(
        &self,
        request: &InsertRequest,
        state: &CoreState,
    ) -> Result<(), String> {
        let InsertRequest::Raw { embedding_json, .. } = request else {
            return Ok(());
        };
        let Some(selected_memory_id) = self.selected_insert_memory_id(state) else {
            return Ok(());
        };
        if self.insert_expected_dim_memory_id.as_deref() != Some(selected_memory_id.as_str()) {
            return Ok(());
        }
        let Some(expected_dim) = self.insert_expected_dim else {
            return Ok(());
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
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }
        self.insert_expected_dim_loading = false;

        match output.result {
            Ok(dim) => {
                self.insert_expected_dim = Some(dim);
            }
            Err(_) => {
                self.insert_expected_dim_memory_id = None;
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
                if self.active_memory_id.as_deref() == Some(output.memory_id.as_str()) {
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

    fn poll_memory_detail_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_memory_detail.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_memory_detail = None;
                self.pending_memory_detail_memory_id = None;
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: Vec::new(),
                });
            }
        };

        self.pending_memory_detail = None;
        let is_current =
            self.pending_memory_detail_memory_id.as_deref() == Some(output.memory_id.as_str());
        self.pending_memory_detail_memory_id = None;
        if !is_current {
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }

        if let Ok(details) = output.result
            && let Some(summary) = self
                .memory_summaries
                .iter_mut()
                .find(|summary| summary.id == output.memory_id)
        {
            summary.name = details.name;
            summary.detail = details.content_preview;
            summary.version = details.version;
            summary.dim = details.dim;
            summary.owners = Some(details.owners);
            summary.stable_memory_size = details.stable_memory_size;
            summary.cycle_amount = details.cycle_amount;
            summary.users = Some(details.users);
            self.loaded_memory_details.insert(output.memory_id);
            self.refresh_memory_records_from_summaries();
        }

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects: Vec::new(),
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
            price_base_units: overview.price_base_units,
            principal_error: overview.principal_error,
            balance_error: overview.balance_error,
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
        let memory_id = state.access_control_memory_id.trim();
        if memory_id.is_empty() {
            return Err("Select a memory before managing access.".to_string());
        }

        let principal_id = state.access_control_principal_id.trim();
        if principal_id.is_empty() {
            return Err("Principal ID is required.".to_string());
        }

        if principal_id == crate::clients::LAUNCHER_CANISTER {
            return Err("Launcher canister access cannot be modified.".to_string());
        }

        if principal_id != "anonymous" {
            Principal::from_text(principal_id)
                .map_err(|_| format!("Invalid principal text: {principal_id}"))?;
        }

        let action = state.access_control_action;
        let role = state.access_control_role;
        if action != AccessControlAction::Remove
            && role == AccessControlRole::Admin
            && principal_id == "anonymous"
        {
            return Err("cannot grant admin role to anonymous".to_string());
        }

        Ok((
            memory_id.to_string(),
            action,
            principal_id.to_string(),
            role,
        ))
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

    fn selected_memory_id_for_default(&self, state: &CoreState) -> Option<String> {
        match self.memories_mode {
            MemoriesMode::Browser => self.active_memory_id.clone(),
            MemoriesMode::Results => self
                .result_records
                .get(state.selected_index.unwrap_or(0))
                .and_then(|record| record.source_memory_id.clone()),
        }
    }

    fn search_target_memory_ids(&self, scope: SearchScope) -> Result<Vec<String>, String> {
        match scope {
            SearchScope::All => {
                let targets = self
                    .memory_records
                    .iter()
                    .filter_map(|record| record.searchable_memory_id.clone())
                    .collect::<Vec<_>>();
                if targets.is_empty() {
                    Err("No searchable memories are available yet.".to_string())
                } else {
                    Ok(targets)
                }
            }
            SearchScope::Selected => {
                let active_memory_id = self.active_memory_id.as_deref().ok_or_else(|| {
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

                let mut items = Vec::new();
                let mut failed_memory_ids = Vec::new();
                let mut first_error = None;
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
                        Ok((_, Ok(mut search_items))) => items.append(&mut search_items),
                        Ok((memory_id, Err(error))) => {
                            if first_error.is_none() {
                                first_error = Some(error.to_string());
                            }
                            failed_memory_ids.push(memory_id);
                        }
                        Err(error) => {
                            if first_error.is_none() {
                                first_error = Some(error.to_string());
                            }
                        }
                    }
                }

                items.sort_by(|left, right| {
                    right
                        .score
                        .partial_cmp(&left.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                if failed_memory_ids.len() == target_memory_ids.len() {
                    Some(Err(first_error.unwrap_or_else(|| {
                        "Search failed before any memory returned results.".to_string()
                    })))
                } else {
                    Some(Ok(SearchBatchResult {
                        items,
                        failed_memory_ids,
                    }))
                }
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
                let failed_count = results.failed_memory_ids.len();
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
                self.memory_summaries = memories;
                self.refresh_memory_records_from_summaries();
                self.active_memory_id = self
                    .default_memory_selection()
                    .preferred_initial_memory_id()
                    .or_else(|| self.memory_records.first().map(|record| record.id.clone()));
                self.last_search_state = None;
                self.start_active_memory_detail_load();
                self.start_insert_dim_load(state);
                Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: Vec::new(),
                })
            }
            Err(error) => {
                self.memory_records.clear();
                self.result_records.clear();
                self.memories_mode = MemoriesMode::Browser;
                self.all = vec![load_error_record(error)];
                self.active_memory_id = None;
                self.last_search_state = None;
                Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::Notify("Unable to load memories.".to_string())],
                })
            }
        }
    }

    fn poll_create_cost_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_create_cost.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_create_cost = None;
                self.invalidate_pending_create_cost();
                self.create_cost_in_flight = false;
                self.create_cost_state = CreateCostState::Error(vec![
                    "Account info refresh worker disconnected.".to_string(),
                ]);
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::Notify(
                        "Account info refresh failed unexpectedly.".to_string(),
                    )],
                });
            }
        };

        self.pending_create_cost = None;
        self.create_cost_in_flight = false;
        let is_current = self.pending_create_cost_request_id == Some(output.request_id);
        self.invalidate_pending_create_cost();
        if !is_current {
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }

        let issues = output.overview.account_issue_messages();
        let details = derive_create_cost(
            output.overview.session.principal_id.as_str(),
            output.overview.balance_base_units,
            output.overview.price_base_units.as_ref(),
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

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects: Vec::new(),
        })
    }

    fn poll_create_submit_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_create_submit.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_create_submit = None;
                self.invalidate_pending_create_submit();
                self.create_submit_in_flight = false;
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::CreateFormError(Some(
                        "Create request failed: background worker disconnected.".to_string(),
                    ))],
                });
            }
        };

        self.pending_create_submit = None;
        self.create_submit_in_flight = false;
        let is_current = self.pending_create_submit_request_id == Some(output.request_id);
        self.invalidate_pending_create_submit();
        if !is_current {
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }

        let mut effects = Vec::new();
        match output.result {
            Ok(success) => {
                self.active_memory_id = Some(success.id.clone());
                if let Some(memories) = success.memories {
                    self.memory_summaries = memories;
                    self.loaded_memory_details.clear();
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
                self.memories_mode = MemoriesMode::Browser;
                self.result_records.clear();
                self.invalidate_pending_search();
                self.last_search_state = None;
                self.start_active_memory_detail_load();
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

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }

    fn poll_session_settings_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_session_settings.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_session_settings = None;
                self.invalidate_pending_session_settings();
                self.session_settings_in_flight = false;
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::Notify(
                        "Session settings refresh failed unexpectedly.".to_string(),
                    )],
                });
            }
        };

        self.pending_session_settings = None;
        self.session_settings_in_flight = false;
        let is_current = self.pending_session_settings_request_id == Some(output.request_id);
        self.invalidate_pending_session_settings();
        if !is_current {
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }

        let failure_message = output.overview.session_settings_refresh_failure_message();
        let notify_message = output.overview.session_settings_refresh_notify_message();
        self.apply_session_overview(output.overview);
        let effects = failure_message
            .or_else(|| (notify_message != "Session settings refreshed.").then_some(notify_message))
            .map(CoreEffect::Notify)
            .into_iter()
            .collect();

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }

    fn poll_insert_submit_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_insert_submit.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_insert_submit = None;
                self.invalidate_pending_insert_submit();
                self.insert_submit_in_flight = false;
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::InsertFormError(Some(
                        "Insert request failed: background worker disconnected.".to_string(),
                    ))],
                });
            }
        };

        self.pending_insert_submit = None;
        self.insert_submit_in_flight = false;
        let is_current = self.pending_insert_submit_request_id == Some(output.request_id);
        self.invalidate_pending_insert_submit();
        if !is_current {
            return Some(ProviderOutput {
                snapshot: Some(self.build_snapshot(state)),
                effects: Vec::new(),
            });
        }

        let effects = match output.result {
            Ok(success) => vec![
                CoreEffect::InsertFormError(None),
                CoreEffect::ResetInsertFormForRepeat,
                CoreEffect::NotifyPersistent(insert_success_status(&success)),
            ],
            Err(error) => vec![CoreEffect::InsertFormError(Some(
                format_insert_submit_error(&error),
            ))],
        };

        Some(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
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
        let mut effects = Vec::new();
        let mut close_picker_after_submit = false;
        match action {
            CoreAction::SetQuery(q) => {
                self.query = q.clone();
                self.sync_active_memory_to_visible_records();
                self.invalidate_pending_search();
                if self.tab_id == KINIC_MEMORIES_TAB_ID && q.is_empty() {
                    self.reset_memories_browser();
                }
            }
            CoreAction::SearchInput(c) => {
                self.query.push(*c);
                self.sync_active_memory_to_visible_records();
                self.invalidate_pending_search();
            }
            CoreAction::SearchBackspace => {
                self.query.pop();
                self.sync_active_memory_to_visible_records();
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
            CoreAction::SearchSubmit => {
                if let Some(effect) = self.run_live_search(state.search_scope) {
                    effects.push(effect);
                }
            }
            CoreAction::MoveNext if self.should_handle_memory_navigation(state) => {
                self.move_active_memory(1)
            }
            CoreAction::MovePrev if self.should_handle_memory_navigation(state) => {
                self.move_active_memory(-1)
            }
            CoreAction::MoveHome if self.should_handle_memory_navigation(state) => {
                self.set_active_memory(0)
            }
            CoreAction::MoveEnd if self.should_handle_memory_navigation(state) => {
                let visible_count = self.visible_memory_count();
                if visible_count != 0 {
                    self.set_active_memory(visible_count - 1);
                }
            }
            CoreAction::MovePageDown if self.should_handle_memory_navigation(state) => {
                self.move_active_memory(10)
            }
            CoreAction::MovePageUp if self.should_handle_memory_navigation(state) => {
                self.move_active_memory(-10)
            }
            CoreAction::SetTab(id) => {
                effects.extend(self.set_tab(id.0.as_str()));
                if id.0.as_str() == KINIC_INSERT_TAB_ID {
                    self.start_insert_dim_load(state);
                }
            }
            CoreAction::ChatSubmit => {
                effects.push(CoreEffect::Notify(
                    "Chat is not implemented yet; search is available first.".to_string(),
                ));
            }
            CoreAction::CreateSubmit => {
                let name = state.create_name.trim().to_string();
                let description = state.create_description.trim().to_string();
                if name.is_empty() || description.is_empty() {
                    effects.push(CoreEffect::CreateFormError(Some(
                        "Name and description are required.".to_string(),
                    )));
                } else if self.create_submit_in_flight {
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
                } else if let Err(error) = self.validate_insert_expected_dim(&request, state) {
                    effects.push(CoreEffect::InsertFormError(Some(error)));
                } else if self.insert_submit_in_flight {
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
            CoreAction::AccessMoveNext => {
                let next_index = self.next_access_index(state, 1);
                effects.push(CoreEffect::SetAccessListIndex(next_index));
                let mut preview_state = state.clone();
                preview_state.access_list_index = next_index;
                return Ok(ProviderOutput {
                    snapshot: Some(self.build_snapshot(&preview_state)),
                    effects,
                });
            }
            CoreAction::AccessMovePrev => {
                let next_index = self.next_access_index(state, -1);
                effects.push(CoreEffect::SetAccessListIndex(next_index));
                let mut preview_state = state.clone();
                preview_state.access_list_index = next_index;
                return Ok(ProviderOutput {
                    snapshot: Some(self.build_snapshot(&preview_state)),
                    effects,
                });
            }
            CoreAction::AccessOpenSelected => {
                let Some(memory_id) = self.active_memory_id.clone() else {
                    effects.push(CoreEffect::Notify(
                        "Select a memory before managing access.".to_string(),
                    ));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                };
                match self.access_selection(state) {
                    Some(AccessSelection::User(user)) => {
                        effects.push(CoreEffect::OpenAccessAction {
                            memory_id,
                            principal_id: user.principal_id.clone(),
                            role: Self::access_role_from_user(user),
                        });
                    }
                    Some(AccessSelection::AddUser) | None => {
                        effects.push(CoreEffect::OpenAccessAdd { memory_id });
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
                    match state.access_control_mode {
                        AccessControlMode::Action => {
                            effects.push(CoreEffect::OpenAccessConfirm {
                                memory_id: state.access_control_memory_id.clone(),
                                principal_id: state.access_control_principal_id.clone(),
                                action: state.access_control_action,
                                role: state.access_control_role,
                            });
                        }
                        AccessControlMode::Add | AccessControlMode::Confirm => {
                            if state.access_control_mode == AccessControlMode::Confirm
                                && !state.access_control_confirm_yes
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
                let Some(memory_id) = self.selected_memory_id_for_default(state) else {
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

        Ok(ProviderOutput {
            snapshot: Some(snapshot),
            effects,
        })
    }

    fn poll_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        self.poll_initial_memories_background(state)
            .or_else(|| self.poll_create_submit_background(state))
            .or_else(|| self.poll_access_submit_background(state))
            .or_else(|| self.poll_insert_dim_background(state))
            .or_else(|| self.poll_memory_detail_background(state))
            .or_else(|| self.poll_insert_submit_background(state))
            .or_else(|| self.poll_create_cost_background(state))
            .or_else(|| self.poll_session_settings_background(state))
            .or_else(|| self.poll_search_background(state))
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

fn render_access_lines(
    users: Option<&Vec<bridge::MemoryUser>>,
    selected_index: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let user_list = users.cloned().unwrap_or_default();

    match users {
        None => lines.push("unavailable".to_string()),
        Some(users) if users.is_empty() => lines.push("none".to_string()),
        Some(_) => {}
    }

    for (index, user) in user_list.iter().enumerate() {
        let marker = if selected_index.min(user_list.len()) == index {
            ">"
        } else {
            " "
        };
        lines.push(format!(
            "{marker} {}   {}",
            adapter::short_id(user.principal_id.as_str()),
            user.role
        ));
    }

    let add_marker = if selected_index.min(user_list.len()) == user_list.len() {
        ">"
    } else {
        " "
    };
    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines.push(format!("{add_marker} + Add User"));
    lines
}

fn short_error(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_string()
}

fn load_error_record(error: String) -> KinicRecord {
    KinicRecord::new(
        "kinic-live-error",
        "Unable to load memories",
        "memories",
        "Check your identity or network configuration.",
        format!("## Live Load Error\n\n{error}"),
    )
}

fn record_from_memory_summary(memory: MemorySummary) -> KinicRecord {
    let detail = parse_memory_detail(memory.detail.as_str());
    let metadata_name = parse_detail_object(memory.name.as_str()).unwrap_or((None, None));
    let resolved_name = detail
        .name
        .as_deref()
        .or(metadata_name.0.as_deref())
        .map(str::to_string);
    let resolved_description = detail
        .description
        .as_deref()
        .or(metadata_name.1.as_deref())
        .map(str::to_string);
    let users_section = render_memory_users_markdown(&memory.users);
    let dim = memory
        .dim
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let owners = memory
        .owners
        .as_ref()
        .map(|owners| format_owner_list(owners))
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
        .map(|description| format!("- Description: `{description}`\n"))
        .unwrap_or_default();
    KinicRecord::new(
        memory.id.clone(),
        memory.id.clone(),
        "memories",
        format!("Status: {}", memory.status),
        format!(
            "## Memory\n\n- Id: `{}`\n- Status: `{}`\n- Name: `{}`\n{}- Version: `{}`\n- Owners: `{}`\n- Dimension: `{}`\n- Stable Memory Size: `{}`\n- Cycle Amount: `{}`\n\n### Content\n{}\n\n### Search\nSelect this item, then type a query and press Enter in the search box.\n\n### Users\n{}\n",
            memory.id,
            memory.status,
            display_memory_name(memory.name.as_str(), resolved_name.as_deref()),
            description_line,
            memory.version,
            owners,
            dim,
            stable_memory_size,
            cycle_amount,
            detail.content,
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

fn format_owner_list(owners: &[String]) -> String {
    if owners.is_empty() {
        return "none".to_string();
    }

    owners.join(", ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedMemoryDetail {
    name: Option<String>,
    description: Option<String>,
    content: String,
}

fn parse_memory_detail(detail: &str) -> ParsedMemoryDetail {
    let trimmed = detail.trim();
    if trimmed.is_empty() || is_memory_boilerplate_detail(trimmed) {
        return ParsedMemoryDetail {
            name: None,
            description: None,
            content: "No additional content available.".to_string(),
        };
    }

    if let Some((name, description)) = parse_detail_object(trimmed) {
        return ParsedMemoryDetail {
            name,
            description,
            content: "No additional content available.".to_string(),
        };
    }

    ParsedMemoryDetail {
        name: None,
        description: None,
        content: trimmed.to_string(),
    }
}

fn is_memory_boilerplate_detail(detail: &str) -> bool {
    matches!(
        detail,
        "Memory is ready for search and writes." | "Launcher is setting up this memory."
    )
}

fn parse_detail_object(detail: &str) -> Option<(Option<String>, Option<String>)> {
    parse_detail_json_object(detail)
        .or_else(|| parse_detail_jsonish_object(detail))
        .and_then(|value| {
            let name = value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let description = value
                .get("description")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            (name.is_some() || description.is_some()).then_some((name, description))
        })
}

fn parse_detail_json_object(detail: &str) -> Option<serde_json::Value> {
    let value = serde_json::from_str::<serde_json::Value>(detail).ok()?;
    value.is_object().then_some(value)
}

fn parse_detail_jsonish_object(detail: &str) -> Option<serde_json::Value> {
    let name = extract_jsonish_field(detail, "name");
    let description = extract_jsonish_field(detail, "description");

    if name.is_none() && description.is_none() {
        return None;
    }

    let mut object = serde_json::Map::new();
    if let Some(name) = name {
        object.insert("name".to_string(), serde_json::Value::String(name));
    }
    if let Some(description) = description {
        object.insert(
            "description".to_string(),
            serde_json::Value::String(description),
        );
    }
    Some(serde_json::Value::Object(object))
}

fn extract_jsonish_field(detail: &str, key: &str) -> Option<String> {
    let patterns = [format!("\"{key}\":\""), format!("{key}\":\"")];
    for pattern in patterns {
        let Some(found_at) = detail.find(&pattern) else {
            continue;
        };
        let start = found_at + pattern.len();
        let tail = &detail[start..];
        let Some(end) = tail.find('"') else {
            continue;
        };
        let value = tail[..end].trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
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
