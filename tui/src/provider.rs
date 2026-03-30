#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::{sync::mpsc, thread};

use super::adapter;
use super::bridge::{self, MemorySummary, SearchResultItem};
use super::settings::{self, PreferencesHealth, UserPreferences};
use crate::{
    create_domain::derive_create_cost,
    insert_service::{InsertRequest, validate_insert_request},
    tui::TuiAuth,
};
use serde::Deserialize;
use tokio::runtime::Runtime;
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreResult, CoreState, CreateCostState, DataProvider, InsertMode,
    LoadedCreateCost, MemorySelectorContext, PaneFocus, ProviderOutput, ProviderSnapshot,
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
        }
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

pub struct KinicProvider {
    all: Vec<KinicRecord>,
    query: String,
    tab_id: String,
    config: TuiConfig,
    session_overview: SessionAccountOverview,
    user_preferences: UserPreferences,
    preferences_health: PreferencesHealth,
    active_memory_id: Option<String>,
    memory_records: Vec<KinicRecord>,
    result_records: Vec<KinicRecord>,
    memories_mode: MemoriesMode,
    pending_initial_memories: Option<mpsc::Receiver<InitialMemoriesTaskOutput>>,
    initial_memories_in_flight: bool,
    pending_search: Option<mpsc::Receiver<SearchTaskOutput>>,
    pending_search_context: Option<SearchRequestContext>,
    next_search_request_id: u64,
    search_in_flight: bool,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchRequestContext {
    request_id: u64,
    memory_id: String,
    query: String,
}

struct SearchTaskOutput {
    request_id: u64,
    memory_id: String,
    query: String,
    result: Result<Vec<SearchResultItem>, String>,
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

    fn preferred_initial_memory_id(self) -> Option<String> {
        let default_memory_id = self.user_preferences.default_memory_id.as_deref()?;
        self.memory_records
            .iter()
            .find(|record| record.id == default_memory_id)
            .map(|record| record.id.clone())
    }

    fn selected_default_memory_id(self) -> Option<String> {
        self.user_preferences.default_memory_id.clone()
    }

    fn is_default_memory(self, memory_id: &str) -> bool {
        self.user_preferences.default_memory_id.as_deref() == Some(memory_id)
    }

    fn label_for_id(self, memory_id: &str) -> Option<String> {
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

struct DefaultMemoryController<'a> {
    is_live: bool,
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
        if !self.is_live {
            return CoreEffect::Notify(
                "Default memory is only available in live mode.".to_string(),
            );
        }
        if self.user_preferences.default_memory_id.as_deref() == Some(memory_id.as_str()) {
            return CoreEffect::Notify(format!("Default memory already set to {memory_id}"));
        }

        let updated_preferences = UserPreferences {
            default_memory_id: Some(memory_id.clone()),
        };
        #[cfg(test)]
        let _settings_io_lock = settings_io_lock()
            .lock()
            .expect("settings io lock should be available");
        match settings::save_user_preferences(&updated_preferences) {
            Ok(()) => {
                #[cfg(test)]
                let reloaded_preferences = Ok(updated_preferences.clone());
                #[cfg(not(test))]
                let reloaded_preferences = settings::load_user_preferences();
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

#[cfg(test)]
fn settings_io_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct InsertSubmitTaskOutput {
    request_id: u64,
    result: Result<bridge::InsertMemorySuccess, bridge::InsertMemoryError>,
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
            all: sample_records(),
            query: String::new(),
            tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            config,
            session_overview,
            user_preferences,
            preferences_health,
            active_memory_id: None,
            memory_records: Vec::new(),
            result_records: Vec::new(),
            memories_mode: MemoriesMode::Browser,
            pending_initial_memories: None,
            initial_memories_in_flight: false,
            pending_search: None,
            pending_search_context: None,
            next_search_request_id: 0,
            search_in_flight: false,
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
        }
    }

    fn initialize_live_memories(&mut self) {
        if !self.is_live() {
            return;
        }

        let _ = self.start_live_memories_load(None, false);
    }

    fn start_live_memories_load(
        &mut self,
        notify_message: Option<&str>,
        preserve_query: bool,
    ) -> Option<CoreEffect> {
        if !self.is_live() {
            return Some(CoreEffect::Notify(
                "Live memories unavailable in mock mode.".to_string(),
            ));
        }

        if self.initial_memories_in_flight {
            return Some(CoreEffect::Notify(
                "Memories are already loading.".to_string(),
            ));
        }

        self.memories_mode = MemoriesMode::Browser;
        if !preserve_query {
            self.query.clear();
        }
        self.result_records.clear();
        self.invalidate_pending_search();
        self.pending_search = None;
        self.search_in_flight = false;

        self.all = vec![loading_memories_record()];
        self.memory_records.clear();
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
            KINIC_INSERT_TAB_ID => vec![CoreEffect::Notify(
                "Insert tab is ready. Submit to write into a memory.".to_string(),
            )],
            KINIC_MEMORIES_TAB_ID => self
                .start_live_memories_load(Some("Refreshing memories..."), true)
                .into_iter()
                .collect(),
            _ => Vec::new(),
        }
    }

    fn is_live(&self) -> bool {
        self.config.auth.is_live()
    }

    fn current_records(&self) -> Vec<&KinicRecord> {
        if self.is_live()
            && self.memories_mode == MemoriesMode::Browser
            && self.memory_records.is_empty()
        {
            return self.all.iter().collect();
        }
        let base = if self.is_live() {
            match self.memories_mode {
                MemoriesMode::Browser => &self.memory_records,
                MemoriesMode::Results => &self.result_records,
            }
        } else {
            &self.all
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
        if !self.is_live() || self.memories_mode != MemoriesMode::Browser {
            return Vec::new();
        }
        if self.memory_records.is_empty() {
            return Vec::new();
        }
        self.current_records()
    }

    fn sync_active_memory_to_visible_records(&mut self) {
        if !self.is_live() || self.memories_mode != MemoriesMode::Browser {
            return;
        }

        let previous_active_memory_id = self.active_memory_id.clone();
        if self.query.is_empty() {
            if self.active_memory_id.is_none() {
                self.active_memory_id = self.memory_records.first().map(|record| record.id.clone());
            }
            if self.active_memory_id != previous_active_memory_id {
                self.invalidate_pending_search();
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
    }

    fn live_search_target_id(&self) -> Option<String> {
        if !self.is_live() || self.memories_mode != MemoriesMode::Browser {
            return self.active_memory_id.clone();
        }

        if self.query.is_empty() {
            return self.active_memory_id.clone();
        }

        let visible_records = self.visible_memory_records();
        if visible_records.is_empty() {
            return None;
        }

        if let Some(active_id) = self.active_memory_id.as_ref()
            && visible_records.iter().any(|record| &record.id == active_id)
        {
            return Some(active_id.clone());
        }

        visible_records.first().map(|record| record.id.clone())
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
        if !self.is_live()
            || self.memories_mode != MemoriesMode::Browser
            || self.visible_memory_count() == 0
        {
            return;
        }

        let visible_records = self.visible_memory_records();
        let current = self.active_visible_memory_index().unwrap_or(0) as isize;
        let last = visible_records.len().saturating_sub(1) as isize;
        let next = (current + delta).clamp(0, last) as usize;
        self.active_memory_id = Some(visible_records[next].id.clone());
        self.invalidate_pending_search();
    }

    fn set_active_memory(&mut self, index: usize) {
        if !self.is_live() || self.memories_mode != MemoriesMode::Browser {
            return;
        }
        let visible_records = self.visible_memory_records();
        let Some(record) = visible_records.get(index) else {
            return;
        };
        self.active_memory_id = Some(record.id.clone());
        self.invalidate_pending_search();
    }

    fn should_handle_memory_navigation(&self, state: &CoreState) -> bool {
        state.current_tab_id == KINIC_MEMORIES_TAB_ID
            && self.tab_id == KINIC_MEMORIES_TAB_ID
            && self.memories_mode == MemoriesMode::Browser
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.current_records();
        let default_memory = self.default_memory_selection();
        let default_memory_selector_items = default_memory.available_memory_ids();
        let default_memory_selector_context = state.default_memory_selector_context;
        let default_memory_selector_selected_id =
            self.selector_selected_memory_id(state, default_memory_selector_context);
        let insert_memory_placeholder = self.insert_memory_placeholder_label(state);
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
        } else if self.is_live() && self.memories_mode == MemoriesMode::Browser {
            if self.memory_records.is_empty() {
                filtered.first().copied().map(adapter::to_content)
            } else {
                self.active_visible_memory_record().map(adapter::to_content)
            }
        } else {
            let sel = state.selected_index.unwrap_or(0);
            filtered.get(sel).map(|r| adapter::to_content(r))
        };

        ProviderSnapshot {
            items,
            selected_content,
            selected_context: None,
            total_count: filtered.len(),
            status_message: Some(self.status_message(filtered.len())),
            create_cost_state: self.create_cost_state.clone(),
            create_submit_state: state.create_submit_state.clone(),
            settings: settings::build_settings_snapshot(
                &self.session_overview,
                &self.user_preferences,
                &default_memory_selector_items,
                &self.preferences_health,
            ),
            default_memory_selector_items,
            default_memory_selector_selected_id,
            default_memory_selector_context,
            insert_memory_placeholder,
        }
    }

    fn selector_selected_memory_id(
        &self,
        state: &CoreState,
        context: MemorySelectorContext,
    ) -> Option<String> {
        match context {
            MemorySelectorContext::DefaultPreference => {
                self.default_memory_selection().selected_default_memory_id()
            }
            MemorySelectorContext::InsertTarget => {
                let insert_memory_id = state.insert_memory_id.trim();
                if !insert_memory_id.is_empty() {
                    return Some(insert_memory_id.to_string());
                }
                self.user_preferences.default_memory_id.clone()
            }
        }
    }

    fn insert_memory_placeholder_label(&self, state: &CoreState) -> Option<String> {
        if !state.insert_memory_id.trim().is_empty() {
            return None;
        }
        let default_memory_id = self.user_preferences.default_memory_id.as_deref()?;
        Some(
            self.default_memory_selection()
                .label_for_id(default_memory_id)
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
            is_live: self.is_live(),
            user_preferences: &mut self.user_preferences,
            preferences_health: &mut self.preferences_health,
        }
    }

    fn start_session_settings_refresh(&mut self) -> Option<CoreEffect> {
        if !self.is_live() {
            return None;
        }
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

        Some(CoreEffect::Notify(
            "Refreshing session settings...".to_string(),
        ))
    }

    fn start_create_cost_refresh(&mut self) -> Option<CoreEffect> {
        if !self.is_live() {
            self.create_cost_state = CreateCostState::Unavailable;
            return Some(CoreEffect::Notify(
                "Live account info unavailable in mock mode.".to_string(),
            ));
        }
        if self.create_cost_in_flight {
            return Some(CoreEffect::Notify(
                "Account info refresh already running.".to_string(),
            ));
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

        Some(CoreEffect::Notify("Refreshing account info...".to_string()))
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

    fn build_insert_request(&self, state: &CoreState) -> InsertRequest {
        let memory_id = state.insert_memory_id.trim().to_string();
        let tag = state.insert_tag.trim().to_string();

        match state.insert_mode {
            InsertMode::Normal => InsertRequest::Normal {
                memory_id,
                tag,
                text: (!state.insert_text.trim().is_empty()).then(|| state.insert_text.clone()),
                file_path: (!state.insert_file_path.trim().is_empty())
                    .then(|| std::path::PathBuf::from(state.insert_file_path.trim())),
            },
            InsertMode::Raw => InsertRequest::Raw {
                memory_id,
                tag,
                text: state.insert_text.clone(),
                embedding_json: state.insert_embedding.clone(),
            },
            InsertMode::Pdf => InsertRequest::Pdf {
                memory_id,
                tag,
                file_path: std::path::PathBuf::from(state.insert_file_path.trim()),
            },
        }
    }

    fn status_message(&self, visible_count: usize) -> String {
        if self.tab_id == KINIC_INSERT_TAB_ID {
            return "kinic(insert): choose mode, target memory, and payload, then press Enter on submit.".to_string();
        }
        let base = if !self.is_live() {
            format!(
                "kinic(mock): {visible_count} filtered / {} total",
                self.all.len()
            )
        } else {
            match self.memories_mode {
                MemoriesMode::Browser => match self.active_memory_id.as_deref() {
                    Some(memory_id) => format!(
                        "kinic(live): target {memory_id} | Enter in search runs remote search | Shift+D saves default"
                    ),
                    None => "kinic(live): no memory selected".to_string(),
                },
                MemoriesMode::Results => match self.active_memory_id.as_deref() {
                    Some(memory_id) => format!(
                        "kinic(live): {visible_count} search results in {memory_id} | Esc clears search and returns | Shift+D saves default"
                    ),
                    None => format!("kinic(live): {visible_count} search results"),
                },
            }
        };

        if !self.is_live() {
            return base;
        }

        if self.initial_memories_in_flight {
            return "kinic(live): loading memories...".to_string();
        }
        if self.is_memories_load_error_visible() {
            return "kinic(live): memories unavailable | F5 retries loading".to_string();
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
        self.pending_search_context = None;
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

    fn search_context(request_id: u64, memory_id: String, query: String) -> SearchRequestContext {
        SearchRequestContext {
            request_id,
            memory_id,
            query,
        }
    }

    fn matches_pending_search(&self, output: &SearchTaskOutput) -> bool {
        self.pending_search_context.as_ref().is_some_and(|context| {
            context.request_id == output.request_id
                && context.memory_id == output.memory_id
                && context.query == output.query
        })
    }

    fn run_live_search(&mut self) -> Option<CoreEffect> {
        let auth = self.config.auth.clone();
        if !auth.is_live() {
            return None;
        }
        if self.search_in_flight {
            return Some(CoreEffect::Notify(
                "Search already running. Wait for the current request to finish.".to_string(),
            ));
        }
        let Some(memory_id) = self.live_search_target_id() else {
            return Some(CoreEffect::Notify(
                "Select a memory in the list before running search.".to_string(),
            ));
        };
        let query = self.query.trim().to_string();
        if query.is_empty() {
            self.memories_mode = MemoriesMode::Browser;
            self.result_records.clear();
            self.invalidate_pending_search();
            return Some(CoreEffect::Notify(
                "Cleared search query and returned to memories.".to_string(),
            ));
        }

        let use_mainnet = self.config.use_mainnet;
        let request_id = self.next_search_request_id;
        self.next_search_request_id += 1;
        self.pending_search_context = Some(Self::search_context(
            request_id,
            memory_id.clone(),
            query.clone(),
        ));
        let (tx, rx) = mpsc::channel();
        self.pending_search = Some(rx);
        self.search_in_flight = true;

        thread::spawn(move || {
            let runtime = Runtime::new().expect("failed to create tokio runtime for search");
            let result = runtime
                .block_on(bridge::search_memory(
                    use_mainnet,
                    auth,
                    memory_id.clone(),
                    query.clone(),
                ))
                .map_err(|error| error.to_string());
            let _ = tx.send(SearchTaskOutput {
                request_id,
                memory_id,
                query,
                result,
            });
        });

        Some(CoreEffect::Notify("Searching...".to_string()))
    }

    fn poll_search_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_search.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_search = None;
                self.invalidate_pending_search();
                self.search_in_flight = false;
                return Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::Notify(
                        "Search worker disconnected unexpectedly.".to_string(),
                    )],
                });
            }
        };

        self.pending_search = None;
        self.search_in_flight = false;
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
                self.result_records = results
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| record_from_search_result(index, &output.memory_id, item))
                    .collect();
                self.memories_mode = MemoriesMode::Results;
                let mut effects = vec![CoreEffect::SelectFirstListItem];
                if state.current_tab_id == KINIC_MEMORIES_TAB_ID {
                    effects.push(CoreEffect::FocusPane(PaneFocus::Items));
                }
                effects.push(CoreEffect::Notify(format!(
                    "Loaded {} search results for {}",
                    self.result_records.len(),
                    output.memory_id
                )));
                effects
            }
            Err(error) => {
                self.result_records.clear();
                self.memories_mode = MemoriesMode::Browser;
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
                self.memory_records = memories
                    .into_iter()
                    .map(record_from_memory_summary)
                    .collect();
                self.all = self.memory_records.clone();
                self.active_memory_id = self
                    .default_memory_selection()
                    .preferred_initial_memory_id()
                    .or_else(|| self.memory_records.first().map(|record| record.id.clone()));
                Some(ProviderOutput {
                    snapshot: Some(self.build_snapshot(state)),
                    effects: vec![CoreEffect::Notify("Loaded memories.".to_string())],
                })
            }
            Err(error) => {
                self.memory_records.clear();
                self.result_records.clear();
                self.memories_mode = MemoriesMode::Browser;
                self.all = vec![load_error_record(error)];
                self.active_memory_id = None;
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
                    self.memory_records = memories
                        .into_iter()
                        .map(record_from_memory_summary)
                        .collect();
                    self.all = self.memory_records.clone();
                    if let Some(index) = self.memory_records.iter().position(|r| r.id == success.id)
                    {
                        let record = self.memory_records.remove(index);
                        self.memory_records.insert(0, record.clone());
                        self.all = self.memory_records.clone();
                    }
                }
                self.memories_mode = MemoriesMode::Browser;
                self.result_records.clear();
                self.invalidate_pending_search();
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

        let notify_message = output
            .overview
            .session_settings_refresh_failure_message()
            .unwrap_or_else(|| output.overview.session_settings_refresh_notify_message());
        self.apply_session_overview(output.overview);
        let effects = vec![CoreEffect::Notify(notify_message)];

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
                CoreEffect::Notify(format!(
                    "Inserted {} item(s) via {} into {} [{}]",
                    success.inserted_count, success.mode, success.memory_id, success.tag
                )),
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
        if self.is_live() {
            self.memories_mode = MemoriesMode::Browser;
            self.result_records.clear();
            self.invalidate_pending_search();
        }
    }

    fn set_tab(&mut self, tab_id: &str) -> Vec<CoreEffect> {
        self.tab_id = tab_id.to_string();

        match tab_id {
            KINIC_MEMORIES_TAB_ID => {
                self.reset_memories_browser();
                vec![CoreEffect::Notify("Switched to memories.".to_string())]
            }
            KINIC_INSERT_TAB_ID => {
                vec![CoreEffect::Notify(
                    "Insert text, embeddings, or PDFs into an existing memory.".to_string(),
                )]
            }
            KINIC_CREATE_TAB_ID => {
                let mut effects = vec![CoreEffect::Notify("Create a new memory.".to_string())];
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

impl DataProvider for KinicProvider {
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
        self.initialize_live_memories();
        Ok(self.build_snapshot(&CoreState::default()))
    }

    fn handle_action(
        &mut self,
        action: &CoreAction,
        state: &CoreState,
    ) -> CoreResult<ProviderOutput> {
        let mut effects = Vec::new();
        match action {
            CoreAction::SetQuery(q) => {
                self.query = q.clone();
                self.invalidate_pending_search();
                if self.tab_id == KINIC_MEMORIES_TAB_ID && q.is_empty() {
                    self.reset_memories_browser();
                }
                self.sync_active_memory_to_visible_records();
            }
            CoreAction::SearchInput(c) => {
                self.query.push(*c);
                self.invalidate_pending_search();
                self.sync_active_memory_to_visible_records();
            }
            CoreAction::SearchBackspace => {
                self.query.pop();
                self.invalidate_pending_search();
                if self.query.is_empty() {
                    self.reset_memories_browser();
                }
                self.sync_active_memory_to_visible_records();
            }
            CoreAction::SearchSubmit => {
                if self.is_live()
                    && let Some(effect) = self.run_live_search()
                {
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
            }
            CoreAction::ChatSubmit => {
                effects.push(CoreEffect::Notify(
                    "Chat is still mock-only; search is live first.".to_string(),
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
                } else if self.is_live() {
                    effects.push(self.start_create_submit(name, description));
                } else {
                    let new_id = format!("mock-memory-{}", self.all.len() + 1);
                    let record = KinicRecord::new(
                        new_id.clone(),
                        name.clone(),
                        "memories",
                        "Status: mock".to_string(),
                        format!(
                            "## Memory\n\n- Id: `{new_id}`\n- Status: `mock`\n\n### Content\n{}\n",
                            state.create_description.trim()
                        ),
                    );
                    self.all.insert(0, record);
                    self.active_memory_id = Some(new_id.clone());
                    effects.extend(self.set_tab(KINIC_MEMORIES_TAB_ID));
                    effects.push(CoreEffect::SelectFirstListItem);
                    effects.push(CoreEffect::ResetCreateFormAndSetTab {
                        tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                    });
                    effects.push(CoreEffect::FocusPane(PaneFocus::Items));
                    effects.push(CoreEffect::Notify(format!("Created mock memory {name}")));
                }
            }
            CoreAction::InsertSubmit => {
                let request = self.build_insert_request(state);
                if let Err(error) = validate_insert_request(&request) {
                    effects.push(CoreEffect::InsertFormError(Some(error.to_string())));
                } else if self.insert_submit_in_flight {
                    effects.push(CoreEffect::Notify(
                        "Insert request already running.".to_string(),
                    ));
                } else if self.is_live() {
                    effects.push(self.start_insert_submit(request));
                } else {
                    effects.push(CoreEffect::InsertFormError(None));
                    effects.push(CoreEffect::ResetInsertFormForRepeat);
                    effects.push(CoreEffect::Notify(format!(
                        "Mock insert accepted for {} [{}]",
                        state.insert_memory_id.trim(),
                        state.insert_tag.trim()
                    )));
                }
            }
            CoreAction::CreateRefresh => {
                if let Some(effect) = self.start_create_cost_refresh() {
                    effects.push(effect);
                }
            }
            CoreAction::RefreshCurrentView => {
                effects.extend(self.refresh_current_view());
            }
            CoreAction::ToggleSettings => {
                if let Some(effect) = self.start_session_settings_refresh() {
                    effects.push(effect);
                }
            }
            CoreAction::OpenDefaultMemoryPicker => {}
            CoreAction::CloseDefaultMemoryPicker => {}
            CoreAction::MoveDefaultMemoryPickerNext => {}
            CoreAction::MoveDefaultMemoryPickerPrev => {}
            CoreAction::ScrollContentPageDown => {}
            CoreAction::ScrollContentPageUp => {}
            CoreAction::ScrollContentHome => {}
            CoreAction::ScrollContentEnd => {}
            CoreAction::SubmitDefaultMemoryPicker => {
                let Some(memory_id) = state
                    .default_memory_selector_items
                    .get(state.default_memory_selector_index)
                    .cloned()
                else {
                    effects.push(CoreEffect::Notify("No memories available yet.".to_string()));
                    return Ok(ProviderOutput {
                        snapshot: Some(self.build_snapshot(state)),
                        effects,
                    });
                };
                match state.default_memory_selector_context {
                    MemorySelectorContext::DefaultPreference => effects.push(
                        self.default_memory_controller()
                            .set_default_memory_preference(memory_id),
                    ),
                    MemorySelectorContext::InsertTarget => {
                        effects.push(CoreEffect::SetInsertMemoryId(memory_id.clone()));
                        effects.push(CoreEffect::Notify(format!(
                            "Selected target memory {memory_id}"
                        )));
                    }
                }
            }
            CoreAction::SetDefaultMemoryFromSelection => {
                let Some(memory_id) = self.active_memory_id.clone() else {
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

        Ok(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }

    fn poll_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        self.poll_initial_memories_background(state)
            .or_else(|| self.poll_create_submit_background(state))
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
    KinicRecord::new(
        memory.id.clone(),
        memory.id.clone(),
        "memories",
        format!("Status: {}", memory.status),
        format!(
            "## Memory\n\n- Id: `{}`\n- Status: `{}`\n\n### Content\n{}\n\n### Search\nSelect this item, then type a query and press Enter in the search box.",
            memory.id, memory.status, memory.detail
        ),
    )
}

fn record_from_search_result(index: usize, memory_id: &str, item: SearchResultItem) -> KinicRecord {
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
        format!("{memory_id}-result-{}", index + 1),
        title,
        "search-result",
        format!("Score: {score} | Tag: {tag}"),
        format!(
            "## Search Hit\n\n- Memory: `{memory_id}`\n- Score: `{score}`\n- Tag: `{tag}`\n\n### Sentence\n{}\n\n### Raw Payload\n{}\n",
            detail_body, item.payload
        ),
    )
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
        bridge::InsertMemoryError::Principal(reason) => {
            format!("Could not resolve memory canister. Cause: {reason}")
        }
        bridge::InsertMemoryError::Execute(reason) => {
            format!("Insert failed. Cause: {reason}")
        }
    }
}

fn sample_records() -> Vec<KinicRecord> {
    vec![
        KinicRecord::new(
            "kinic-1",
            "Unified TUI Kit",
            "memories",
            "Single facade crate with modular host/runtime/render layers.",
            r#"## Daily Note
- Split crate structure into `host/runtime/render/model`
- Added unified facade crate: `tui-kit`

### Why it mattered
Keeping runtime domain-agnostic made every demo easier to compose.

```rust
let ui = TuiKitUi::new(&theme);
```
"#,
        ),
        KinicRecord::new(
            "kinic-5",
            "Keyboard Workflow Snapshot",
            "memories",
            "Focused on tab-first navigation and predictable pane order.",
            r#"## Navigation Log
1. Search for an entry
2. Move to list with `Tab`
3. Open content with `Enter`

### Notes
- Keep tabs at the end of the focus cycle
- Prioritize consistency over shortcuts
"#,
        ),
        KinicRecord::new(
            "kinic-6",
            "UI Polish Memo",
            "memories",
            "Captured tweaks around scrollbars, placeholders, and cursor behavior.",
            r#"## UI Polish
- Placeholder uses muted/italic style
- Cursor only appears in active input fields
- Scrollbar uses a custom track + thumb

### TODO
- [ ] Mouse wheel support per pane
- [ ] Unified toast notifications
"#,
        ),
        KinicRecord::new(
            "kinic-7",
            "Release Draft 0.1",
            "memories",
            "First release draft notes for the reusable tui-kit package.",
            r#"## Release Draft 0.1
- Stabilize facade crate exports
- Freeze keyboard navigation defaults
- Add one polished domain sample

### Changelog Snippet
- `feat`: tabs focus cycle
- `fix`: list scrollbar behavior
"#,
        ),
        KinicRecord::new(
            "kinic-8",
            "Design Review: Left Pane",
            "memories",
            "Discussed list density and readability under compact terminals.",
            r#"## Left Pane Review
- Keep icon + kind prefix
- Avoid truncating item title too early
- Prefer subtle divider over heavy borders

```text
Goal: scanability first, decoration second.
```
"#,
        ),
        KinicRecord::new(
            "kinic-9",
            "Design Review: Right Pane",
            "memories",
            "Evaluated section hierarchy and markdown-ish readability.",
            r#"## Right Pane Review
1. Strong title
2. Definition block
3. Sections with clear heading

### Decision
Use `◇ Content` naming consistently.
"#,
        ),
        KinicRecord::new(
            "kinic-10",
            "Keyboard Mapping Matrix",
            "memories",
            "Captured focus navigation matrix for Search/List/Content/Tabs/Chat.",
            r#"## Keyboard Matrix
- `Tab`: next focus
- `Shift+Tab`: previous focus
- Tabs focus: `↑/↓` to switch tab

### Regression Check
- Ensure `Enter` from Tabs reaches Content.
"#,
        ),
        KinicRecord::new(
            "kinic-11",
            "Interaction Bug Notes",
            "memories",
            "Log of edge cases found during runtime-first migration.",
            r#"## Bug Notes
- Chat focus consumed key without sync
- List scroll anchor drifted on reverse motion
- Status row index mismatch after layout update

### Action
Patch quickly, then add snapshot tests.
"#,
        ),
        KinicRecord::new(
            "kinic-12",
            "Theme Study",
            "memories",
            "Compared contrast ratios across dark presets and pink variant.",
            r#"## Theme Study
- Nord: calm, high legibility
- Dracula: vivid syntax emphasis
- Pink: branded accent direction

### Follow-up
Add high-contrast accessibility preset.
"#,
        ),
        KinicRecord::new(
            "kinic-13",
            "Provider Contract Memo",
            "memories",
            "Summarized DataProvider boundaries for domain teams.",
            r#"## Provider Contract
- Provider owns data shaping
- Runtime owns local interaction state
- Render stays domain-agnostic

```rust
fn handle_action(&mut self, action: &CoreAction, state: &CoreState)
```
"#,
        ),
        KinicRecord::new(
            "kinic-14",
            "Host Boundary Memo",
            "memories",
            "Clarified responsibilities of host loop and side-effect execution.",
            r#"## Host Boundary
- Poll input
- Resolve global commands
- Execute effects (`OpenExternal`, notifications)

### Keep out of runtime
Anything terminal/platform-specific.
"#,
        ),
        KinicRecord::new(
            "kinic-15",
            "Render Boundary Memo",
            "memories",
            "Defined what belongs in render and what does not.",
            r#"## Render Boundary
- Layout and visuals
- Cursor coordinates
- No business/domain side effects

### Practical Rule
If it needs OS I/O, it is not render.
"#,
        ),
        KinicRecord::new(
            "kinic-16",
            "Migration Checklist",
            "memories",
            "Checklist for moving legacy app flows into shared contracts.",
            r#"## Migration Checklist
- [x] Split model/runtime/host/render
- [x] Add facade crate
- [x] Move common runtime loop
- [ ] Replace remaining domain loops with hooks
"#,
        ),
        KinicRecord::new(
            "kinic-17",
            "UX Copy Candidates",
            "memories",
            "Alternatives for generic labels in reusable UI kit.",
            r#"## Copy Candidates
- Tabs -> Views / Sections
- Inspector -> Content
- Context -> Group

### Principle
Prefer domain-neutral defaults with app-level overrides.
"#,
        ),
        KinicRecord::new(
            "kinic-18",
            "Sample Data Expansion Plan",
            "memories",
            "Prepared larger datasets for manual scrolling and search QA.",
            r#"## Expansion Plan
1. Add 20+ memory records
2. Ensure varied title lengths
3. Include markdown-like body text

### Why
Better confidence in viewport + scrollbar behavior.
"#,
        ),
        KinicRecord::new(
            "kinic-19",
            "Command Palette Idea",
            "memories",
            "Rough concept for command palette integration.",
            r#"## Command Palette Idea
- Trigger with `:`
- Fuzzy command search
- Action dispatch into runtime

### Future
Could become a separate optional module.
"#,
        ),
        KinicRecord::new(
            "kinic-20",
            "Copilot-to-Chat Rename",
            "memories",
            "Documented terminology cleanup for domain neutrality.",
            r#"## Terminology Cleanup
- Remove product-specific terms from shared crates
- Keep neutral naming in runtime/render/host

### Result
UI kit can be reused across mail/task/other domains.
"#,
        ),
        KinicRecord::new(
            "kinic-21",
            "Mouse Support TODO",
            "memories",
            "Pending mouse wheel and click interactions for each pane.",
            r#"## Mouse Support TODO
- Wheel scroll in List/Content/Chat
- Click to focus pane
- Click tabs to switch

### Constraint
Maintain keyboard-first behavior as baseline.
"#,
        ),
        KinicRecord::new(
            "kinic-22",
            "Content Mock Library",
            "memories",
            "Centralized mock snippets for demos and screenshots.",
            r#"## Content Mock Library
- Keep short and long variants
- Include bullets, headings, and pseudo code
- Avoid domain-sensitive terms by default

```md
## Heading
- item
```
"#,
        ),
    ]
}

#[cfg(test)]
#[path = "provider/tests.rs"]
mod tests;
