//! Shared runtime contracts for the Kinic tui-kit stack.
//!
//! This crate defines common actions/effects, shared runtime state, and the
//! `DataProvider` trait used by the Kinic TUI crates.

pub mod kinic_tabs;

use candid::Nat;
use tui_kit_model::{UiContextNode, UiItemContent, UiItemSummary};

pub const SETTINGS_ENTRY_DEFAULT_MEMORY_ID: &str = "default_memory";

/// Core result type used by provider and reducer contracts.
pub type CoreResult<T> = Result<T, CoreError>;

/// Minimal core error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreError {
    pub message: String,
}

impl CoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CoreError {}

/// Which pane currently receives interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaneFocus {
    #[default]
    Search,
    Items,
    Tabs,
    Content,
    Form,
    Extra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabFocusPolicy {
    pub default_focus: PaneFocus,
    pub allows_search: bool,
    pub allows_items: bool,
    pub allows_tabs: bool,
    pub allows_content: bool,
    pub allows_form: bool,
    pub allows_chat: bool,
}

pub fn tab_focus_policy(tab_id: &str) -> TabFocusPolicy {
    match kinic_tabs::tab_kind(tab_id) {
        kinic_tabs::TabKind::Memories | kinic_tabs::TabKind::Unknown => TabFocusPolicy {
            default_focus: PaneFocus::Search,
            allows_search: true,
            allows_items: true,
            allows_tabs: true,
            allows_content: true,
            allows_form: false,
            allows_chat: true,
        },
        kinic_tabs::TabKind::InsertForm | kinic_tabs::TabKind::CreateForm => TabFocusPolicy {
            default_focus: PaneFocus::Tabs,
            allows_search: false,
            allows_items: false,
            allows_tabs: true,
            allows_content: false,
            allows_form: true,
            allows_chat: true,
        },
        kinic_tabs::TabKind::PlaceholderMarket | kinic_tabs::TabKind::PlaceholderSettings => {
            TabFocusPolicy {
                default_focus: PaneFocus::Tabs,
                allows_search: false,
                allows_items: false,
                allows_tabs: true,
                allows_content: true,
                allows_form: false,
                allows_chat: true,
            }
        }
    }
}

pub fn tab_entry_focus(tab_id: &str) -> Option<PaneFocus> {
    match kinic_tabs::tab_kind(tab_id) {
        kinic_tabs::TabKind::Memories => Some(PaneFocus::Search),
        kinic_tabs::TabKind::InsertForm | kinic_tabs::TabKind::CreateForm => Some(PaneFocus::Form),
        kinic_tabs::TabKind::PlaceholderMarket
        | kinic_tabs::TabKind::PlaceholderSettings
        | kinic_tabs::TabKind::Unknown => Some(PaneFocus::Content),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreateModalFocus {
    #[default]
    Name,
    Description,
    Submit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InsertMode {
    #[default]
    Normal,
    Raw,
    Pdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchScope {
    #[default]
    All,
    Selected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InsertFormFocus {
    #[default]
    Mode,
    MemoryId,
    Tag,
    Text,
    FilePath,
    Embedding,
    Submit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemorySelectorContext {
    #[default]
    DefaultPreference,
    InsertTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemorySelectorItem {
    pub id: String,
    pub title: Option<String>,
}

impl MemorySelectorItem {
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or(self.id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CreateSubmitState {
    #[default]
    Idle,
    Submitting,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CreateCostState {
    #[default]
    Hidden,
    Loading,
    Unavailable,
    Loaded(Box<LoadedCreateCost>),
    Error(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSettingsSnapshot {
    pub auth_mode: String,
    pub identity_name: String,
    pub principal_id: String,
    pub network: String,
    pub embedding_api_endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAccountOverview {
    pub session: SessionSettingsSnapshot,
    pub balance_base_units: Option<u128>,
    pub price_base_units: Option<Nat>,
    pub principal_error: Option<String>,
    pub balance_error: Option<String>,
    pub price_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedCreateCost {
    pub principal: String,
    pub balance_kinic: String,
    pub price_kinic: String,
    pub required_total_kinic: String,
    pub required_total_base_units: String,
    pub difference_kinic: String,
    pub difference_base_units: String,
    pub sufficient_balance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCreateCost {
    pub overview: SessionAccountOverview,
    pub details: Option<DerivedCreateCost>,
}

impl SessionAccountOverview {
    pub fn new(session: SessionSettingsSnapshot) -> Self {
        Self {
            session,
            balance_base_units: None,
            price_base_units: None,
            principal_error: None,
            balance_error: None,
            price_error: None,
        }
    }

    pub fn has_complete_create_cost(&self) -> bool {
        self.balance_base_units.is_some() && self.price_base_units.is_some()
    }

    pub fn account_issue_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        if let Some(error) = &self.principal_error {
            messages.push(format!("Could not derive principal. Cause: {error}"));
        }
        if let Some(error) = &self.balance_error {
            messages.push(format!("Could not fetch KINIC balance. Cause: {error}"));
        }
        if let Some(error) = &self.price_error {
            messages.push(format!("Could not fetch create price. Cause: {error}"));
        }
        messages
    }

    pub fn account_issue_note(&self) -> Option<String> {
        let issues = self.account_issue_messages();
        (!issues.is_empty()).then(|| issues.join(" | "))
    }

    pub fn session_settings_refresh_failure_message(&self) -> Option<String> {
        self.principal_error
            .as_ref()
            .map(|error| format!("Session settings refresh failed: {error}"))
    }

    pub fn session_settings_refresh_notify_message(&self) -> String {
        let account_incomplete = self.principal_error.is_some()
            || self.balance_error.is_some()
            || self.price_error.is_some()
            || !self.has_complete_create_cost();
        if account_incomplete {
            "Session settings updated (partial account info). See Settings → Account.".to_string()
        } else {
            "Session settings refreshed.".to_string()
        }
    }
}

pub fn format_e8s_to_kinic_string_u128(value: u128) -> String {
    format_e8s_to_kinic_string_str(value.to_string().as_str())
}

pub fn format_e8s_to_kinic_string_nat(value: &Nat) -> String {
    format_e8s_to_kinic_string_str(value.to_string().as_str())
}

fn format_e8s_to_kinic_string_str(value: &str) -> String {
    const SCALE: usize = 8;

    let digits = value.replace('_', "");
    if digits.len() <= SCALE {
        return format!("0.{:0>width$}", digits, width = SCALE);
    }

    let split_at = digits.len() - SCALE;
    let (whole, fraction) = digits.split_at(split_at);
    format!("{whole}.{fraction}")
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SettingsEntry {
    pub id: String,
    pub label: String,
    pub value: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SettingsSection {
    pub title: String,
    pub entries: Vec<SettingsEntry>,
    pub footer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SettingsSnapshot {
    pub quick_entries: Vec<SettingsEntry>,
    pub sections: Vec<SettingsSection>,
}

/// Domain-agnostic runtime state owned by the core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreState {
    pub current_tab_id: String,
    pub focus: PaneFocus,
    pub query: String,
    pub selected_index: Option<usize>,
    pub list_items: Vec<UiItemSummary>,
    pub selected_content: Option<UiItemContent>,
    pub selected_context: Option<UiContextNode>,
    pub total_count: usize,
    pub status_message: Option<String>,
    pub chat_open: bool,
    pub chat_messages: Vec<(String, String)>,
    pub chat_input: String,
    pub chat_loading: bool,
    pub chat_scroll: usize,
    pub create_name: String,
    pub create_description: String,
    pub create_submit_state: CreateSubmitState,
    pub create_spinner_frame: usize,
    pub create_error: Option<String>,
    pub create_focus: CreateModalFocus,
    pub create_cost_state: CreateCostState,
    pub settings: SettingsSnapshot,
    pub default_memory_selector_open: bool,
    pub default_memory_selector_index: usize,
    pub default_memory_selector_items: Vec<MemorySelectorItem>,
    pub default_memory_selector_selected_id: Option<String>,
    pub saved_default_memory_id: Option<String>,
    pub default_memory_selector_context: MemorySelectorContext,
    pub search_scope: SearchScope,
    pub insert_mode: InsertMode,
    pub insert_memory_id: String,
    pub insert_memory_placeholder: Option<String>,
    pub insert_tag: String,
    pub insert_text: String,
    pub insert_file_path: String,
    pub insert_embedding: String,
    pub insert_submit_state: CreateSubmitState,
    pub insert_spinner_frame: usize,
    pub insert_error: Option<String>,
    pub insert_focus: InsertFormFocus,
}

impl Default for CoreState {
    fn default() -> Self {
        Self {
            current_tab_id: "default".to_string(),
            focus: PaneFocus::Search,
            query: String::new(),
            selected_index: None,
            list_items: Vec::new(),
            selected_content: None,
            selected_context: None,
            total_count: 0,
            status_message: None,
            chat_open: false,
            chat_messages: Vec::new(),
            chat_input: String::new(),
            chat_loading: false,
            chat_scroll: 0,
            create_name: String::new(),
            create_description: String::new(),
            create_submit_state: CreateSubmitState::default(),
            create_spinner_frame: 0,
            create_error: None,
            create_focus: CreateModalFocus::default(),
            create_cost_state: CreateCostState::default(),
            settings: SettingsSnapshot::default(),
            default_memory_selector_open: false,
            default_memory_selector_index: 0,
            default_memory_selector_items: Vec::new(),
            default_memory_selector_selected_id: None,
            saved_default_memory_id: None,
            default_memory_selector_context: MemorySelectorContext::default(),
            search_scope: SearchScope::default(),
            insert_mode: InsertMode::default(),
            insert_memory_id: String::new(),
            insert_memory_placeholder: None,
            insert_tag: String::new(),
            insert_text: String::new(),
            insert_file_path: String::new(),
            insert_embedding: String::new(),
            insert_submit_state: CreateSubmitState::default(),
            insert_spinner_frame: 0,
            insert_error: None,
            insert_focus: InsertFormFocus::default(),
        }
    }
}

/// Common action set used across domains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreAction {
    MoveNext,
    MovePrev,
    MovePageDown,
    MovePageUp,
    MoveHome,
    MoveEnd,
    ScrollContentPageDown,
    ScrollContentPageUp,
    ScrollContentHome,
    ScrollContentEnd,
    FocusNext,
    FocusPrev,
    FocusSearch,
    FocusItems,
    FocusContent,
    FocusForm,
    OpenSelected,
    Back,
    SearchInput(char),
    SearchBackspace,
    SearchSubmit,
    SearchScopePrev,
    SearchScopeNext,
    SetQuery(String),
    SelectTabIndex(usize),
    SelectNextTab,
    SelectPrevTab,
    SetTab(CoreTabId),
    ToggleHelp,
    ToggleSettings,
    ToggleChat,
    OpenDefaultMemoryPicker,
    CloseDefaultMemoryPicker,
    MoveDefaultMemoryPickerNext,
    MoveDefaultMemoryPickerPrev,
    SubmitDefaultMemoryPicker,
    SetDefaultMemoryFromSelection,
    CreateInput(char),
    CreateBackspace,
    CreateNextField,
    CreatePrevField,
    CreateRefresh,
    RefreshCurrentView,
    CreateSubmit,
    InsertInput(char),
    InsertBackspace,
    InsertNextField,
    InsertPrevField,
    InsertCycleModePrev,
    InsertCycleMode,
    InsertSubmit,
    Submit,
    Cancel,
    ChatInput(char),
    ChatBackspace,
    ChatSubmit,
    ChatScrollUp,
    ChatScrollDown,
    ChatScrollPageUp,
    ChatScrollPageDown,
    ChatScrollHome,
    ChatScrollEnd,
    OpenExternal(String),
    Custom(CustomAction),
}

/// Opaque runtime tab id to avoid raw string coupling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoreTabId(pub String);

impl CoreTabId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<&str> for CoreTabId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CoreTabId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Extension action for domain-specific behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomAction {
    pub id: String,
    pub payload: Option<String>,
}

impl CustomAction {
    pub fn new(id: impl Into<String>, payload: Option<String>) -> Self {
        Self {
            id: id.into(),
            payload,
        }
    }
}

/// Side effects to execute outside pure reducers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreEffect {
    OpenExternal(String),
    Notify(String),
    RequestRefresh,
    /// Validation or async error for the create form (clears submitting state).
    CreateFormError(Option<String>),
    /// Validation or async error for the insert form (clears submitting state).
    InsertFormError(Option<String>),
    /// Select the first row in the list (no-op when empty).
    SelectFirstListItem,
    /// Move keyboard focus to a pane.
    FocusPane(PaneFocus),
    /// Clear create form fields and switch the active tab (e.g. after successful create).
    ResetCreateFormAndSetTab {
        tab_id: String,
    },
    /// Clear insert content fields while keeping target selection for repeated inserts.
    ResetInsertFormForRepeat,
    /// Apply a selector-picked insert target without routing it through text input.
    SetInsertMemoryId(String),
    /// Escape hatch for domain-specific integrations (examples, experiments).
    Custom {
        id: String,
        payload: Option<String>,
    },
}

/// Provider-owned snapshot sent to core/UI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderSnapshot {
    pub items: Vec<UiItemSummary>,
    pub selected_content: Option<UiItemContent>,
    pub selected_context: Option<UiContextNode>,
    pub total_count: usize,
    pub status_message: Option<String>,
    pub create_cost_state: CreateCostState,
    pub create_submit_state: CreateSubmitState,
    pub settings: SettingsSnapshot,
    pub default_memory_selector_items: Vec<MemorySelectorItem>,
    pub default_memory_selector_selected_id: Option<String>,
    pub saved_default_memory_id: Option<String>,
    pub default_memory_selector_context: MemorySelectorContext,
    pub insert_memory_placeholder: Option<String>,
}

/// Provider response to one action.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderOutput {
    pub snapshot: Option<ProviderSnapshot>,
    pub effects: Vec<CoreEffect>,
}

#[cfg(test)]
mod formatter_tests {
    use super::{format_e8s_to_kinic_string_nat, format_e8s_to_kinic_string_u128};
    use candid::Nat;

    #[test]
    fn format_e8s_to_kinic_string_u128_keeps_eight_fraction_digits() {
        assert_eq!(
            format_e8s_to_kinic_string_u128(123_456_789u128),
            "1.23456789"
        );
        assert_eq!(format_e8s_to_kinic_string_u128(42u128), "0.00000042");
    }

    #[test]
    fn format_e8s_to_kinic_string_nat_supports_values_larger_than_u128() {
        let large = Nat::parse(b"340282366920938463463374607431768211456").expect("valid Nat");

        assert_eq!(
            format_e8s_to_kinic_string_nat(&large),
            "3402823669209384634633746074317.68211456"
        );
    }
}

/// Input key abstraction for shared key->action mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreKey {
    Char(char),
    Slash,
    Tab,
    BackTab,
    Backspace,
    Enter,
    Down,
    Up,
    Left,
    Right,
    PageDown,
    PageUp,
    Home,
    End,
}

/// Domain plugin contract.
///
/// Implement this trait for each domain (e.g. Rust code, Mail, Task manager).
pub trait DataProvider {
    /// Build initial snapshot when app starts.
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot>;

    /// Handle one action in a domain-specific manner.
    fn handle_action(
        &mut self,
        action: &CoreAction,
        state: &CoreState,
    ) -> CoreResult<ProviderOutput>;

    fn poll_background(&mut self, _state: &CoreState) -> Option<ProviderOutput> {
        None
    }
}

/// Apply one core action to local runtime state.
///
/// Providers may further modify visible data via snapshots; this reducer handles
/// local interaction state (query, tab, focus, selection).
pub fn apply_core_action(state: &mut CoreState, action: &CoreAction) {
    let has_tabs = !state.current_tab_id.is_empty();
    let previous_focus = state.focus;
    match action {
        CoreAction::InsertInput(c) => {
            if insert_form_locked(state) {
                return;
            }
            match state.insert_focus {
                InsertFormFocus::Mode | InsertFormFocus::MemoryId | InsertFormFocus::Submit => {}
                InsertFormFocus::Tag => state.insert_tag.push(*c),
                InsertFormFocus::Text => state.insert_text.push(*c),
                InsertFormFocus::FilePath => state.insert_file_path.push(*c),
                InsertFormFocus::Embedding => state.insert_embedding.push(*c),
            }
            state.insert_error = None;
            if state.insert_submit_state == CreateSubmitState::Error {
                state.insert_submit_state = CreateSubmitState::Idle;
            }
        }
        CoreAction::InsertBackspace => {
            if insert_form_locked(state) {
                return;
            }
            match state.insert_focus {
                InsertFormFocus::Mode | InsertFormFocus::MemoryId | InsertFormFocus::Submit => {}
                InsertFormFocus::Tag => {
                    state.insert_tag.pop();
                }
                InsertFormFocus::Text => {
                    state.insert_text.pop();
                }
                InsertFormFocus::FilePath => {
                    state.insert_file_path.pop();
                }
                InsertFormFocus::Embedding => {
                    state.insert_embedding.pop();
                }
            }
        }
        CoreAction::InsertNextField => {
            if insert_form_locked(state) {
                return;
            }
            state.insert_focus = next_insert_focus(state.insert_mode, state.insert_focus);
        }
        CoreAction::InsertPrevField => {
            if insert_form_locked(state) {
                return;
            }
            state.insert_focus = prev_insert_focus(state.insert_mode, state.insert_focus);
        }
        CoreAction::InsertCycleModePrev => {
            if insert_form_locked(state) {
                return;
            }
            state.insert_mode = prev_insert_mode(state.insert_mode);
            state.insert_focus = InsertFormFocus::Mode;
            state.insert_error = None;
            if state.insert_submit_state == CreateSubmitState::Error {
                state.insert_submit_state = CreateSubmitState::Idle;
            }
        }
        CoreAction::InsertCycleMode => {
            if insert_form_locked(state) {
                return;
            }
            state.insert_mode = next_insert_mode(state.insert_mode);
            state.insert_focus = InsertFormFocus::Mode;
            state.insert_error = None;
            if state.insert_submit_state == CreateSubmitState::Error {
                state.insert_submit_state = CreateSubmitState::Idle;
            }
        }
        CoreAction::InsertSubmit => {
            state.insert_submit_state = CreateSubmitState::Submitting;
            state.insert_spinner_frame = 0;
            state.insert_error = None;
        }
        CoreAction::CreateInput(c) => {
            match state.create_focus {
                CreateModalFocus::Name => state.create_name.push(*c),
                CreateModalFocus::Description => state.create_description.push(*c),
                CreateModalFocus::Submit => {}
            }
            state.create_error = None;
            if state.create_submit_state == CreateSubmitState::Error {
                state.create_submit_state = CreateSubmitState::Idle;
            }
        }
        CoreAction::CreateBackspace => match state.create_focus {
            CreateModalFocus::Name => {
                state.create_name.pop();
            }
            CreateModalFocus::Description => {
                state.create_description.pop();
            }
            CreateModalFocus::Submit => {}
        },
        CoreAction::CreateNextField => {
            state.create_focus = match state.create_focus {
                CreateModalFocus::Name => CreateModalFocus::Description,
                CreateModalFocus::Description => CreateModalFocus::Submit,
                CreateModalFocus::Submit => CreateModalFocus::Name,
            };
        }
        CoreAction::CreatePrevField => {
            state.create_focus = match state.create_focus {
                CreateModalFocus::Name => CreateModalFocus::Submit,
                CreateModalFocus::Description => CreateModalFocus::Name,
                CreateModalFocus::Submit => CreateModalFocus::Description,
            };
        }
        CoreAction::CreateRefresh => {
            state.create_cost_state = CreateCostState::Loading;
            state.create_spinner_frame = 0;
        }
        CoreAction::CreateSubmit => {
            state.create_submit_state = CreateSubmitState::Submitting;
            state.create_spinner_frame = 0;
            state.create_error = None;
        }
        CoreAction::SetQuery(q) => {
            state.query = q.clone();
            state.selected_index = Some(0);
        }
        CoreAction::SearchInput(c) => {
            state.query.push(*c);
            state.selected_index = Some(0);
        }
        CoreAction::SearchBackspace => {
            state.query.pop();
            state.selected_index = Some(0);
        }
        CoreAction::SearchScopePrev => {
            if state.current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID {
                state.search_scope = prev_search_scope(state.search_scope);
            }
        }
        CoreAction::SearchScopeNext => {
            if state.current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID {
                state.search_scope = next_search_scope(state.search_scope);
            }
        }
        CoreAction::SetTab(tab_id) => {
            state.current_tab_id = tab_id.0.clone();
            state.selected_index = Some(0);
            state.default_memory_selector_open = false;
        }
        CoreAction::SelectTabIndex(index) => {
            state.current_tab_id = format!("tab-{}", index + 1);
            state.selected_index = Some(0);
        }
        CoreAction::FocusNext => {
            state.focus = match state.focus {
                PaneFocus::Search => PaneFocus::Items,
                PaneFocus::Items => PaneFocus::Content,
                PaneFocus::Content => {
                    if state.chat_open {
                        PaneFocus::Extra
                    } else if has_tabs {
                        PaneFocus::Tabs
                    } else {
                        PaneFocus::Search
                    }
                }
                PaneFocus::Form => PaneFocus::Tabs,
                PaneFocus::Extra => {
                    if has_tabs {
                        PaneFocus::Tabs
                    } else {
                        PaneFocus::Search
                    }
                }
                PaneFocus::Tabs => PaneFocus::Search,
            };
        }
        CoreAction::FocusPrev => {
            state.focus = match state.focus {
                PaneFocus::Search => {
                    if has_tabs {
                        PaneFocus::Tabs
                    } else if state.chat_open {
                        PaneFocus::Extra
                    } else {
                        PaneFocus::Content
                    }
                }
                PaneFocus::Items => PaneFocus::Search,
                PaneFocus::Content => PaneFocus::Items,
                PaneFocus::Form => PaneFocus::Tabs,
                PaneFocus::Extra => PaneFocus::Content,
                PaneFocus::Tabs => {
                    if state.chat_open {
                        PaneFocus::Extra
                    } else {
                        PaneFocus::Content
                    }
                }
            };
        }
        CoreAction::FocusSearch => state.focus = PaneFocus::Search,
        CoreAction::FocusItems => state.focus = PaneFocus::Items,
        CoreAction::FocusContent => state.focus = PaneFocus::Content,
        CoreAction::FocusForm => {
            state.focus = PaneFocus::Form;
            match kinic_tabs::tab_kind(state.current_tab_id.as_str()) {
                kinic_tabs::TabKind::CreateForm => {
                    state.create_focus = CreateModalFocus::Name;
                }
                kinic_tabs::TabKind::InsertForm => {
                    state.insert_focus = InsertFormFocus::Mode;
                }
                _ => {}
            }
        }
        CoreAction::OpenSelected => state.focus = PaneFocus::Content,
        CoreAction::Back => {
            state.focus = if state.focus == PaneFocus::Extra {
                PaneFocus::Content
            } else {
                PaneFocus::Items
            };
        }
        CoreAction::ToggleChat => {
            state.chat_open = !state.chat_open;
            if state.chat_open {
                state.focus = PaneFocus::Extra;
            } else if state.focus == PaneFocus::Extra {
                state.focus = focus_after_chat_close(state.current_tab_id.as_str());
            }
        }
        CoreAction::OpenDefaultMemoryPicker => {
            state.default_memory_selector_context = default_memory_selector_context(state);
            if state.default_memory_selector_context == MemorySelectorContext::InsertTarget {
                let insert_memory_id = state.insert_memory_id.trim();
                if !insert_memory_id.is_empty() {
                    state.default_memory_selector_selected_id = Some(insert_memory_id.to_string());
                }
            }
            state.default_memory_selector_open = true;
            state.default_memory_selector_index = state
                .default_memory_selector_selected_id
                .as_ref()
                .and_then(|selected| {
                    state
                        .default_memory_selector_items
                        .iter()
                        .position(|item| item.id == *selected)
                })
                .unwrap_or(0);
        }
        CoreAction::CloseDefaultMemoryPicker | CoreAction::SubmitDefaultMemoryPicker => {
            state.default_memory_selector_open = false;
        }
        CoreAction::MoveDefaultMemoryPickerNext => {
            let len = state.default_memory_selector_items.len();
            if len != 0 {
                state.default_memory_selector_index =
                    (state.default_memory_selector_index + 1).min(len - 1);
            }
        }
        CoreAction::MoveDefaultMemoryPickerPrev => {
            state.default_memory_selector_index =
                state.default_memory_selector_index.saturating_sub(1);
        }
        CoreAction::ChatInput(c) => {
            state.chat_input.push(*c);
        }
        CoreAction::ChatBackspace => {
            state.chat_input.pop();
        }
        CoreAction::ChatSubmit => {
            let input = state.chat_input.trim().to_string();
            if !input.is_empty() {
                state.chat_messages.push(("user".to_string(), input));
                state.chat_input.clear();
                state.chat_loading = true;
            }
        }
        CoreAction::ChatScrollUp => state.chat_scroll = state.chat_scroll.saturating_sub(1),
        CoreAction::ChatScrollDown => state.chat_scroll = state.chat_scroll.saturating_add(1),
        CoreAction::ChatScrollPageUp => state.chat_scroll = state.chat_scroll.saturating_sub(10),
        CoreAction::ChatScrollPageDown => state.chat_scroll = state.chat_scroll.saturating_add(10),
        CoreAction::ChatScrollHome => state.chat_scroll = 0,
        CoreAction::ChatScrollEnd => state.chat_scroll = state.chat_scroll.saturating_add(9999),
        CoreAction::MoveNext => {
            let len = selectable_len(state);
            if len == 0 {
                state.selected_index = None;
            } else {
                let idx = state.selected_index.unwrap_or(0);
                state.selected_index = Some((idx + 1).min(len - 1));
            }
        }
        CoreAction::MovePrev => {
            let len = selectable_len(state);
            if len == 0 {
                state.selected_index = None;
            } else {
                let idx = state.selected_index.unwrap_or(0);
                state.selected_index = Some(idx.saturating_sub(1));
            }
        }
        CoreAction::MoveHome => {
            state.selected_index = if selectable_len(state) == 0 {
                None
            } else {
                Some(0)
            };
        }
        CoreAction::MoveEnd => {
            let len = selectable_len(state);
            state.selected_index = if len == 0 { None } else { Some(len - 1) };
        }
        CoreAction::MovePageDown => {
            let len = selectable_len(state);
            if len == 0 {
                state.selected_index = None;
            } else {
                let idx = state.selected_index.unwrap_or(0);
                state.selected_index = Some((idx + 10).min(len - 1));
            }
        }
        CoreAction::MovePageUp => {
            let len = selectable_len(state);
            if len == 0 {
                state.selected_index = None;
            } else {
                let idx = state.selected_index.unwrap_or(0);
                state.selected_index = Some(idx.saturating_sub(10));
            }
        }
        _ => {}
    }

    normalize_focus_for_tab(state, previous_focus);
}

fn normalize_focus_for_tab(state: &mut CoreState, previous_focus: PaneFocus) {
    let policy = tab_focus_policy(state.current_tab_id.as_str());

    if is_focus_allowed_for_policy(policy, state.focus) {
        return;
    }

    if is_focus_allowed_for_policy(policy, previous_focus) {
        state.focus = previous_focus;
        return;
    }

    state.focus = default_focus_for_policy(policy, state.chat_open);
}

fn insert_focus_order(mode: InsertMode) -> &'static [InsertFormFocus] {
    match mode {
        InsertMode::Normal => &[
            InsertFormFocus::Mode,
            InsertFormFocus::MemoryId,
            InsertFormFocus::Tag,
            InsertFormFocus::Text,
            InsertFormFocus::FilePath,
            InsertFormFocus::Submit,
        ],
        InsertMode::Raw => &[
            InsertFormFocus::Mode,
            InsertFormFocus::MemoryId,
            InsertFormFocus::Tag,
            InsertFormFocus::Text,
            InsertFormFocus::Embedding,
            InsertFormFocus::Submit,
        ],
        InsertMode::Pdf => &[
            InsertFormFocus::Mode,
            InsertFormFocus::MemoryId,
            InsertFormFocus::Tag,
            InsertFormFocus::FilePath,
            InsertFormFocus::Submit,
        ],
    }
}

fn next_insert_focus(mode: InsertMode, focus: InsertFormFocus) -> InsertFormFocus {
    let order = insert_focus_order(mode);
    let current = order
        .iter()
        .position(|candidate| *candidate == focus)
        .unwrap_or(0);
    order[(current + 1) % order.len()]
}

fn prev_insert_focus(mode: InsertMode, focus: InsertFormFocus) -> InsertFormFocus {
    let order = insert_focus_order(mode);
    let current = order
        .iter()
        .position(|candidate| *candidate == focus)
        .unwrap_or(0);
    order[(current + order.len() - 1) % order.len()]
}

fn next_insert_mode(mode: InsertMode) -> InsertMode {
    match mode {
        InsertMode::Normal => InsertMode::Raw,
        InsertMode::Raw => InsertMode::Pdf,
        InsertMode::Pdf => InsertMode::Normal,
    }
}

fn prev_insert_mode(mode: InsertMode) -> InsertMode {
    match mode {
        InsertMode::Normal => InsertMode::Pdf,
        InsertMode::Raw => InsertMode::Normal,
        InsertMode::Pdf => InsertMode::Raw,
    }
}

fn next_search_scope(scope: SearchScope) -> SearchScope {
    match scope {
        SearchScope::All => SearchScope::Selected,
        SearchScope::Selected => SearchScope::All,
    }
}

fn prev_search_scope(scope: SearchScope) -> SearchScope {
    next_search_scope(scope)
}

fn insert_form_locked(state: &CoreState) -> bool {
    state.insert_submit_state == CreateSubmitState::Submitting
}

fn is_focus_allowed_for_policy(policy: TabFocusPolicy, focus: PaneFocus) -> bool {
    match focus {
        PaneFocus::Search => policy.allows_search,
        PaneFocus::Items => policy.allows_items,
        PaneFocus::Tabs => policy.allows_tabs,
        PaneFocus::Content => policy.allows_content,
        PaneFocus::Form => policy.allows_form,
        PaneFocus::Extra => policy.allows_chat,
    }
}

fn default_focus_for_policy(policy: TabFocusPolicy, chat_open: bool) -> PaneFocus {
    if chat_open {
        return PaneFocus::Extra;
    }

    policy.default_focus
}

fn focus_after_chat_close(tab_id: &str) -> PaneFocus {
    let policy = tab_focus_policy(tab_id);

    if policy.allows_form {
        return PaneFocus::Form;
    }
    if policy.allows_content {
        return PaneFocus::Content;
    }
    if policy.allows_items {
        return PaneFocus::Items;
    }

    policy.default_focus
}

/// Dispatch one action through local reducer + provider + snapshot merge.
///
/// Returns provider effects for callers that need side-effect execution.
pub fn dispatch_action(
    provider: &mut impl DataProvider,
    state: &mut CoreState,
    action: &CoreAction,
) -> CoreResult<Vec<CoreEffect>> {
    apply_core_action(state, action);
    let out = provider.handle_action(action, state)?;
    if let Some(snapshot) = out.snapshot {
        apply_snapshot(state, snapshot);
    }
    Ok(out.effects)
}

/// Shared focus-aware keymap from abstract keys to core actions.
pub fn action_for_key(key: CoreKey, focus: PaneFocus, current_tab_id: &str) -> Option<CoreAction> {
    if focus == PaneFocus::Tabs {
        return match key {
            CoreKey::Up => Some(CoreAction::SelectPrevTab),
            CoreKey::Down => Some(CoreAction::SelectNextTab),
            CoreKey::Left | CoreKey::Char('h') => None,
            CoreKey::Tab | CoreKey::Right | CoreKey::Char('l') | CoreKey::Enter => {
                tab_entry_focus(current_tab_id).map(|focus| match focus {
                    PaneFocus::Search => CoreAction::FocusSearch,
                    PaneFocus::Items => CoreAction::FocusItems,
                    PaneFocus::Tabs => CoreAction::FocusNext,
                    PaneFocus::Content => CoreAction::FocusContent,
                    PaneFocus::Form => CoreAction::FocusForm,
                    PaneFocus::Extra => CoreAction::ToggleChat,
                })
            }
            CoreKey::BackTab => Some(CoreAction::FocusPrev),
            CoreKey::Char(c) if c.is_ascii_digit() && c != '0' => {
                let idx = (c as u8 - b'1') as usize;
                Some(CoreAction::SelectTabIndex(idx))
            }
            _ => None,
        };
    }

    match key {
        CoreKey::Slash => Some(CoreAction::FocusSearch),
        CoreKey::Tab => Some(CoreAction::FocusNext),
        CoreKey::BackTab => Some(CoreAction::FocusPrev),
        CoreKey::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as u8 - b'1') as usize;
            Some(CoreAction::SelectTabIndex(idx))
        }
        _ => match focus {
            PaneFocus::Search => match key {
                CoreKey::Backspace => Some(CoreAction::SearchBackspace),
                CoreKey::Enter => Some(CoreAction::SearchSubmit),
                CoreKey::Left if current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID => {
                    Some(CoreAction::SearchScopePrev)
                }
                CoreKey::Right if current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID => {
                    Some(CoreAction::SearchScopeNext)
                }
                CoreKey::Down => Some(CoreAction::FocusItems),
                CoreKey::Char(c) if !c.is_control() => Some(CoreAction::SearchInput(c)),
                _ => None,
            },
            PaneFocus::Items => match key {
                CoreKey::Down => Some(CoreAction::MoveNext),
                CoreKey::Up => Some(CoreAction::MovePrev),
                CoreKey::PageDown => Some(CoreAction::MovePageDown),
                CoreKey::PageUp => Some(CoreAction::MovePageUp),
                CoreKey::Home | CoreKey::Char('g') => Some(CoreAction::MoveHome),
                CoreKey::End | CoreKey::Char('G') => Some(CoreAction::MoveEnd),
                CoreKey::Enter | CoreKey::Right | CoreKey::Char('l') => {
                    Some(CoreAction::OpenSelected)
                }
                _ => None,
            },
            PaneFocus::Tabs => None,
            PaneFocus::Content => match key {
                CoreKey::Enter if is_settings_content(current_tab_id, PaneFocus::Content) => None,
                CoreKey::Left | CoreKey::Char('h') => Some(CoreAction::Back),
                _ if is_settings_content(current_tab_id, PaneFocus::Content) => {
                    settings_content_action_for_key(key)
                }
                CoreKey::Down => Some(CoreAction::ScrollContentPageDown),
                CoreKey::Up => Some(CoreAction::ScrollContentPageUp),
                CoreKey::PageDown => Some(CoreAction::ScrollContentPageDown),
                CoreKey::PageUp => Some(CoreAction::ScrollContentPageUp),
                CoreKey::Home | CoreKey::Char('g') => Some(CoreAction::ScrollContentHome),
                CoreKey::End | CoreKey::Char('G') => Some(CoreAction::ScrollContentEnd),
                _ => None,
            },
            PaneFocus::Form => None,
            PaneFocus::Extra => match key {
                CoreKey::Backspace => Some(CoreAction::ChatBackspace),
                CoreKey::Enter => Some(CoreAction::ChatSubmit),
                CoreKey::Down => Some(CoreAction::ChatScrollDown),
                CoreKey::Up => Some(CoreAction::ChatScrollUp),
                CoreKey::PageDown => Some(CoreAction::ChatScrollPageDown),
                CoreKey::PageUp => Some(CoreAction::ChatScrollPageUp),
                CoreKey::Home => Some(CoreAction::ChatScrollHome),
                CoreKey::End => Some(CoreAction::ChatScrollEnd),
                CoreKey::Char(c) if !c.is_control() => Some(CoreAction::ChatInput(c)),
                _ => None,
            },
        },
    }
}

/// Apply a new snapshot to core runtime state.
pub fn apply_snapshot(state: &mut CoreState, snapshot: ProviderSnapshot) {
    state.list_items = snapshot.items;
    state.selected_content = snapshot.selected_content;
    state.selected_context = snapshot.selected_context;
    state.total_count = snapshot.total_count;
    state.status_message = snapshot.status_message;
    state.create_cost_state = snapshot.create_cost_state;
    state.create_submit_state = snapshot.create_submit_state;
    state.settings = snapshot.settings;
    state.default_memory_selector_items = snapshot.default_memory_selector_items;
    state.default_memory_selector_selected_id = snapshot.default_memory_selector_selected_id;
    state.saved_default_memory_id = snapshot.saved_default_memory_id;
    state.default_memory_selector_context = snapshot.default_memory_selector_context;
    state.insert_memory_placeholder = snapshot.insert_memory_placeholder;
    if state.current_tab_id == kinic_tabs::KINIC_INSERT_TAB_ID && state.insert_memory_id.is_empty()
    {
        state.insert_memory_id = state.saved_default_memory_id.clone().unwrap_or_default();
    }
    if state.default_memory_selector_items.is_empty() {
        state.default_memory_selector_index = 0;
    } else if state.default_memory_selector_index >= state.default_memory_selector_items.len() {
        state.default_memory_selector_index = state.default_memory_selector_items.len() - 1;
    }

    let selectable_len = selectable_len(state);
    if let Some(sel) = state.selected_index {
        if sel >= selectable_len {
            state.selected_index = if selectable_len == 0 { None } else { Some(0) };
        }
    } else if selectable_len != 0 {
        state.selected_index = Some(0);
    }
}

fn selectable_len(state: &CoreState) -> usize {
    if is_settings_content(state.current_tab_id.as_str(), state.focus) {
        return settings_selectable_len(&state.settings);
    }

    item_selectable_len(state)
}

fn item_selectable_len(state: &CoreState) -> usize {
    state.list_items.len()
}

fn settings_selectable_len(settings: &SettingsSnapshot) -> usize {
    settings_entry_count(settings)
}

fn settings_entry_count(settings: &SettingsSnapshot) -> usize {
    settings
        .sections
        .iter()
        .map(|section| section.entries.len())
        .sum()
}

pub fn settings_entry(settings: &SettingsSnapshot, index: usize) -> Option<&SettingsEntry> {
    let mut remaining = index;
    for section in &settings.sections {
        if remaining < section.entries.len() {
            return section.entries.get(remaining);
        }
        remaining = remaining.saturating_sub(section.entries.len());
    }
    None
}

pub fn should_open_default_memory_picker(state: &CoreState) -> bool {
    is_settings_content(state.current_tab_id.as_str(), state.focus)
        && state
            .selected_index
            .and_then(|index| settings_entry(&state.settings, index))
            .map(|entry| entry.id.as_str())
            == Some(SETTINGS_ENTRY_DEFAULT_MEMORY_ID)
}

fn is_settings_content(current_tab_id: &str, focus: PaneFocus) -> bool {
    current_tab_id == kinic_tabs::KINIC_SETTINGS_TAB_ID && focus == PaneFocus::Content
}

fn settings_content_action_for_key(key: CoreKey) -> Option<CoreAction> {
    match key {
        CoreKey::Down => Some(CoreAction::MoveNext),
        CoreKey::Up => Some(CoreAction::MovePrev),
        CoreKey::PageDown => Some(CoreAction::MovePageDown),
        CoreKey::PageUp => Some(CoreAction::MovePageUp),
        CoreKey::Home | CoreKey::Char('g') => Some(CoreAction::MoveHome),
        CoreKey::End | CoreKey::Char('G') => Some(CoreAction::MoveEnd),
        _ => None,
    }
}

fn default_memory_selector_context(state: &CoreState) -> MemorySelectorContext {
    if state.current_tab_id == kinic_tabs::KINIC_INSERT_TAB_ID
        && state.focus == PaneFocus::Form
        && state.insert_focus == InsertFormFocus::MemoryId
    {
        return MemorySelectorContext::InsertTarget;
    }

    MemorySelectorContext::DefaultPreference
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_snapshot_sets_total_and_selection() {
        let mut state = CoreState::default();
        let snapshot = ProviderSnapshot {
            items: vec![UiItemSummary {
                id: "1".to_string(),
                name: "item".to_string(),
                leading_marker: None,
                kind: tui_kit_model::UiItemKind::Custom("x".to_string()),
                visibility: tui_kit_model::UiVisibility::Private,
                qualified_name: None,
                subtitle: None,
                tags: vec![],
            }],
            selected_content: None,
            selected_context: None,
            total_count: 1,
            status_message: Some("ok".to_string()),
            ..ProviderSnapshot::default()
        };

        apply_snapshot(&mut state, snapshot);
        assert_eq!(state.total_count, 1);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn test_apply_core_action_updates_selection() {
        let mut state = CoreState {
            list_items: vec![
                UiItemSummary {
                    id: "1".to_string(),
                    name: "a".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("x".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "2".to_string(),
                    name: "b".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("x".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
            ],
            selected_index: Some(0),
            ..CoreState::default()
        };
        apply_core_action(&mut state, &CoreAction::MoveNext);
        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn test_dispatch_action_applies_provider_snapshot() {
        struct DispatchTestProvider;

        impl DataProvider for DispatchTestProvider {
            fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
                Ok(ProviderSnapshot {
                    total_count: 1,
                    ..ProviderSnapshot::default()
                })
            }

            fn handle_action(
                &mut self,
                _action: &CoreAction,
                _state: &CoreState,
            ) -> CoreResult<ProviderOutput> {
                Ok(ProviderOutput::default())
            }
        }

        let mut provider = DispatchTestProvider;
        let mut state = CoreState::default();
        let effects = dispatch_action(&mut provider, &mut state, &CoreAction::FocusItems).unwrap();
        assert!(effects.is_empty());
    }

    #[test]
    fn toggle_chat_returns_to_create_focus_on_create_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ToggleChat);
        assert_eq!(state.focus, PaneFocus::Extra);

        apply_core_action(&mut state, &CoreAction::ToggleChat);
        assert_eq!(state.focus, PaneFocus::Form);
    }

    #[test]
    fn focus_search_is_blocked_on_create_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Tabs,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusSearch);
        assert_eq!(state.focus, PaneFocus::Tabs);
    }

    #[test]
    fn focus_next_stays_visible_on_placeholder_tabs() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MARKET_TAB_ID.to_string(),
            focus: PaneFocus::Tabs,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusNext);
        assert_eq!(state.focus, PaneFocus::Tabs);
    }

    #[test]
    fn focus_prev_is_clamped_on_create_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Tabs,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusPrev);
        assert_eq!(state.focus, PaneFocus::Tabs);
    }

    #[test]
    fn back_is_clamped_on_create_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::Back);
        assert_eq!(state.focus, PaneFocus::Form);
    }

    #[test]
    fn create_next_field_cycles_back_to_name() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            create_focus: CreateModalFocus::Submit,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::CreateNextField);
        assert_eq!(state.create_focus, CreateModalFocus::Name);
    }

    #[test]
    fn tabs_enter_targets_search_on_memories() {
        assert_eq!(
            action_for_key(
                CoreKey::Enter,
                PaneFocus::Tabs,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::FocusSearch)
        );
    }

    #[test]
    fn tabs_tab_targets_search_on_memories() {
        assert_eq!(
            action_for_key(
                CoreKey::Tab,
                PaneFocus::Tabs,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::FocusSearch)
        );
    }

    #[test]
    fn tabs_jk_no_longer_switch_tabs() {
        assert_eq!(
            action_for_key(
                CoreKey::Char('j'),
                PaneFocus::Tabs,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            None
        );
        assert_eq!(
            action_for_key(
                CoreKey::Char('k'),
                PaneFocus::Tabs,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            None
        );
    }

    #[test]
    fn tabs_enter_targets_form_on_create() {
        assert_eq!(
            action_for_key(
                CoreKey::Enter,
                PaneFocus::Tabs,
                kinic_tabs::KINIC_CREATE_TAB_ID
            ),
            Some(CoreAction::FocusForm)
        );
    }

    #[test]
    fn tabs_tab_targets_form_on_create() {
        assert_eq!(
            action_for_key(
                CoreKey::Tab,
                PaneFocus::Tabs,
                kinic_tabs::KINIC_CREATE_TAB_ID
            ),
            Some(CoreAction::FocusForm)
        );
    }

    #[test]
    fn tabs_enter_targets_detail_on_placeholder() {
        assert_eq!(
            action_for_key(
                CoreKey::Enter,
                PaneFocus::Tabs,
                kinic_tabs::KINIC_MARKET_TAB_ID
            ),
            Some(CoreAction::FocusContent)
        );
    }

    #[test]
    fn tabs_tab_targets_detail_on_placeholder() {
        assert_eq!(
            action_for_key(
                CoreKey::Tab,
                PaneFocus::Tabs,
                kinic_tabs::KINIC_MARKET_TAB_ID
            ),
            Some(CoreAction::FocusContent)
        );
    }

    #[test]
    fn tabs_left_does_not_exit_tab_bar() {
        assert_eq!(
            action_for_key(
                CoreKey::Left,
                PaneFocus::Tabs,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            None
        );
        assert_eq!(
            action_for_key(
                CoreKey::Char('h'),
                PaneFocus::Tabs,
                kinic_tabs::KINIC_CREATE_TAB_ID
            ),
            None
        );
    }

    #[test]
    fn focus_form_resets_create_entry_to_name() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Tabs,
            create_focus: CreateModalFocus::Submit,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusForm);

        assert_eq!(state.focus, PaneFocus::Form);
        assert_eq!(state.create_focus, CreateModalFocus::Name);
    }

    #[test]
    fn settings_content_enter_does_not_open_without_row_context() {
        assert_eq!(
            action_for_key(
                CoreKey::Enter,
                PaneFocus::Content,
                kinic_tabs::KINIC_SETTINGS_TAB_ID
            ),
            None
        );
    }

    #[test]
    fn items_jk_no_longer_move_selection() {
        assert_eq!(
            action_for_key(
                CoreKey::Char('j'),
                PaneFocus::Items,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            None
        );
        assert_eq!(
            action_for_key(
                CoreKey::Char('k'),
                PaneFocus::Items,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            None
        );
    }

    #[test]
    fn content_jk_no_longer_move_or_scroll() {
        assert_eq!(
            action_for_key(
                CoreKey::Char('j'),
                PaneFocus::Content,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            None
        );
        assert_eq!(
            action_for_key(
                CoreKey::Char('k'),
                PaneFocus::Content,
                kinic_tabs::KINIC_SETTINGS_TAB_ID
            ),
            None
        );
    }

    #[test]
    fn memories_search_focus_maps_left_right_to_scope_changes() {
        assert_eq!(
            action_for_key(
                CoreKey::Left,
                PaneFocus::Search,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::SearchScopePrev)
        );
        assert_eq!(
            action_for_key(
                CoreKey::Right,
                PaneFocus::Search,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::SearchScopeNext)
        );
        assert_eq!(
            action_for_key(
                CoreKey::Left,
                PaneFocus::Search,
                kinic_tabs::KINIC_CREATE_TAB_ID
            ),
            None
        );
    }

    #[test]
    fn search_scope_actions_only_mutate_memories_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            search_scope: SearchScope::All,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::SearchScopeNext);
        assert_eq!(state.search_scope, SearchScope::Selected);

        apply_core_action(&mut state, &CoreAction::SearchScopePrev);
        assert_eq!(state.search_scope, SearchScope::All);

        state.current_tab_id = kinic_tabs::KINIC_CREATE_TAB_ID.to_string();
        apply_core_action(&mut state, &CoreAction::SearchScopeNext);
        assert_eq!(state.search_scope, SearchScope::All);
    }

    #[test]
    fn should_open_default_memory_picker_only_on_default_memory_row() {
        let settings = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![SettingsSection {
                title: "Saved preferences".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                        label: "Preferred memory".to_string(),
                        value: "aaaaa-aa".to_string(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "preferences_status".to_string(),
                        label: "Preferences status".to_string(),
                        value: "ok".to_string(),
                        note: None,
                    },
                ],
                footer: None,
            }],
        };
        let state = CoreState {
            current_tab_id: kinic_tabs::KINIC_SETTINGS_TAB_ID.to_string(),
            focus: PaneFocus::Content,
            selected_index: Some(0),
            settings: settings.clone(),
            ..CoreState::default()
        };
        let other_row_state = CoreState {
            selected_index: Some(1),
            settings,
            ..state.clone()
        };

        assert!(should_open_default_memory_picker(&state));
        assert!(!should_open_default_memory_picker(&other_row_state));
    }

    #[test]
    fn open_default_memory_picker_uses_insert_context_from_insert_memory_field() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_INSERT_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            insert_focus: InsertFormFocus::MemoryId,
            default_memory_selector_items: vec![
                MemorySelectorItem {
                    id: "aaaaa-aa".to_string(),
                    title: Some("Alpha Memory".to_string()),
                },
                MemorySelectorItem {
                    id: "bbbbb-bb".to_string(),
                    title: Some("Beta Memory".to_string()),
                },
            ],
            default_memory_selector_selected_id: Some("bbbbb-bb".to_string()),
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::OpenDefaultMemoryPicker);

        assert!(state.default_memory_selector_open);
        assert_eq!(
            state.default_memory_selector_context,
            MemorySelectorContext::InsertTarget
        );
        assert_eq!(state.default_memory_selector_index, 1);
    }

    #[test]
    fn open_default_memory_picker_prefers_explicit_insert_target_selection() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_INSERT_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            insert_focus: InsertFormFocus::MemoryId,
            insert_memory_id: "aaaaa-aa".to_string(),
            default_memory_selector_items: vec![
                MemorySelectorItem {
                    id: "aaaaa-aa".to_string(),
                    title: Some("Alpha Memory".to_string()),
                },
                MemorySelectorItem {
                    id: "bbbbb-bb".to_string(),
                    title: Some("Beta Memory".to_string()),
                },
            ],
            default_memory_selector_selected_id: Some("bbbbb-bb".to_string()),
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::OpenDefaultMemoryPicker);

        assert_eq!(
            state.default_memory_selector_selected_id.as_deref(),
            Some("aaaaa-aa")
        );
        assert_eq!(state.default_memory_selector_index, 0);
    }

    #[test]
    fn apply_snapshot_preserves_settings_row_selection() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_SETTINGS_TAB_ID.to_string(),
            focus: PaneFocus::Content,
            selected_index: Some(1),
            ..CoreState::default()
        };
        let snapshot = ProviderSnapshot {
            settings: SettingsSnapshot {
                quick_entries: vec![],
                sections: vec![SettingsSection {
                    title: "Saved preferences".to_string(),
                    entries: vec![
                        SettingsEntry {
                            id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                            label: "Default memory".to_string(),
                            value: "aaaaa-aa".to_string(),
                            note: None,
                        },
                        SettingsEntry {
                            id: "preferences_status".to_string(),
                            label: "Preferences status".to_string(),
                            value: "ok".to_string(),
                            note: None,
                        },
                    ],
                    footer: None,
                }],
            },
            ..ProviderSnapshot::default()
        };

        apply_snapshot(&mut state, snapshot);

        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn apply_snapshot_updates_selector_context_and_insert_placeholder() {
        let mut state = CoreState::default();
        let snapshot = ProviderSnapshot {
            saved_default_memory_id: Some("aaaaa-aa".to_string()),
            default_memory_selector_context: MemorySelectorContext::InsertTarget,
            insert_memory_placeholder: Some("Alpha Memory".to_string()),
            ..ProviderSnapshot::default()
        };

        apply_snapshot(&mut state, snapshot);

        assert_eq!(
            state.default_memory_selector_context,
            MemorySelectorContext::InsertTarget
        );
        assert_eq!(state.saved_default_memory_id.as_deref(), Some("aaaaa-aa"));
        assert_eq!(
            state.insert_memory_placeholder.as_deref(),
            Some("Alpha Memory")
        );
    }

    #[test]
    fn apply_snapshot_sets_insert_memory_id_from_saved_default_on_insert_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_INSERT_TAB_ID.to_string(),
            ..CoreState::default()
        };
        let snapshot = ProviderSnapshot {
            saved_default_memory_id: Some("aaaaa-aa".to_string()),
            ..ProviderSnapshot::default()
        };

        apply_snapshot(&mut state, snapshot);

        assert_eq!(state.insert_memory_id, "aaaaa-aa");
    }

    #[test]
    fn settings_content_arrow_keys_map_to_settings_actions() {
        assert_eq!(
            action_for_key(
                CoreKey::Down,
                PaneFocus::Content,
                kinic_tabs::KINIC_SETTINGS_TAB_ID
            ),
            Some(CoreAction::MoveNext)
        );
        assert_eq!(
            action_for_key(
                CoreKey::End,
                PaneFocus::Content,
                kinic_tabs::KINIC_SETTINGS_TAB_ID
            ),
            Some(CoreAction::MoveEnd)
        );
        assert_eq!(
            action_for_key(
                CoreKey::PageUp,
                PaneFocus::Content,
                kinic_tabs::KINIC_SETTINGS_TAB_ID
            ),
            Some(CoreAction::MovePageUp)
        );
    }

    #[test]
    fn memories_content_end_maps_to_scroll_end() {
        assert_eq!(
            action_for_key(
                CoreKey::End,
                PaneFocus::Content,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::ScrollContentEnd)
        );
    }

    #[test]
    fn move_actions_follow_settings_selection_length() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_SETTINGS_TAB_ID.to_string(),
            focus: PaneFocus::Content,
            selected_index: Some(0),
            list_items: vec![
                UiItemSummary {
                    id: "memory-1".to_string(),
                    name: "memory-1".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "memory-2".to_string(),
                    name: "memory-2".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
            ],
            settings: SettingsSnapshot {
                quick_entries: vec![],
                sections: vec![SettingsSection {
                    title: "Saved preferences".to_string(),
                    entries: vec![
                        SettingsEntry {
                            id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                            label: "Default memory".to_string(),
                            value: "aaaaa-aa".to_string(),
                            note: None,
                        },
                        SettingsEntry {
                            id: "preferences_status".to_string(),
                            label: "Preferences status".to_string(),
                            value: "ok".to_string(),
                            note: None,
                        },
                        SettingsEntry {
                            id: "preferences_mode".to_string(),
                            label: "Preferences mode".to_string(),
                            value: "live".to_string(),
                            note: None,
                        },
                    ],
                    footer: None,
                }],
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::MoveNext);
        assert_eq!(state.selected_index, Some(1));

        apply_core_action(&mut state, &CoreAction::MoveNext);
        assert_eq!(state.selected_index, Some(2));

        apply_core_action(&mut state, &CoreAction::MoveNext);
        assert_eq!(state.selected_index, Some(2));

        state.selected_index = Some(0);
        apply_core_action(&mut state, &CoreAction::MoveEnd);
        assert_eq!(state.selected_index, Some(2));
    }

    #[test]
    fn insert_input_is_ignored_while_submit_is_running() {
        let mut state = CoreState {
            insert_focus: InsertFormFocus::Text,
            insert_text: "draft".to_string(),
            insert_submit_state: CreateSubmitState::Submitting,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertInput('x'));

        assert_eq!(state.insert_text, "draft");
    }

    #[test]
    fn insert_memory_id_ignores_direct_text_editing() {
        let mut state = CoreState {
            insert_focus: InsertFormFocus::MemoryId,
            insert_memory_id: "aaaaa-aa".to_string(),
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertInput('x'));
        apply_core_action(&mut state, &CoreAction::InsertBackspace);

        assert_eq!(state.insert_memory_id, "aaaaa-aa");
    }

    #[test]
    fn insert_navigation_is_ignored_while_submit_is_running() {
        let mut state = CoreState {
            insert_mode: InsertMode::Normal,
            insert_focus: InsertFormFocus::Text,
            insert_submit_state: CreateSubmitState::Submitting,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertNextField);
        apply_core_action(&mut state, &CoreAction::InsertCycleMode);

        assert_eq!(state.insert_focus, InsertFormFocus::Text);
        assert_eq!(state.insert_mode, InsertMode::Normal);
    }

    #[test]
    fn insert_cycle_mode_prev_moves_backwards_and_resets_focus() {
        let mut state = CoreState {
            insert_mode: InsertMode::Raw,
            insert_focus: InsertFormFocus::Embedding,
            insert_error: Some("boom".to_string()),
            insert_submit_state: CreateSubmitState::Error,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertCycleModePrev);

        assert_eq!(state.insert_mode, InsertMode::Normal);
        assert_eq!(state.insert_focus, InsertFormFocus::Mode);
        assert_eq!(state.insert_error, None);
        assert_eq!(state.insert_submit_state, CreateSubmitState::Idle);
    }

    #[test]
    fn insert_cycle_mode_wraps_between_first_and_last_modes() {
        let mut state = CoreState {
            insert_mode: InsertMode::Normal,
            insert_focus: InsertFormFocus::Mode,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertCycleModePrev);
        assert_eq!(state.insert_mode, InsertMode::Pdf);

        apply_core_action(&mut state, &CoreAction::InsertCycleMode);
        assert_eq!(state.insert_mode, InsertMode::Normal);
        assert_eq!(state.insert_focus, InsertFormFocus::Mode);
    }
}
