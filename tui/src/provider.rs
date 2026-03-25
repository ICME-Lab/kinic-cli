use std::{future::Future, sync::mpsc, thread};

use super::adapter;
use super::bridge::{self, MemorySummary, SearchResultItem};
use super::settings::{self, PreferencesHealth, SessionSettingsSnapshot, UserPreferences};
use crate::tui::TuiAuth;
use serde::Deserialize;
use tokio::runtime::{Handle, Runtime};
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreResult, CoreState, DataProvider, PaneFocus, ProviderOutput,
    ProviderSnapshot,
    kinic_tabs::{
        KINIC_CREATE_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID, KINIC_SETTINGS_TAB_ID,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
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

enum BlockingRuntime {
    Owned(Runtime),
    Handle(Handle),
}

impl BlockingRuntime {
    fn new() -> Self {
        if let Ok(handle) = Handle::try_current() {
            Self::Handle(handle)
        } else {
            Self::Owned(Runtime::new().expect("failed to create tokio runtime for tui"))
        }
    }

    fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        match self {
            Self::Owned(runtime) => runtime.block_on(future),
            Self::Handle(handle) => tokio::task::block_in_place(|| handle.block_on(future)),
        }
    }
}

pub struct KinicProvider {
    all: Vec<KinicRecord>,
    query: String,
    tab_id: String,
    runtime: BlockingRuntime,
    config: TuiConfig,
    session_settings: SessionSettingsSnapshot,
    user_preferences: UserPreferences,
    preferences_health: PreferencesHealth,
    active_memory_id: Option<String>,
    memory_records: Vec<KinicRecord>,
    result_records: Vec<KinicRecord>,
    memories_mode: MemoriesMode,
    pending_search: Option<mpsc::Receiver<SearchTaskOutput>>,
    search_in_flight: bool,
}

struct SearchTaskOutput {
    memory_id: String,
    result: Result<Vec<SearchResultItem>, String>,
}

impl KinicProvider {
    pub fn new(config: TuiConfig) -> Self {
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
        let session_settings = SessionSettingsSnapshot::new(
            &config.auth,
            config.use_mainnet,
            None,
            crate::embedding::embedding_base_url(),
            user_preferences.default_memory_id.clone(),
        );

        Self {
            all: sample_records(),
            query: String::new(),
            tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            runtime: BlockingRuntime::new(),
            config,
            session_settings,
            user_preferences,
            preferences_health,
            active_memory_id: None,
            memory_records: Vec::new(),
            result_records: Vec::new(),
            memories_mode: MemoriesMode::Browser,
            pending_search: None,
            search_in_flight: false,
        }
    }

    fn initialize_live_memories(&mut self) {
        if !self.is_live() {
            return;
        }

        match self.runtime.block_on(bridge::list_memories(
            self.config.use_mainnet,
            self.config.auth.clone(),
        )) {
            Ok(memories) => {
                self.memory_records = memories
                    .into_iter()
                    .map(record_from_memory_summary)
                    .collect();
                self.prioritize_default_memory();
                self.all = self.memory_records.clone();
                self.active_memory_id = self.memory_records.first().map(|record| record.id.clone());
            }
            Err(error) => {
                self.all = vec![KinicRecord::new(
                    "kinic-live-error",
                    "Unable to load memories",
                    "memories",
                    "Check your identity or network configuration.",
                    format!("## Live Load Error\n\n{error}"),
                )];
            }
        }
    }

    fn reload_live_memories(&mut self, prioritize_id: Option<&str>) -> Result<(), String> {
        if !self.is_live() {
            return Ok(());
        }
        let memories = self
            .runtime
            .block_on(bridge::list_memories(
                self.config.use_mainnet,
                self.config.auth.clone(),
            ))
            .map_err(|error| error.to_string())?;
        self.memory_records = memories
            .into_iter()
            .map(record_from_memory_summary)
            .collect();
        if let Some(id) = prioritize_id
            && let Some(index) = self.memory_records.iter().position(|r| r.id == id)
        {
            let record = self.memory_records.remove(index);
            self.memory_records.insert(0, record);
        } else {
            self.prioritize_default_memory();
        }
        self.all = self.memory_records.clone();
        self.active_memory_id = self.memory_records.first().map(|record| record.id.clone());
        self.memories_mode = MemoriesMode::Browser;
        self.result_records.clear();
        Ok(())
    }

    fn is_live(&self) -> bool {
        matches!(self.config.auth, TuiAuth::KeyringIdentity(_))
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

        if self.query.is_empty() {
            if self.active_memory_id.is_none() {
                self.active_memory_id = self.memory_records.first().map(|record| record.id.clone());
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
        self.persist_default_memory_preference();
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
        self.persist_default_memory_preference();
    }

    fn build_snapshot(&self, state: &CoreState) -> ProviderSnapshot {
        let filtered = self.current_records();
        let items = filtered
            .iter()
            .map(|r| adapter::to_summary(r))
            .collect::<Vec<_>>();
        let selected_detail = if self.is_live() && self.memories_mode == MemoriesMode::Browser {
            if self.memory_records.is_empty() {
                filtered.first().copied().map(adapter::to_detail)
            } else {
                self.active_visible_memory_record().map(adapter::to_detail)
            }
        } else {
            let sel = state.selected_index.unwrap_or(0);
            filtered.get(sel).map(|r| adapter::to_detail(r))
        };

        ProviderSnapshot {
            items,
            selected_detail,
            selected_context: None,
            total_count: filtered.len(),
            status_message: Some(self.status_message(filtered.len())),
            settings: settings::build_settings_snapshot(
                &self.session_settings,
                &self.user_preferences,
                &self.available_memory_ids(),
                &self.preferences_health,
            ),
        }
    }

    fn available_memory_ids(&self) -> Vec<String> {
        self.memory_records
            .iter()
            .map(|record| record.id.clone())
            .collect()
    }

    fn refresh_session_settings(&mut self) {
        self.session_settings = self.runtime.block_on(bridge::load_session_settings(
            self.config.use_mainnet,
            self.config.auth.clone(),
            self.user_preferences.default_memory_id.clone(),
        ));
    }

    fn prioritize_default_memory(&mut self) {
        let Some(default_memory_id) = self.user_preferences.default_memory_id.as_deref() else {
            return;
        };
        let Some(index) = self
            .memory_records
            .iter()
            .position(|record| record.id == default_memory_id)
        else {
            return;
        };
        if index == 0 {
            return;
        }

        let record = self.memory_records.remove(index);
        self.memory_records.insert(0, record);
    }

    fn persist_default_memory_preference(&mut self) {
        if !self.is_live()
            || self.memories_mode != MemoriesMode::Browser
            || self.tab_id != KINIC_MEMORIES_TAB_ID
        {
            return;
        }
        let Some(active_memory_id) = self.active_memory_id.clone() else {
            return;
        };
        if self.user_preferences.default_memory_id.as_deref() == Some(active_memory_id.as_str()) {
            return;
        }

        let updated_preferences = UserPreferences {
            default_memory_id: Some(active_memory_id),
        };
        match settings::save_user_preferences(&updated_preferences) {
            Ok(()) => {
                self.user_preferences = updated_preferences;
                self.session_settings.default_memory_id =
                    self.user_preferences.default_memory_id.clone();
                self.preferences_health.save_error = None;
            }
            Err(error) => {
                self.preferences_health.save_error = Some(error.to_string());
            }
        }
    }

    fn status_message(&self, visible_count: usize) -> String {
        if self.is_live()
            && self.memory_records.is_empty()
            && self
                .all
                .first()
                .is_some_and(|record| record.id == "kinic-live-error")
        {
            return "kinic(live): unable to load memories | check identity or network configuration"
                .to_string();
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
                    "kinic(live): target {memory_id} | j/k selects memory canister | Enter in search runs remote search"
                ),
                None => "kinic(live): no memory selected | j/k selects memory canister".to_string(),
            },
            MemoriesMode::Results => match self.active_memory_id.as_deref() {
                Some(memory_id) => format!(
                    "kinic(live): {visible_count} search results in {memory_id} | Esc clears search and returns"
                ),
                None => format!("kinic(live): {visible_count} search results"),
            },
            }
        };
        if let Some(error) = &self.preferences_health.save_error {
            return format!("{base} | preferences save failed: {error}");
        }
        if let Some(error) = &self.preferences_health.load_error {
            return format!("{base} | preferences load failed: {error}");
        }
        base
    }

    fn run_live_search(&mut self) -> Option<CoreEffect> {
        let auth = self.config.auth.clone();
        if !matches!(auth, TuiAuth::KeyringIdentity(_)) {
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
            return Some(CoreEffect::Notify(
                "Cleared search query and returned to memories.".to_string(),
            ));
        }

        let use_mainnet = self.config.use_mainnet;
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
                    query,
                ))
                .map_err(|error| error.to_string());
            let _ = tx.send(SearchTaskOutput { memory_id, result });
        });

        Some(CoreEffect::Notify("Searching...".to_string()))
    }

    pub fn poll_background(&mut self, state: &CoreState) -> Option<ProviderOutput> {
        let receiver = self.pending_search.as_ref()?;
        let output = match receiver.try_recv() {
            Ok(output) => output,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending_search = None;
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

        let effects = match output.result {
            Ok(results) => {
                self.result_records = results
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| record_from_search_result(index, &output.memory_id, item))
                    .collect();
                self.memories_mode = MemoriesMode::Results;
                vec![CoreEffect::SearchCompleted {
                    message: format!(
                        "Loaded {} search results for {}",
                        self.result_records.len(),
                        output.memory_id
                    ),
                }]
            }
            Err(error) => vec![CoreEffect::Notify(format!("Search failed: {error}"))],
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
        }
    }

    fn set_tab(&mut self, tab_id: &str) -> Vec<CoreEffect> {
        self.tab_id = tab_id.to_string();

        match tab_id {
            KINIC_MEMORIES_TAB_ID => {
                self.reset_memories_browser();
                vec![CoreEffect::Notify("Switched to memories.".to_string())]
            }
            KINIC_CREATE_TAB_ID => vec![CoreEffect::Notify("Create a new memory.".to_string())],
            KINIC_MARKET_TAB_ID => {
                vec![CoreEffect::Notify(
                    "Market is not implemented yet.".to_string(),
                )]
            }
            KINIC_SETTINGS_TAB_ID => vec![CoreEffect::Notify(
                "Settings tab shows detailed status. Shift+S opens quick status.".to_string(),
            )],
            _ => vec![CoreEffect::Notify(format!("Switched kinic tab: {tab_id}"))],
        }
    }
}

impl DataProvider for KinicProvider {
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
        self.refresh_session_settings();
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
                if self.tab_id == KINIC_MEMORIES_TAB_ID && q.is_empty() {
                    self.reset_memories_browser();
                }
                self.sync_active_memory_to_visible_records();
            }
            CoreAction::SearchInput(c) => {
                self.query.push(*c);
                self.sync_active_memory_to_visible_records();
            }
            CoreAction::SearchBackspace => {
                self.query.pop();
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
            CoreAction::MoveNext => self.move_active_memory(1),
            CoreAction::MovePrev => self.move_active_memory(-1),
            CoreAction::MoveHome => self.set_active_memory(0),
            CoreAction::MoveEnd => {
                let visible_count = self.visible_memory_count();
                if visible_count != 0 {
                    self.set_active_memory(visible_count - 1);
                }
            }
            CoreAction::MovePageDown => self.move_active_memory(10),
            CoreAction::MovePageUp => self.move_active_memory(-10),
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
                } else if self.is_live() {
                    match self.runtime.block_on(bridge::create_memory(
                        self.config.use_mainnet,
                        self.config.auth.clone(),
                        name.clone(),
                        description,
                    )) {
                        Ok(created_id) => match self.reload_live_memories(Some(&created_id)) {
                            Ok(()) => {
                                self.active_memory_id = Some(created_id.clone());
                                self.persist_default_memory_preference();
                                effects.extend(self.set_tab(KINIC_MEMORIES_TAB_ID));
                                effects.push(CoreEffect::SelectFirstListItem);
                                effects.push(CoreEffect::ResetCreateFormAndSetTab {
                                    tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                                });
                                effects.push(CoreEffect::FocusPane(PaneFocus::List));
                                effects.push(CoreEffect::Notify(format!(
                                    "Created memory {created_id}"
                                )));
                            }
                            Err(error) => {
                                effects.push(CoreEffect::CreateFormError(Some(error)));
                            }
                        },
                        Err(error) => {
                            effects.push(CoreEffect::CreateFormError(Some(error.to_string())));
                        }
                    }
                } else {
                    let new_id = format!("mock-memory-{}", self.all.len() + 1);
                    let record = KinicRecord::new(
                        new_id.clone(),
                        name.clone(),
                        "memories",
                        "Status: mock".to_string(),
                        format!(
                            "## Memory\n\n- Id: `{new_id}`\n- Status: `mock`\n\n### Detail\n{}\n",
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
                    effects.push(CoreEffect::FocusPane(PaneFocus::List));
                    effects.push(CoreEffect::Notify(format!("Created mock memory {name}")));
                }
            }
            _ => {}
        }

        Ok(ProviderOutput {
            snapshot: Some(self.build_snapshot(state)),
            effects,
        })
    }
}

fn record_from_memory_summary(memory: MemorySummary) -> KinicRecord {
    KinicRecord::new(
        memory.id.clone(),
        memory.id.clone(),
        "memories",
        format!("Status: {}", memory.status),
        format!(
            "## Memory\n\n- Id: `{}`\n- Status: `{}`\n\n### Detail\n{}\n\n### Search\nSelect this item, then type a query and press Enter in the search box.",
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
- Tabs focus: `j/k` or `↑/↓` to switch tab

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
mod tests {
    use super::*;

    fn live_config() -> TuiConfig {
        TuiConfig {
            auth: TuiAuth::KeyringIdentity("alice".to_string()),
            use_mainnet: false,
        }
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

    #[test]
    fn current_records_returns_live_browser_memories() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![live_memory("aaaaa-aa", "Memory A")];

        let records = provider.current_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "aaaaa-aa");
    }

    #[test]
    fn current_records_uses_error_row_when_live_load_failed() {
        let mut provider = KinicProvider::new(live_config());
        provider.all = vec![KinicRecord::new(
            "kinic-live-error",
            "Unable to load memories",
            "memories",
            "Check your identity or network configuration.",
            "## Live Load Error",
        )];

        let records = provider.current_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "kinic-live-error");
    }

    #[test]
    fn build_snapshot_uses_error_row_for_detail_when_no_active_memory_exists() {
        let mut provider = KinicProvider::new(live_config());
        provider.all = vec![KinicRecord::new(
            "kinic-live-error",
            "Unable to load memories",
            "memories",
            "Check your identity or network configuration.",
            "## Live Load Error\n\nboom",
        )];

        let snapshot = provider.build_snapshot(&CoreState::default());
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(
            snapshot
                .selected_detail
                .as_ref()
                .map(|detail| detail.id.as_str()),
            Some("kinic-live-error")
        );
    }

    #[test]
    fn live_search_target_is_none_when_live_load_failed() {
        let mut provider = KinicProvider::new(live_config());
        provider.all = vec![KinicRecord::new(
            "kinic-live-error",
            "Unable to load memories",
            "memories",
            "Check your identity or network configuration.",
            "## Live Load Error\n\nboom",
        )];
        provider.query = "alpha".to_string();

        assert_eq!(provider.live_search_target_id(), None);
    }

    #[test]
    fn create_submit_stays_mock_when_auth_is_mock() {
        let provider = KinicProvider::new(TuiConfig {
            auth: TuiAuth::Mock,
            use_mainnet: false,
        });

        assert!(!provider.is_live());
    }

    #[test]
    fn set_tab_create_requests_open_modal() {
        let mut provider = KinicProvider::new(TuiConfig {
            auth: TuiAuth::Mock,
            use_mainnet: false,
        });

        let effects = provider.set_tab(KINIC_CREATE_TAB_ID);

        assert_eq!(provider.tab_id, KINIC_CREATE_TAB_ID);
        assert!(effects.iter().any(|effect| matches!(
            effect,
            CoreEffect::Notify(message) if message == "Create a new memory."
        )));
    }

    #[test]
    fn set_tab_memories_keeps_existing_snapshot_shape() {
        let mut provider = KinicProvider::new(TuiConfig {
            auth: TuiAuth::Mock,
            use_mainnet: false,
        });
        provider.query = "Unified".to_string();

        let effects = provider.set_tab(KINIC_MEMORIES_TAB_ID);
        let snapshot = provider.build_snapshot(&CoreState::default());

        assert_eq!(provider.tab_id, KINIC_MEMORIES_TAB_ID);
        assert!(!snapshot.items.is_empty());
        assert!(effects.iter().any(|effect| matches!(
            effect,
            CoreEffect::Notify(message) if message == "Switched to memories."
        )));
    }

    #[test]
    fn create_submit_returns_to_memories_in_mock_mode() {
        let mut provider = KinicProvider::new(TuiConfig {
            auth: TuiAuth::Mock,
            use_mainnet: false,
        });
        provider.tab_id = KINIC_CREATE_TAB_ID.to_string();

        let state = CoreState {
            create_name: "New Memory".to_string(),
            create_description: "Created from test".to_string(),
            ..CoreState::default()
        };

        let output = provider
            .handle_action(&CoreAction::CreateSubmit, &state)
            .unwrap();

        assert!(output.effects.iter().any(|effect| matches!(
            effect,
            CoreEffect::ResetCreateFormAndSetTab { tab_id }
                if tab_id == KINIC_MEMORIES_TAB_ID
        )));
        assert!(
            output
                .effects
                .iter()
                .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::List)))
        );
        assert_eq!(provider.tab_id, KINIC_MEMORIES_TAB_ID);
    }

    #[test]
    fn clearing_query_after_create_resets_memories_browser() {
        let mut provider = KinicProvider::new(TuiConfig {
            auth: TuiAuth::Mock,
            use_mainnet: false,
        });
        provider.tab_id = KINIC_CREATE_TAB_ID.to_string();
        provider.query = "stale".to_string();
        provider
            .handle_action(
                &CoreAction::CreateSubmit,
                &CoreState {
                    create_name: "New Memory".to_string(),
                    create_description: "Created from test".to_string(),
                    ..CoreState::default()
                },
            )
            .unwrap();

        let output = provider
            .handle_action(&CoreAction::SetQuery(String::new()), &CoreState::default())
            .unwrap();

        assert_eq!(provider.tab_id, KINIC_MEMORIES_TAB_ID);
        assert_eq!(provider.query, "");
        assert_eq!(output.snapshot.unwrap().total_count, provider.all.len());
    }

    #[test]
    fn sync_active_memory_switches_to_first_visible_match() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
        ];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.query = "beta".to_string();

        provider.sync_active_memory_to_visible_records();

        assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
    }

    #[test]
    fn sync_active_memory_keeps_visible_selection() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
        ];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.query = "alpha".to_string();

        provider.sync_active_memory_to_visible_records();

        assert_eq!(provider.active_memory_id.as_deref(), Some("aaaaa-aa"));
    }

    #[test]
    fn sync_active_memory_clears_selection_when_filter_hides_all_memories() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.query = "gamma".to_string();

        provider.sync_active_memory_to_visible_records();

        assert_eq!(provider.active_memory_id, None);
        let snapshot = provider.build_snapshot(&CoreState::default());
        assert!(snapshot.selected_detail.is_none());
        assert!(snapshot.items.is_empty());
    }

    #[test]
    fn sync_active_memory_restores_first_memory_when_query_clears() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
        ];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.query = "gamma".to_string();

        provider.sync_active_memory_to_visible_records();
        assert_eq!(provider.active_memory_id, None);

        provider.query.clear();
        provider.sync_active_memory_to_visible_records();

        assert_eq!(provider.active_memory_id.as_deref(), Some("aaaaa-aa"));
        assert_eq!(
            provider
                .build_snapshot(&CoreState::default())
                .selected_detail
                .as_ref()
                .map(|detail| detail.id.as_str()),
            Some("aaaaa-aa")
        );
        assert_eq!(
            provider.live_search_target_id().as_deref(),
            Some("aaaaa-aa")
        );
    }

    #[test]
    fn live_search_target_matches_visible_memory_after_filtering() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
        ];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.query = "beta".to_string();

        provider.sync_active_memory_to_visible_records();

        assert_eq!(
            provider.live_search_target_id().as_deref(),
            Some("bbbbb-bb")
        );
    }

    #[test]
    fn move_next_stays_within_visible_filtered_memories() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
            live_memory("ccccc-cc", "Bravo Memory"),
        ];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.query = "b".to_string();

        let output = provider
            .handle_action(&CoreAction::MoveNext, &CoreState::default())
            .expect("move next should succeed");

        assert_eq!(provider.active_memory_id.as_deref(), Some("ccccc-cc"));
        assert_eq!(
            output
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.selected_detail.as_ref())
                .map(|detail| detail.id.as_str()),
            Some("ccccc-cc")
        );
        assert_eq!(
            provider.live_search_target_id().as_deref(),
            Some("ccccc-cc")
        );
    }

    #[test]
    fn move_home_and_end_use_visible_filtered_memories() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
            live_memory("ccccc-cc", "Bravo Memory"),
        ];
        provider.all = provider.memory_records.clone();
        provider.query = "b".to_string();
        provider.active_memory_id = Some("ccccc-cc".to_string());

        provider
            .handle_action(&CoreAction::MoveHome, &CoreState::default())
            .expect("move home should succeed");
        assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));

        let output = provider
            .handle_action(&CoreAction::MoveEnd, &CoreState::default())
            .expect("move end should succeed");
        assert_eq!(provider.active_memory_id.as_deref(), Some("ccccc-cc"));
        assert_eq!(
            output
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.selected_detail.as_ref())
                .map(|detail| detail.id.as_str()),
            Some("ccccc-cc")
        );
    }

    #[test]
    fn settings_tab_navigation_does_not_persist_default_memory() {
        let mut provider = KinicProvider::new(live_config());
        provider.tab_id = KINIC_SETTINGS_TAB_ID.to_string();
        provider.memory_records = vec![
            live_memory("aaaaa-aa", "Alpha Memory"),
            live_memory("bbbbb-bb", "Beta Memory"),
        ];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

        provider
            .handle_action(&CoreAction::MoveNext, &CoreState::default())
            .expect("move next should succeed");

        assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
        assert_eq!(
            provider.user_preferences.default_memory_id.as_deref(),
            Some("aaaaa-aa")
        );
        assert!(provider.preferences_health.save_error.is_none());
    }

    #[test]
    fn build_snapshot_surfaces_preferences_load_error_in_status() {
        let mut provider = KinicProvider::new(live_config());
        provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
        provider.all = provider.memory_records.clone();
        provider.active_memory_id = Some("aaaaa-aa".to_string());
        provider.preferences_health.load_error = Some("invalid YAML".to_string());

        let snapshot = provider.build_snapshot(&CoreState::default());

        assert_eq!(
            snapshot.status_message.as_deref(),
            Some(
                "kinic(live): target aaaaa-aa | j/k selects memory canister | Enter in search runs remote search | preferences load failed: invalid YAML"
            )
        );
        assert_eq!(snapshot.settings.quick_entries[5].value, "preferences unavailable");
    }
}
