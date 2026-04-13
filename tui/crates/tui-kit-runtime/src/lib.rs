//! Shared runtime contracts for the Kinic tui-kit stack.
//!
//! This crate defines common actions/effects, shared runtime state, and the
//! `DataProvider` trait used by the Kinic TUI crates.

pub mod chat_commands;
mod form_descriptor;
mod input_rules;
pub mod kinic_tabs;

use candid::Nat;
pub use form_descriptor::{
    FormCommand, FormDescriptor, FormFocus, FormKind, FormResetKind, apply_form_focus,
    core_action_to_form_command, current_form_focus, current_form_focus_for_tab,
    form_char_input_command, form_command_to_action, form_descriptor, form_enter_command,
    form_horizontal_change_command, form_shows_horizontal_change_hint,
};
pub use kinic_core::amount::{
    KinicAmountParseError, editing_kinic_amount_accepts_char, format_e8s_to_kinic_string_nat,
    format_e8s_to_kinic_string_u128, parse_editing_kinic_display_to_e8s,
    parse_required_kinic_amount_to_e8s,
};
use std::path::PathBuf;
use tui_kit_model::{UiContextNode, UiItemContent, UiItemKind, UiItemSummary};

pub const SETTINGS_ENTRY_DEFAULT_MEMORY_ID: &str = "default_memory";
pub const SETTINGS_ENTRY_KINIC_BALANCE_ID: &str = "kinic_balance";
pub const SETTINGS_ENTRY_SAVED_TAGS_ID: &str = "saved_tags";
pub const SETTINGS_ENTRY_CHAT_RESULT_LIMIT_ID: &str = "chat_result_limit";
pub const SETTINGS_ENTRY_CHAT_PER_MEMORY_LIMIT_ID: &str = "chat_per_memory_limit";
pub const SETTINGS_ENTRY_CHAT_DIVERSITY_ID: &str = "chat_diversity";
pub const FILE_MODE_ALLOWED_EXTENSIONS: &[&str] = &[
    "md", "markdown", "mdx", "txt", "json", "yaml", "yml", "csv", "log", "pdf",
];

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
            allows_chat: false,
        },
        kinic_tabs::TabKind::PlaceholderMarket | kinic_tabs::TabKind::PlaceholderSettings => {
            TabFocusPolicy {
                default_focus: PaneFocus::Tabs,
                allows_search: false,
                allows_items: false,
                allows_tabs: true,
                allows_content: true,
                allows_form: false,
                allows_chat: false,
            }
        }
    }
}

fn chat_supported_for_tab(tab_id: &str) -> bool {
    kinic_tabs::tab_kind(tab_id) == kinic_tabs::TabKind::Memories
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
    File,
    InlineText,
    ManualEmbedding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchScope {
    #[default]
    All,
    Selected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatScope {
    All,
    #[default]
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
pub enum PickerContext {
    #[default]
    DefaultMemory,
    InsertTarget,
    InsertTag,
    TagManagement,
    AddTag,
    ChatResultLimit,
    ChatPerMemoryLimit,
    ChatDiversity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessControlAction {
    #[default]
    Add,
    Remove,
    Change,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessControlRole {
    Admin,
    Writer,
    #[default]
    Reader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessControlFocus {
    #[default]
    Principal,
    Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferModalFocus {
    #[default]
    Principal,
    Amount,
    Max,
    Submit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenameModalFocus {
    #[default]
    Name,
    Submit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferModalMode {
    #[default]
    Edit,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessControlMode {
    #[default]
    None,
    Action,
    Add,
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemorySelectorItem {
    pub id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsRowBehavior {
    pub enter_action: Option<CoreAction>,
    pub status_hint: &'static str,
}

impl SettingsRowBehavior {
    fn new(enter_action: Option<CoreAction>, status_hint: &'static str) -> Self {
        Self {
            enter_action,
            status_hint,
        }
    }
}

impl MemorySelectorItem {
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or(self.id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextInputModalState {
    pub open: bool,
    pub value: String,
    pub submit_state: CreateSubmitState,
    pub error: Option<String>,
}

impl TextInputModalState {
    pub fn clear_error(&mut self) {
        clear_modal_error_state(&mut self.submit_state, &mut self.error);
    }

    pub fn reset_submission(&mut self) {
        reset_modal_submission(&mut self.submit_state, &mut self.error);
    }

    pub fn begin_submit(&mut self) {
        begin_modal_submit(&mut self.submit_state, &mut self.error);
    }

    pub fn is_locked(&self) -> bool {
        modal_locked(self.submit_state.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenameMemoryModalState {
    pub form: TextInputModalState,
    pub memory_id: String,
    pub focus: RenameModalFocus,
}

impl RenameMemoryModalState {
    pub fn open(&mut self) {
        self.form.open = true;
        self.focus = RenameModalFocus::Name;
        self.form.reset_submission();
    }

    pub fn close(&mut self) {
        self.form.open = false;
        self.focus = RenameModalFocus::Name;
        self.form.reset_submission();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoveMemoryModalState {
    pub open: bool,
    pub confirm_yes: bool,
    pub submit_state: CreateSubmitState,
    pub error: Option<String>,
}

impl RemoveMemoryModalState {
    pub fn open(&mut self) {
        self.open = true;
        self.confirm_yes = true;
        reset_modal_submission(&mut self.submit_state, &mut self.error);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.confirm_yes = true;
        reset_modal_submission(&mut self.submit_state, &mut self.error);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccessControlModalState {
    pub open: bool,
    pub mode: AccessControlMode,
    pub memory_id: String,
    pub action: AccessControlAction,
    pub role: AccessControlRole,
    pub current_role: AccessControlRole,
    pub principal_id: String,
    pub confirm_yes: bool,
    pub submit_state: CreateSubmitState,
    pub error: Option<String>,
    pub focus: AccessControlFocus,
}

impl AccessControlModalState {
    pub fn open(&mut self) {
        self.open = true;
        self.confirm_yes = true;
        reset_modal_submission(&mut self.submit_state, &mut self.error);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.mode = AccessControlMode::None;
        self.confirm_yes = true;
        reset_modal_submission(&mut self.submit_state, &mut self.error);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransferModalState {
    pub open: bool,
    pub mode: TransferModalMode,
    pub prerequisites_loading: bool,
    pub principal_id: String,
    pub amount: String,
    pub fee_base_units: Option<u128>,
    pub available_balance_base_units: Option<u128>,
    pub confirm_yes: bool,
    pub submit_state: CreateSubmitState,
    pub error: Option<String>,
    pub focus: TransferModalFocus,
}

impl TransferModalState {
    pub fn open_edit(&mut self) {
        self.open = true;
        self.mode = TransferModalMode::Edit;
        self.prerequisites_loading = false;
        self.confirm_yes = true;
        self.focus = TransferModalFocus::Principal;
        reset_modal_submission(&mut self.submit_state, &mut self.error);
    }

    pub fn open_confirm(&mut self) {
        self.open_edit();
        self.mode = TransferModalMode::Confirm;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.mode = TransferModalMode::Edit;
        self.prerequisites_loading = false;
        self.confirm_yes = true;
        self.focus = TransferModalFocus::Principal;
        reset_modal_submission(&mut self.submit_state, &mut self.error);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerItemKind {
    Option,
    AddAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerItem {
    pub id: String,
    pub label: String,
    pub is_current_default: bool,
    pub kind: PickerItemKind,
}

impl PickerItem {
    pub fn option(
        id: impl Into<String>,
        label: impl Into<String>,
        is_current_default: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            is_current_default,
            kind: PickerItemKind::Option,
        }
    }

    pub fn add_action(label: impl Into<String>) -> Self {
        Self {
            id: String::new(),
            label: label.into(),
            is_current_default: false,
            kind: PickerItemKind::AddAction,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PickerListMode {
    #[default]
    Browsing,
    Confirm {
        kind: PickerConfirmKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerConfirmKind {
    DeleteTag { tag_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PickerState {
    #[default]
    Closed,
    List {
        context: PickerContext,
        items: Vec<PickerItem>,
        selected_index: usize,
        selected_id: Option<String>,
        mode: PickerListMode,
    },
    Input {
        context: PickerContext,
        origin_context: Option<PickerContext>,
        value: String,
    },
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
    pub fee_base_units: Option<u128>,
    pub price_base_units: Option<Nat>,
    pub principal_error: Option<String>,
    pub balance_error: Option<String>,
    pub fee_error: Option<String>,
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
            fee_base_units: None,
            price_base_units: None,
            principal_error: None,
            balance_error: None,
            fee_error: None,
            price_error: None,
        }
    }

    pub fn has_complete_create_cost(&self) -> bool {
        self.balance_base_units.is_some()
            && self.price_base_units.is_some()
            && self.fee_base_units.is_some()
    }

    pub fn account_issue_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        if let Some(error) = &self.principal_error {
            messages.push(format!("Could not derive principal. Cause: {error}"));
        }
        if let Some(error) = &self.balance_error {
            messages.push(format!("Could not fetch KINIC balance. Cause: {error}"));
        }
        if let Some(error) = &self.fee_error {
            messages.push(format!("Could not fetch ledger fee. Cause: {error}"));
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
            || self.fee_error.is_some()
            || self.price_error.is_some()
            || !self.has_complete_create_cost();
        if account_incomplete {
            "Session settings updated (partial account info). See Settings → Account.".to_string()
        } else {
            "Session settings refreshed.".to_string()
        }
    }
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
    pub selected_memory_label: Option<String>,
    pub persistent_status_message: Option<String>,
    pub chat_open: bool,
    pub chat_messages: Vec<(String, String)>,
    pub chat_input: String,
    pub chat_loading: bool,
    pub chat_scroll: usize,
    pub chat_scope: ChatScope,
    pub chat_scope_label: Option<String>,
    pub create_name: String,
    pub create_description: String,
    pub create_submit_state: CreateSubmitState,
    pub create_spinner_frame: usize,
    pub create_error: Option<String>,
    pub create_focus: CreateModalFocus,
    pub create_cost_state: CreateCostState,
    pub settings: SettingsSnapshot,
    pub picker: PickerState,
    pub saved_default_memory_id: Option<String>,
    pub search_scope: SearchScope,
    pub insert_mode: InsertMode,
    pub insert_memory_id: String,
    pub insert_memory_placeholder: Option<String>,
    pub insert_expected_dim: Option<u64>,
    pub insert_expected_dim_loading: bool,
    pub insert_current_dim: Option<String>,
    pub insert_validation_message: Option<String>,
    pub insert_tag: String,
    pub insert_text: String,
    pub insert_file_path_input: String,
    pub insert_selected_file_path: Option<PathBuf>,
    pub insert_embedding: String,
    pub insert_submit_state: CreateSubmitState,
    pub insert_spinner_frame: usize,
    pub insert_error: Option<String>,
    pub insert_focus: InsertFormFocus,
    pub access_list_index: usize,
    pub memory_content_action_index: usize,
    pub access_control: AccessControlModalState,
    pub add_memory: TextInputModalState,
    pub remove_memory: RemoveMemoryModalState,
    pub rename_memory: RenameMemoryModalState,
    pub transfer_modal: TransferModalState,
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
            selected_memory_label: None,
            persistent_status_message: None,
            chat_open: false,
            chat_messages: Vec::new(),
            chat_input: String::new(),
            chat_loading: false,
            chat_scroll: 0,
            chat_scope: ChatScope::default(),
            chat_scope_label: None,
            create_name: String::new(),
            create_description: String::new(),
            create_submit_state: CreateSubmitState::default(),
            create_spinner_frame: 0,
            create_error: None,
            create_focus: CreateModalFocus::default(),
            create_cost_state: CreateCostState::default(),
            settings: SettingsSnapshot::default(),
            picker: PickerState::default(),
            saved_default_memory_id: None,
            search_scope: SearchScope::default(),
            insert_mode: InsertMode::default(),
            insert_memory_id: String::new(),
            insert_memory_placeholder: None,
            insert_expected_dim: None,
            insert_expected_dim_loading: false,
            insert_current_dim: None,
            insert_validation_message: None,
            insert_tag: String::new(),
            insert_text: String::new(),
            insert_file_path_input: String::new(),
            insert_selected_file_path: None,
            insert_embedding: String::new(),
            insert_submit_state: CreateSubmitState::default(),
            insert_spinner_frame: 0,
            insert_error: None,
            insert_focus: InsertFormFocus::default(),
            access_list_index: 0,
            memory_content_action_index: 0,
            access_control: AccessControlModalState::default(),
            add_memory: TextInputModalState::default(),
            remove_memory: RemoveMemoryModalState::default(),
            rename_memory: RenameMemoryModalState::default(),
            transfer_modal: TransferModalState::default(),
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
    OpenPicker(PickerContext),
    ClosePicker,
    MovePickerNext,
    MovePickerPrev,
    DeleteSelectedPickerItem,
    SubmitPicker,
    PickerInput(char),
    PickerBackspace,
    SetDefaultMemoryFromSelection,
    MemoryContentMoveNext,
    MemoryContentMovePrev,
    MemoryContentJumpNext,
    MemoryContentJumpPrev,
    MemoryContentOpenSelected,
    CloseAccessControl,
    AccessNextField,
    AccessPrevField,
    AccessInput(char),
    AccessBackspace,
    AccessCycleAction,
    AccessCycleActionPrev,
    AccessCycleRole,
    AccessCycleRolePrev,
    AccessSubmit,
    OpenAddMemory,
    CloseAddMemory,
    AddMemoryInput(char),
    AddMemoryBackspace,
    AddMemorySubmit,
    OpenRemoveMemory,
    CloseRemoveMemory,
    RemoveMemoryToggleConfirm,
    RemoveMemorySubmit,
    OpenRenameMemory,
    CloseRenameMemory,
    RenameMemoryInput(char),
    RenameMemoryBackspace,
    RenameMemoryNextField,
    RenameMemoryPrevField,
    RenameMemorySubmit,
    OpenTransferModal,
    CloseTransferModal,
    TransferInput(char),
    TransferBackspace,
    TransferNextField,
    TransferPrevField,
    TransferApplyMax,
    TransferSubmit,
    TransferConfirmToggle,
    CreateInput(char),
    CreateBackspace,
    CreateNextField,
    CreatePrevField,
    CreateRefresh,
    RefreshCurrentView,
    CreateSubmit,
    InsertInput(char),
    InsertBackspace,
    InsertOpenFileDialog,
    InsertNextField,
    InsertPrevField,
    InsertPrevMode,
    InsertNextMode,
    InsertSubmit,
    Submit,
    Cancel,
    ChatInput(char),
    ChatBackspace,
    ChatScopePrev,
    ChatScopeNext,
    ChatScopeAll,
    ChatNewThread,
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
    NotifyPersistent(String),
    ReplaceChatMessages(Vec<(String, String)>),
    AppendChatMessage {
        role: String,
        content: String,
    },
    SetChatLoading(bool),
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
    /// Apply a selector-picked insert tag without routing it through text input.
    SetInsertTag(String),
    SetAccessListIndex(usize),
    SetMemoryContentActionIndex(usize),
    OpenAccessAction {
        memory_id: String,
        principal_id: String,
        role: AccessControlRole,
    },
    OpenAccessAdd {
        memory_id: String,
    },
    OpenAccessConfirm {
        memory_id: String,
        principal_id: String,
        action: AccessControlAction,
        role: AccessControlRole,
    },
    /// Close the access control overlay and reset ephemeral errors.
    CloseAccessControl,
    /// Validation or async error for the access control form.
    AccessFormError(Option<String>),
    OpenAddMemory,
    CloseAddMemory,
    AddMemoryFormError(Option<String>),
    OpenRemoveMemory,
    CloseRemoveMemory,
    RemoveMemoryFormError(Option<String>),
    OpenRenameMemory {
        memory_id: String,
        current_name: String,
    },
    CloseRenameMemory,
    RenameFormError(Option<String>),
    OpenTransferModal {
        fee_base_units: u128,
        available_balance_base_units: u128,
    },
    OpenTransferConfirm,
    CloseTransferModal,
    TransferFormError(Option<String>),
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
    pub selected_index: Option<usize>,
    pub selected_content: Option<UiItemContent>,
    pub selected_context: Option<UiContextNode>,
    pub total_count: usize,
    pub status_message: Option<String>,
    pub selected_memory_label: Option<String>,
    pub chat_scope_label: Option<String>,
    pub create_cost_state: CreateCostState,
    pub create_submit_state: CreateSubmitState,
    pub settings: SettingsSnapshot,
    pub picker: PickerState,
    pub saved_default_memory_id: Option<String>,
    pub insert_memory_placeholder: Option<String>,
    pub insert_expected_dim: Option<u64>,
    pub insert_expected_dim_loading: bool,
    pub insert_current_dim: Option<String>,
    pub insert_validation_message: Option<String>,
}

/// Provider response to one action.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderOutput {
    pub snapshot: Option<ProviderSnapshot>,
    pub effects: Vec<CoreEffect>,
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
        action if core_action_to_form_command(action).is_some() => {
            apply_form_command(state, action)
        }
        CoreAction::OpenPicker(context) => {
            open_picker(state, *context);
        }
        CoreAction::ClosePicker => {
            close_picker(state);
        }
        CoreAction::MovePickerNext => {
            move_picker_next(state);
        }
        CoreAction::MovePickerPrev => {
            move_picker_prev(state);
        }
        CoreAction::DeleteSelectedPickerItem => {
            begin_picker_delete_confirm(state);
        }
        CoreAction::SubmitPicker => {
            submit_picker(state);
        }
        CoreAction::PickerInput(c) => {
            if let PickerState::Input { value, .. } = &mut state.picker {
                value.push(*c);
            }
        }
        CoreAction::PickerBackspace => {
            if let PickerState::Input { value, .. } = &mut state.picker {
                value.pop();
            }
        }
        CoreAction::AccessInput(c) => {
            if access_control_locked(state)
                || state.access_control.mode != AccessControlMode::Add
                || state.access_control.focus != AccessControlFocus::Principal
            {
                return;
            }
            if input_rules::principal_text_accepts_char(*c) {
                state.access_control.principal_id.push(*c);
            } else {
                return;
            }
            clear_access_control_error(state);
        }
        CoreAction::AccessBackspace => {
            if access_control_locked(state)
                || state.access_control.mode != AccessControlMode::Add
                || state.access_control.focus != AccessControlFocus::Principal
            {
                return;
            }
            state.access_control.principal_id.pop();
        }
        CoreAction::AccessNextField => {
            if access_control_locked(state) || state.access_control.mode != AccessControlMode::Add {
                return;
            }
            state.access_control.focus = next_access_control_focus(state.access_control.focus);
        }
        CoreAction::AccessPrevField => {
            if access_control_locked(state) || state.access_control.mode != AccessControlMode::Add {
                return;
            }
            state.access_control.focus = prev_access_control_focus(state.access_control.focus);
        }
        CoreAction::AccessCycleAction => {
            if access_control_locked(state) {
                return;
            }
            match state.access_control.mode {
                AccessControlMode::Action => {
                    let (action, role) = next_access_control_selection(
                        state.access_control.current_role,
                        state.access_control.action,
                        state.access_control.role,
                    );
                    state.access_control.action = action;
                    state.access_control.role = role;
                }
                AccessControlMode::Confirm => {
                    state.access_control.confirm_yes = !state.access_control.confirm_yes;
                }
                _ => return,
            }
            clear_access_control_error(state);
        }
        CoreAction::AccessCycleActionPrev => {
            if access_control_locked(state) {
                return;
            }
            match state.access_control.mode {
                AccessControlMode::Action => {
                    let (action, role) = prev_access_control_selection(
                        state.access_control.current_role,
                        state.access_control.action,
                        state.access_control.role,
                    );
                    state.access_control.action = action;
                    state.access_control.role = role;
                }
                AccessControlMode::Confirm => {
                    state.access_control.confirm_yes = !state.access_control.confirm_yes;
                }
                _ => return,
            }
            clear_access_control_error(state);
        }
        CoreAction::AccessCycleRole => {
            if access_control_locked(state)
                || !matches!(state.access_control.mode, AccessControlMode::Add)
                || (state.access_control.mode == AccessControlMode::Add
                    && state.access_control.focus != AccessControlFocus::Role)
            {
                return;
            }
            state.access_control.role = next_access_control_role(state.access_control.role);
            clear_access_control_error(state);
        }
        CoreAction::AccessCycleRolePrev => {
            if access_control_locked(state)
                || !matches!(state.access_control.mode, AccessControlMode::Add)
                || (state.access_control.mode == AccessControlMode::Add
                    && state.access_control.focus != AccessControlFocus::Role)
            {
                return;
            }
            state.access_control.role = prev_access_control_role(state.access_control.role);
            clear_access_control_error(state);
        }
        CoreAction::AccessSubmit => {
            if matches!(
                state.access_control.mode,
                AccessControlMode::Add | AccessControlMode::Confirm
            ) {
                begin_modal_submit(
                    &mut state.access_control.submit_state,
                    &mut state.access_control.error,
                );
            }
        }
        CoreAction::OpenAddMemory => {
            open_add_memory_modal(state);
        }
        CoreAction::CloseAddMemory => {
            close_add_memory_modal(state);
        }
        CoreAction::AddMemoryInput(c) => {
            if !input_rules::principal_text_accepts_char(*c) {
                return;
            }
            apply_text_input_modal_command(
                &mut state.add_memory,
                true,
                TextInputModalCommand::Input(*c),
            );
        }
        CoreAction::AddMemoryBackspace => {
            apply_text_input_modal_command(
                &mut state.add_memory,
                true,
                TextInputModalCommand::Backspace,
            );
        }
        CoreAction::AddMemorySubmit => {
            apply_text_input_modal_command(
                &mut state.add_memory,
                false,
                TextInputModalCommand::Submit,
            );
        }
        CoreAction::OpenRemoveMemory => {
            open_remove_memory_modal(state);
        }
        CoreAction::CloseRemoveMemory => {
            close_remove_memory_modal(state);
        }
        CoreAction::RemoveMemoryToggleConfirm => {
            toggle_confirm_choice(
                remove_memory_modal_locked(state),
                state.remove_memory.open,
                &mut state.remove_memory.confirm_yes,
                &mut state.remove_memory.submit_state,
                &mut state.remove_memory.error,
            );
        }
        CoreAction::RemoveMemorySubmit => {
            if !state.remove_memory.open {
                return;
            }
            if state.remove_memory.confirm_yes {
                begin_modal_submit(
                    &mut state.remove_memory.submit_state,
                    &mut state.remove_memory.error,
                );
            } else {
                close_remove_memory_modal(state);
            }
        }
        CoreAction::OpenRenameMemory => {}
        CoreAction::CloseRenameMemory => {
            close_rename_memory_modal(state);
        }
        CoreAction::RenameMemoryInput(c) => {
            apply_text_input_modal_command(
                &mut state.rename_memory.form,
                state.rename_memory.focus == RenameModalFocus::Name,
                TextInputModalCommand::Input(*c),
            );
        }
        CoreAction::RenameMemoryBackspace => {
            apply_text_input_modal_command(
                &mut state.rename_memory.form,
                state.rename_memory.focus == RenameModalFocus::Name,
                TextInputModalCommand::Backspace,
            );
        }
        CoreAction::RenameMemoryNextField => {
            if rename_modal_locked(state) || !state.rename_memory.form.open {
                return;
            }
            state.rename_memory.focus = next_rename_focus(state.rename_memory.focus);
            clear_rename_error(state);
        }
        CoreAction::RenameMemoryPrevField => {
            if rename_modal_locked(state) || !state.rename_memory.form.open {
                return;
            }
            state.rename_memory.focus = prev_rename_focus(state.rename_memory.focus);
            clear_rename_error(state);
        }
        CoreAction::RenameMemorySubmit => {
            apply_text_input_modal_command(
                &mut state.rename_memory.form,
                false,
                TextInputModalCommand::Submit,
            );
        }
        CoreAction::OpenTransferModal => {
            open_transfer_modal(state);
            state.transfer_modal.prerequisites_loading = true;
            state.transfer_modal.principal_id.clear();
            state.transfer_modal.amount.clear();
            state.transfer_modal.fee_base_units = None;
            state.transfer_modal.available_balance_base_units = None;
        }
        CoreAction::CloseTransferModal => {
            close_transfer_modal(state);
        }
        CoreAction::TransferInput(c) => {
            if transfer_modal_locked(state) || state.transfer_modal.mode != TransferModalMode::Edit
            {
                return;
            }
            match state.transfer_modal.focus {
                TransferModalFocus::Principal => {
                    if input_rules::principal_text_accepts_char(*c) {
                        state.transfer_modal.principal_id.push(*c);
                    } else {
                        return;
                    }
                }
                TransferModalFocus::Amount => {
                    if editing_kinic_amount_accepts_char(&state.transfer_modal.amount, *c) {
                        state.transfer_modal.amount.push(*c);
                    } else {
                        return;
                    }
                }
                TransferModalFocus::Max | TransferModalFocus::Submit => {}
            }
            clear_transfer_error(state);
        }
        CoreAction::TransferBackspace => {
            if transfer_modal_locked(state) || state.transfer_modal.mode != TransferModalMode::Edit
            {
                return;
            }
            match state.transfer_modal.focus {
                TransferModalFocus::Principal => {
                    state.transfer_modal.principal_id.pop();
                }
                TransferModalFocus::Amount => {
                    state.transfer_modal.amount.pop();
                }
                TransferModalFocus::Max | TransferModalFocus::Submit => {}
            }
            clear_transfer_error(state);
        }
        CoreAction::TransferNextField => {
            if transfer_modal_locked(state) {
                return;
            }
            match state.transfer_modal.mode {
                TransferModalMode::Edit => {
                    state.transfer_modal.focus =
                        next_transfer_focus(state.transfer_modal.focus, state);
                }
                TransferModalMode::Confirm => {
                    state.transfer_modal.confirm_yes = !state.transfer_modal.confirm_yes;
                }
            }
            clear_transfer_error(state);
        }
        CoreAction::TransferPrevField => {
            if transfer_modal_locked(state) {
                return;
            }
            match state.transfer_modal.mode {
                TransferModalMode::Edit => {
                    state.transfer_modal.focus =
                        prev_transfer_focus(state.transfer_modal.focus, state);
                }
                TransferModalMode::Confirm => {
                    state.transfer_modal.confirm_yes = !state.transfer_modal.confirm_yes;
                }
            }
            clear_transfer_error(state);
        }
        CoreAction::TransferApplyMax => {
            if transfer_modal_locked(state) || state.transfer_modal.mode != TransferModalMode::Edit
            {
                return;
            }
            let available = state
                .transfer_modal
                .available_balance_base_units
                .unwrap_or(0);
            let fee = state.transfer_modal.fee_base_units.unwrap_or(0);
            let max_amount = available.saturating_sub(fee);
            state.transfer_modal.amount = format_e8s_to_kinic_string_u128(max_amount);
            state.transfer_modal.focus = TransferModalFocus::Submit;
            clear_transfer_error(state);
        }
        CoreAction::TransferConfirmToggle => {
            toggle_confirm_choice(
                transfer_modal_locked(state),
                state.transfer_modal.mode == TransferModalMode::Confirm,
                &mut state.transfer_modal.confirm_yes,
                &mut state.transfer_modal.submit_state,
                &mut state.transfer_modal.error,
            );
        }
        CoreAction::TransferSubmit => match state.transfer_modal.mode {
            TransferModalMode::Edit => {}
            TransferModalMode::Confirm if state.transfer_modal.confirm_yes => {
                begin_modal_submit(
                    &mut state.transfer_modal.submit_state,
                    &mut state.transfer_modal.error,
                );
            }
            TransferModalMode::Confirm => {}
        },
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
            if !chat_supported_for_tab(state.current_tab_id.as_str()) {
                state.chat_open = false;
                if state.focus == PaneFocus::Extra {
                    state.focus = focus_after_chat_close(state.current_tab_id.as_str());
                }
            }
            state.selected_index = Some(0);
            close_picker(state);
            close_access_control_modal(state);
            state.access_list_index = 0;
            state.memory_content_action_index = 0;
            close_add_memory_modal(state);
            close_remove_memory_modal(state);
            close_rename_memory_modal(state);
            close_transfer_modal(state);
        }
        CoreAction::SelectTabIndex(index) => {
            state.current_tab_id = format!("tab-{}", index + 1);
            state.selected_index = Some(0);
        }
        CoreAction::FocusNext => {
            state.focus = match state.focus {
                PaneFocus::Search => PaneFocus::Items,
                PaneFocus::Items => {
                    if memories_chat_replaces_content(state) {
                        PaneFocus::Extra
                    } else {
                        PaneFocus::Content
                    }
                }
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
                PaneFocus::Extra => {
                    if memories_chat_replaces_content(state) {
                        PaneFocus::Items
                    } else {
                        PaneFocus::Content
                    }
                }
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
        CoreAction::FocusContent => {
            state.focus = PaneFocus::Content;
            if state.current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID {
                state.memory_content_action_index = 0;
            }
        }
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
        CoreAction::OpenSelected => {
            if memories_chat_replaces_content(state) {
                state.focus = PaneFocus::Extra;
            } else {
                state.focus = PaneFocus::Content;
            }
            if state.current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID
                && state.focus == PaneFocus::Content
            {
                state.memory_content_action_index = 0;
            }
        }
        CoreAction::Back => {
            state.focus = if state.focus == PaneFocus::Extra {
                PaneFocus::Content
            } else {
                PaneFocus::Items
            };
        }
        CoreAction::ToggleChat => {
            if !chat_supported_for_tab(state.current_tab_id.as_str()) {
                state.chat_open = false;
                if state.focus == PaneFocus::Extra {
                    state.focus = focus_after_chat_close(state.current_tab_id.as_str());
                }
                return;
            }
            state.chat_open = !state.chat_open;
            if state.chat_open {
                state.focus = PaneFocus::Extra;
            } else if state.focus == PaneFocus::Extra {
                state.focus = focus_after_chat_close(state.current_tab_id.as_str());
            }
        }
        CoreAction::MemoryContentMoveNext => {}
        CoreAction::MemoryContentMovePrev => {}
        CoreAction::MemoryContentJumpNext => {}
        CoreAction::MemoryContentJumpPrev => {}
        CoreAction::MemoryContentOpenSelected => {}
        CoreAction::CloseAccessControl => {
            close_access_control_modal(state);
        }
        CoreAction::ChatInput(c) => {
            state.chat_input.push(*c);
        }
        CoreAction::ChatBackspace => {
            state.chat_input.pop();
        }
        CoreAction::ChatScopePrev => {
            if state.current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID {
                prev_chat_scope(state);
            }
        }
        CoreAction::ChatScopeNext => {
            if state.current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID {
                next_chat_scope(state);
            }
        }
        CoreAction::ChatScopeAll => {
            if state.current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID {
                state.chat_scope = ChatScope::All;
            }
        }
        CoreAction::ChatNewThread => {}
        CoreAction::ChatSubmit => {
            if state.chat_loading {
                return;
            }
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
                state.selected_index = Some(if idx + 1 >= len { 0 } else { idx + 1 });
            }
        }
        CoreAction::MovePrev => {
            let len = selectable_len(state);
            if len == 0 {
                state.selected_index = None;
            } else {
                let idx = state.selected_index.unwrap_or(0);
                state.selected_index = Some(if idx == 0 { len - 1 } else { idx - 1 });
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

fn memories_chat_replaces_content(state: &CoreState) -> bool {
    state.current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID && state.chat_open
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
        InsertMode::File => &[
            InsertFormFocus::Mode,
            InsertFormFocus::MemoryId,
            InsertFormFocus::Tag,
            InsertFormFocus::FilePath,
            InsertFormFocus::Submit,
        ],
        InsertMode::InlineText => &[
            InsertFormFocus::Mode,
            InsertFormFocus::MemoryId,
            InsertFormFocus::Tag,
            InsertFormFocus::Text,
            InsertFormFocus::Submit,
        ],
        InsertMode::ManualEmbedding => &[
            InsertFormFocus::Mode,
            InsertFormFocus::MemoryId,
            InsertFormFocus::Tag,
            InsertFormFocus::Text,
            InsertFormFocus::Embedding,
            InsertFormFocus::Submit,
        ],
    }
}

fn apply_form_command(state: &mut CoreState, action: &CoreAction) {
    let Some((kind, command)) = core_action_to_form_command(action) else {
        return;
    };

    if kind == FormKind::Insert
        && is_insert_form_locked(state)
        && !matches!(
            command,
            FormCommand::OpenPicker(_) | FormCommand::OpenFileDialog
        )
    {
        return;
    }

    match (kind, command) {
        (FormKind::Insert, FormCommand::Input(c)) => apply_insert_text_input(state, c),
        (FormKind::Insert, FormCommand::Backspace) => apply_insert_backspace(state),
        (FormKind::Insert, FormCommand::NextField) => {
            state.insert_focus = next_insert_focus(state.insert_mode, state.insert_focus);
        }
        (FormKind::Insert, FormCommand::PrevField) => {
            state.insert_focus = prev_insert_focus(state.insert_mode, state.insert_focus);
        }
        (FormKind::Insert, FormCommand::Submit) => start_insert_submit(state),
        (FormKind::Insert, FormCommand::HorizontalChangePrev) => {
            state.insert_mode = prev_insert_mode(state.insert_mode);
            state.insert_focus = InsertFormFocus::Mode;
            clear_insert_error_state(state);
        }
        (FormKind::Insert, FormCommand::HorizontalChangeNext) => {
            state.insert_mode = next_insert_mode(state.insert_mode);
            state.insert_focus = InsertFormFocus::Mode;
            clear_insert_error_state(state);
        }
        (FormKind::Insert, FormCommand::OpenPicker(context)) => open_picker(state, context),
        (FormKind::Insert, FormCommand::OpenFileDialog) => {}
        (FormKind::Create, FormCommand::Input(c)) => apply_create_text_input(state, c),
        (FormKind::Create, FormCommand::Backspace) => apply_create_backspace(state),
        (FormKind::Create, FormCommand::NextField) => {
            state.create_focus = next_create_focus(state.create_focus);
        }
        (FormKind::Create, FormCommand::PrevField) => {
            state.create_focus = prev_create_focus(state.create_focus);
        }
        (FormKind::Create, FormCommand::Submit) => start_create_submit(state),
        (FormKind::Create, FormCommand::HorizontalChangePrev)
        | (FormKind::Create, FormCommand::HorizontalChangeNext)
        | (FormKind::Create, FormCommand::OpenPicker(_))
        | (FormKind::Create, FormCommand::OpenFileDialog) => {}
    }
}

fn apply_insert_text_input(state: &mut CoreState, c: char) {
    match state.insert_focus {
        InsertFormFocus::Mode
        | InsertFormFocus::MemoryId
        | InsertFormFocus::Tag
        | InsertFormFocus::Submit => {}
        InsertFormFocus::Text => state.insert_text.push(c),
        InsertFormFocus::FilePath => {
            state.insert_selected_file_path = None;
            state.insert_file_path_input.push(c);
        }
        InsertFormFocus::Embedding => state.insert_embedding.push(c),
    }
    clear_insert_error_state(state);
}

fn apply_insert_backspace(state: &mut CoreState) {
    match state.insert_focus {
        InsertFormFocus::Mode
        | InsertFormFocus::MemoryId
        | InsertFormFocus::Tag
        | InsertFormFocus::Submit => {}
        InsertFormFocus::Text => {
            state.insert_text.pop();
        }
        InsertFormFocus::FilePath => {
            state.insert_selected_file_path = None;
            state.insert_file_path_input.pop();
        }
        InsertFormFocus::Embedding => {
            state.insert_embedding.pop();
        }
    }
}

fn clear_insert_error_state(state: &mut CoreState) {
    state.insert_error = None;
    if state.insert_submit_state == CreateSubmitState::Error {
        state.insert_submit_state = CreateSubmitState::Idle;
    }
}

fn start_insert_submit(state: &mut CoreState) {
    state.insert_submit_state = CreateSubmitState::Submitting;
    state.insert_spinner_frame = 0;
    state.insert_error = None;
}

fn apply_create_text_input(state: &mut CoreState, c: char) {
    match state.create_focus {
        CreateModalFocus::Name => state.create_name.push(c),
        CreateModalFocus::Description => state.create_description.push(c),
        CreateModalFocus::Submit => {}
    }
    clear_create_error_state(state);
}

fn apply_create_backspace(state: &mut CoreState) {
    match state.create_focus {
        CreateModalFocus::Name => {
            state.create_name.pop();
        }
        CreateModalFocus::Description => {
            state.create_description.pop();
        }
        CreateModalFocus::Submit => {}
    }
}

fn clear_create_error_state(state: &mut CoreState) {
    state.create_error = None;
    if state.create_submit_state == CreateSubmitState::Error {
        state.create_submit_state = CreateSubmitState::Idle;
    }
}

fn start_create_submit(state: &mut CoreState) {
    state.create_submit_state = CreateSubmitState::Submitting;
    state.create_spinner_frame = 0;
    state.create_error = None;
}

fn next_create_focus(focus: CreateModalFocus) -> CreateModalFocus {
    match focus {
        CreateModalFocus::Name => CreateModalFocus::Description,
        CreateModalFocus::Description => CreateModalFocus::Submit,
        CreateModalFocus::Submit => CreateModalFocus::Name,
    }
}

fn prev_create_focus(focus: CreateModalFocus) -> CreateModalFocus {
    match focus {
        CreateModalFocus::Name => CreateModalFocus::Submit,
        CreateModalFocus::Description => CreateModalFocus::Name,
        CreateModalFocus::Submit => CreateModalFocus::Description,
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
        InsertMode::File => InsertMode::InlineText,
        InsertMode::InlineText => InsertMode::ManualEmbedding,
        InsertMode::ManualEmbedding => InsertMode::File,
    }
}

fn prev_insert_mode(mode: InsertMode) -> InsertMode {
    match mode {
        InsertMode::File => InsertMode::ManualEmbedding,
        InsertMode::InlineText => InsertMode::File,
        InsertMode::ManualEmbedding => InsertMode::InlineText,
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

fn next_chat_scope(state: &mut CoreState) {
    let visible_ids = visible_chat_scope_memory_ids(state);
    if visible_ids.is_empty() {
        if state.chat_scope == ChatScope::Selected {
            state.chat_scope = ChatScope::All;
        }
        return;
    }

    match state.chat_scope {
        ChatScope::All => {
            state.chat_scope = ChatScope::Selected;
            state.selected_index = first_memory_list_index(state);
        }
        ChatScope::Selected => {
            let current_id = resolve_selected_chat_scope_memory_id(state)
                .or_else(|| visible_ids.first().cloned());
            let Some(current_id) = current_id else {
                state.chat_scope = ChatScope::All;
                return;
            };
            let current_index = visible_ids
                .iter()
                .position(|id| id == &current_id)
                .unwrap_or(0);
            if current_index + 1 >= visible_ids.len() {
                state.chat_scope = ChatScope::All;
            } else {
                let next_id = visible_ids[current_index + 1].clone();
                state.selected_index = list_index_for_memory_id(state, &next_id);
            }
        }
    }
}

fn prev_chat_scope(state: &mut CoreState) {
    let visible_ids = visible_chat_scope_memory_ids(state);
    if visible_ids.is_empty() {
        if state.chat_scope == ChatScope::Selected {
            state.chat_scope = ChatScope::All;
        }
        return;
    }

    match state.chat_scope {
        ChatScope::All => {
            state.chat_scope = ChatScope::Selected;
            let last_id = visible_ids.last().expect("non-empty checked").clone();
            state.selected_index = list_index_for_memory_id(state, &last_id);
        }
        ChatScope::Selected => {
            let current_id = resolve_selected_chat_scope_memory_id(state)
                .or_else(|| visible_ids.first().cloned());
            let Some(current_id) = current_id else {
                state.chat_scope = ChatScope::All;
                return;
            };
            let current_index = visible_ids
                .iter()
                .position(|id| id == &current_id)
                .unwrap_or(0);
            if current_index == 0 {
                state.chat_scope = ChatScope::All;
            } else {
                let prev_id = visible_ids[current_index - 1].clone();
                state.selected_index = list_index_for_memory_id(state, &prev_id);
            }
        }
    }
}

fn visible_chat_scope_memory_ids(state: &CoreState) -> Vec<String> {
    state
        .list_items
        .iter()
        .filter(|item| matches!(&item.kind, UiItemKind::Custom(kind) if kind == "memory"))
        .map(|item| item.id.clone())
        .collect()
}

fn resolve_selected_chat_scope_memory_id(state: &CoreState) -> Option<String> {
    state.selected_index.and_then(|index| {
        state.list_items.get(index).and_then(|item| {
            matches!(&item.kind, UiItemKind::Custom(kind) if kind == "memory")
                .then(|| item.id.clone())
        })
    })
}

fn first_memory_list_index(state: &CoreState) -> Option<usize> {
    state
        .list_items
        .iter()
        .position(|item| matches!(&item.kind, UiItemKind::Custom(kind) if kind == "memory"))
}

fn list_index_for_memory_id(state: &CoreState, memory_id: &str) -> Option<usize> {
    state.list_items.iter().position(|item| {
        item.id == memory_id && matches!(&item.kind, UiItemKind::Custom(kind) if kind == "memory")
    })
}

pub fn clear_modal_error_state(submit_state: &mut CreateSubmitState, error: &mut Option<String>) {
    *error = None;
    if *submit_state == CreateSubmitState::Error {
        *submit_state = CreateSubmitState::Idle;
    }
}

pub fn apply_modal_error(
    submit_state: &mut CreateSubmitState,
    error: &mut Option<String>,
    message: Option<String>,
) {
    *submit_state = if message.is_some() {
        CreateSubmitState::Error
    } else {
        CreateSubmitState::Idle
    };
    *error = message;
}

pub fn reset_modal_submission(submit_state: &mut CreateSubmitState, error: &mut Option<String>) {
    *submit_state = CreateSubmitState::Idle;
    *error = None;
}

pub fn begin_modal_submit(submit_state: &mut CreateSubmitState, error: &mut Option<String>) {
    *submit_state = CreateSubmitState::Submitting;
    *error = None;
}

pub fn modal_locked(submit_state: CreateSubmitState) -> bool {
    submit_state == CreateSubmitState::Submitting
}

fn clear_access_control_error(state: &mut CoreState) {
    clear_modal_error_state(
        &mut state.access_control.submit_state,
        &mut state.access_control.error,
    );
}

fn clear_transfer_error(state: &mut CoreState) {
    clear_modal_error_state(
        &mut state.transfer_modal.submit_state,
        &mut state.transfer_modal.error,
    );
}

fn access_control_locked(state: &CoreState) -> bool {
    modal_locked(state.access_control.submit_state.clone())
}

fn transfer_modal_locked(state: &CoreState) -> bool {
    modal_locked(state.transfer_modal.submit_state.clone())
}

fn access_control_focus_order() -> &'static [AccessControlFocus] {
    &[AccessControlFocus::Principal, AccessControlFocus::Role]
}

fn next_access_control_focus(focus: AccessControlFocus) -> AccessControlFocus {
    next_in_cycle(focus, access_control_focus_order())
}

fn prev_access_control_focus(focus: AccessControlFocus) -> AccessControlFocus {
    prev_in_cycle(focus, access_control_focus_order())
}

fn next_access_control_selection(
    current_role: AccessControlRole,
    action: AccessControlAction,
    role: AccessControlRole,
) -> (AccessControlAction, AccessControlRole) {
    let options = access_control_options(current_role);
    let current = options
        .iter()
        .position(|candidate| *candidate == (action, role))
        .unwrap_or(0);
    options[(current + 1) % options.len()]
}

fn prev_access_control_selection(
    current_role: AccessControlRole,
    action: AccessControlAction,
    role: AccessControlRole,
) -> (AccessControlAction, AccessControlRole) {
    let options = access_control_options(current_role);
    let current = options
        .iter()
        .position(|candidate| *candidate == (action, role))
        .unwrap_or(0);
    options[(current + options.len() - 1) % options.len()]
}

fn access_control_options(
    current_role: AccessControlRole,
) -> &'static [(AccessControlAction, AccessControlRole)] {
    match current_role {
        AccessControlRole::Admin => &[
            (AccessControlAction::Change, AccessControlRole::Writer),
            (AccessControlAction::Change, AccessControlRole::Reader),
            (AccessControlAction::Remove, AccessControlRole::Admin),
        ],
        AccessControlRole::Writer => &[
            (AccessControlAction::Change, AccessControlRole::Admin),
            (AccessControlAction::Change, AccessControlRole::Reader),
            (AccessControlAction::Remove, AccessControlRole::Writer),
        ],
        AccessControlRole::Reader => &[
            (AccessControlAction::Change, AccessControlRole::Admin),
            (AccessControlAction::Change, AccessControlRole::Writer),
            (AccessControlAction::Remove, AccessControlRole::Reader),
        ],
    }
}

fn next_access_control_role(role: AccessControlRole) -> AccessControlRole {
    match role {
        AccessControlRole::Admin => AccessControlRole::Writer,
        AccessControlRole::Writer => AccessControlRole::Reader,
        AccessControlRole::Reader => AccessControlRole::Admin,
    }
}

fn prev_access_control_role(role: AccessControlRole) -> AccessControlRole {
    match role {
        AccessControlRole::Admin => AccessControlRole::Reader,
        AccessControlRole::Writer => AccessControlRole::Admin,
        AccessControlRole::Reader => AccessControlRole::Writer,
    }
}

fn clear_rename_error(state: &mut CoreState) {
    clear_modal_error_state(
        &mut state.rename_memory.form.submit_state,
        &mut state.rename_memory.form.error,
    );
}

fn remove_memory_modal_locked(state: &CoreState) -> bool {
    modal_locked(state.remove_memory.submit_state.clone())
}

fn rename_modal_locked(state: &CoreState) -> bool {
    modal_locked(state.rename_memory.form.submit_state.clone())
}

fn next_rename_focus(focus: RenameModalFocus) -> RenameModalFocus {
    next_in_cycle(focus, &[RenameModalFocus::Name, RenameModalFocus::Submit])
}

fn prev_rename_focus(focus: RenameModalFocus) -> RenameModalFocus {
    prev_in_cycle(focus, &[RenameModalFocus::Name, RenameModalFocus::Submit])
}

fn transfer_focus_order(state: &CoreState) -> &'static [TransferModalFocus] {
    if state.transfer_modal.prerequisites_loading {
        &[TransferModalFocus::Principal, TransferModalFocus::Amount]
    } else {
        &[
            TransferModalFocus::Principal,
            TransferModalFocus::Amount,
            TransferModalFocus::Max,
            TransferModalFocus::Submit,
        ]
    }
}

fn next_transfer_focus(focus: TransferModalFocus, state: &CoreState) -> TransferModalFocus {
    next_in_cycle(focus, transfer_focus_order(state))
}

fn prev_transfer_focus(focus: TransferModalFocus, state: &CoreState) -> TransferModalFocus {
    prev_in_cycle(focus, transfer_focus_order(state))
}

fn toggle_confirm_choice(
    is_locked: bool,
    is_confirm_mode: bool,
    confirm_yes: &mut bool,
    submit_state: &mut CreateSubmitState,
    error: &mut Option<String>,
) {
    if is_locked || !is_confirm_mode {
        return;
    }
    *confirm_yes = !*confirm_yes;
    clear_modal_error_state(submit_state, error);
}

fn next_in_cycle<T: Copy + PartialEq>(current: T, order: &[T]) -> T {
    let index = order
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0);
    order[(index + 1) % order.len()]
}

fn prev_in_cycle<T: Copy + PartialEq>(current: T, order: &[T]) -> T {
    let index = order
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0);
    order[(index + order.len() - 1) % order.len()]
}

enum TextInputModalCommand {
    Input(char),
    Backspace,
    Submit,
}

fn apply_text_input_modal_command(
    modal: &mut TextInputModalState,
    is_editable: bool,
    command: TextInputModalCommand,
) {
    if !modal.open {
        return;
    }
    match command {
        TextInputModalCommand::Submit => {
            begin_modal_submit(&mut modal.submit_state, &mut modal.error);
        }
        TextInputModalCommand::Input(c)
            if is_editable && !modal_locked(modal.submit_state.clone()) =>
        {
            modal.value.push(c);
            clear_modal_error_state(&mut modal.submit_state, &mut modal.error);
        }
        TextInputModalCommand::Backspace
            if is_editable && !modal_locked(modal.submit_state.clone()) =>
        {
            modal.value.pop();
            clear_modal_error_state(&mut modal.submit_state, &mut modal.error);
        }
        TextInputModalCommand::Input(_) | TextInputModalCommand::Backspace => {}
    }
}

pub fn open_add_memory_modal(state: &mut CoreState) {
    state.add_memory.open = true;
    state.add_memory.value.clear();
    state.add_memory.reset_submission();
}

pub fn close_add_memory_modal(state: &mut CoreState) {
    state.add_memory.open = false;
    state.add_memory.reset_submission();
}

pub fn open_remove_memory_modal(state: &mut CoreState) {
    state.remove_memory.open();
}

pub fn close_remove_memory_modal(state: &mut CoreState) {
    state.remove_memory.close();
}

pub fn open_rename_memory_modal(state: &mut CoreState) {
    state.rename_memory.open();
}

pub fn close_rename_memory_modal(state: &mut CoreState) {
    state.rename_memory.close();
}

pub fn open_transfer_modal(state: &mut CoreState) {
    state.transfer_modal.open_edit();
}

pub fn open_transfer_confirm(state: &mut CoreState) {
    state.transfer_modal.open_confirm();
}

pub fn close_transfer_modal(state: &mut CoreState) {
    state.transfer_modal.close();
}

pub fn open_access_control_modal(state: &mut CoreState) {
    state.access_control.open();
}

pub fn close_access_control_modal(state: &mut CoreState) {
    state.access_control.close();
}

pub fn is_insert_form_locked(state: &CoreState) -> bool {
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
    if chat_open && policy.allows_chat {
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
    if focus == PaneFocus::Content && current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID {
        match key {
            CoreKey::Tab => return Some(CoreAction::MemoryContentJumpNext),
            CoreKey::BackTab => return Some(CoreAction::MemoryContentJumpPrev),
            _ => {}
        }
    }

    if focus == PaneFocus::Tabs {
        return match key {
            CoreKey::Up | CoreKey::Left => Some(CoreAction::SelectPrevTab),
            CoreKey::Down | CoreKey::Right => Some(CoreAction::SelectNextTab),
            CoreKey::Char('h') => None,
            CoreKey::Tab | CoreKey::Char('l') | CoreKey::Enter => tab_entry_focus(current_tab_id)
                .map(|focus| match focus {
                    PaneFocus::Search => CoreAction::FocusSearch,
                    PaneFocus::Items => CoreAction::FocusItems,
                    PaneFocus::Tabs => CoreAction::FocusNext,
                    PaneFocus::Content => CoreAction::FocusContent,
                    PaneFocus::Form => CoreAction::FocusForm,
                    PaneFocus::Extra => CoreAction::ToggleChat,
                }),
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
                CoreKey::Enter if current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID => {
                    Some(CoreAction::MemoryContentOpenSelected)
                }
                CoreKey::Down if current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID => {
                    Some(CoreAction::MemoryContentMoveNext)
                }
                CoreKey::Up if current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID => {
                    Some(CoreAction::MemoryContentMovePrev)
                }
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
                CoreKey::Left if current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID => {
                    Some(CoreAction::ChatScopePrev)
                }
                CoreKey::Right if current_tab_id == kinic_tabs::KINIC_MEMORIES_TAB_ID => {
                    Some(CoreAction::ChatScopeNext)
                }
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
    let snapshot_selected_index = snapshot.selected_index;
    state.selected_content = snapshot.selected_content;
    state.selected_context = snapshot.selected_context;
    state.total_count = snapshot.total_count;
    if state.persistent_status_message.is_none() {
        state.status_message = snapshot.status_message;
    }
    state.selected_memory_label = snapshot.selected_memory_label;
    state.chat_scope_label = snapshot.chat_scope_label;
    state.create_cost_state = snapshot.create_cost_state;
    state.create_submit_state = snapshot.create_submit_state;
    state.settings = snapshot.settings;
    state.picker = reconcile_picker_state(state, snapshot.picker);
    state.saved_default_memory_id = snapshot.saved_default_memory_id;
    state.insert_memory_placeholder = snapshot.insert_memory_placeholder;
    state.insert_expected_dim = snapshot.insert_expected_dim;
    state.insert_expected_dim_loading = snapshot.insert_expected_dim_loading;
    state.insert_current_dim = snapshot.insert_current_dim;
    state.insert_validation_message = snapshot.insert_validation_message;
    let selectable_len = selectable_len(state);
    if let Some(selected_index) = snapshot_selected_index {
        state.selected_index = if selectable_len == 0 {
            None
        } else {
            Some(selected_index.min(selectable_len.saturating_sub(1)))
        };
        return;
    }

    if !is_settings_content(state.current_tab_id.as_str(), state.focus)
        && let Some(selected_content_id) = state
            .selected_content
            .as_ref()
            .map(|content| content.id.as_str())
        && let Some(selected_index) = state
            .list_items
            .iter()
            .position(|item| item.id == selected_content_id)
    {
        state.selected_index = Some(selected_index);
    }

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

pub fn selected_settings_row_behavior(state: &CoreState) -> Option<SettingsRowBehavior> {
    if !is_settings_content(state.current_tab_id.as_str(), state.focus) {
        return None;
    }

    state
        .selected_index
        .and_then(|index| settings_row_behavior_for_index(&state.settings, index))
}

pub fn settings_row_behavior_for_index(
    settings: &SettingsSnapshot,
    index: usize,
) -> Option<SettingsRowBehavior> {
    let entry = settings_entry(settings, index)?;
    Some(match entry.id.as_str() {
        SETTINGS_ENTRY_KINIC_BALANCE_ID => {
            SettingsRowBehavior::new(Some(CoreAction::OpenTransferModal), " send KINIC ")
        }
        SETTINGS_ENTRY_DEFAULT_MEMORY_ID => SettingsRowBehavior::new(
            Some(CoreAction::OpenPicker(PickerContext::DefaultMemory)),
            " open Default memory ",
        ),
        SETTINGS_ENTRY_SAVED_TAGS_ID => SettingsRowBehavior::new(
            Some(CoreAction::OpenPicker(PickerContext::TagManagement)),
            " manage saved tags ",
        ),
        SETTINGS_ENTRY_CHAT_RESULT_LIMIT_ID => SettingsRowBehavior::new(
            Some(CoreAction::OpenPicker(PickerContext::ChatResultLimit)),
            " adjust chat limit ",
        ),
        SETTINGS_ENTRY_CHAT_PER_MEMORY_LIMIT_ID => SettingsRowBehavior::new(
            Some(CoreAction::OpenPicker(PickerContext::ChatPerMemoryLimit)),
            " adjust per-memory limit ",
        ),
        SETTINGS_ENTRY_CHAT_DIVERSITY_ID => SettingsRowBehavior::new(
            Some(CoreAction::OpenPicker(PickerContext::ChatDiversity)),
            " adjust chat diversity ",
        ),
        _ => SettingsRowBehavior::new(None, " row details "),
    })
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

fn reconcile_picker_state(state: &CoreState, snapshot: PickerState) -> PickerState {
    match snapshot {
        PickerState::Closed => PickerState::Closed,
        PickerState::Input {
            context,
            origin_context,
            value,
        } => PickerState::Input {
            context,
            origin_context,
            value,
        },
        PickerState::List {
            context,
            items,
            selected_index,
            selected_id,
            mode,
        } => {
            let previous_index = match &state.picker {
                PickerState::List {
                    context: previous_context,
                    selected_index,
                    ..
                } if *previous_context == context => *selected_index,
                _ => selected_index,
            };
            let previous_selected_id = match &state.picker {
                PickerState::List {
                    context: previous_context,
                    selected_id,
                    ..
                } if *previous_context == context => selected_id.clone(),
                _ => selected_id.clone(),
            };
            let previous_mode = match &state.picker {
                PickerState::List {
                    context: previous_context,
                    mode,
                    ..
                } if *previous_context == context => mode.clone(),
                _ => mode.clone(),
            };
            if previous_selected_id.is_none()
                && previous_index < items.len()
                && items[previous_index].kind == PickerItemKind::AddAction
            {
                return PickerState::List {
                    context,
                    items,
                    selected_index: previous_index,
                    selected_id: None,
                    mode: PickerListMode::Browsing,
                };
            }
            let resolved_selected_id = previous_selected_id.or(selected_id);
            let resolved_index = picker_selected_index(
                &items,
                context,
                resolved_selected_id.as_deref(),
                previous_index,
                state,
            );
            let resolved_selected_id = items.get(resolved_index).and_then(|item| match item.kind {
                PickerItemKind::Option => Some(item.id.clone()),
                PickerItemKind::AddAction => None,
            });
            let resolved_mode = match previous_mode {
                PickerListMode::Browsing => PickerListMode::Browsing,
                PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag { tag_id },
                } if items.iter().any(|item| item.id == tag_id) => PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag { tag_id },
                },
                PickerListMode::Confirm { .. } => PickerListMode::Browsing,
            };

            PickerState::List {
                context,
                items,
                selected_index: resolved_index,
                selected_id: resolved_selected_id,
                mode: resolved_mode,
            }
        }
    }
}

fn picker_selected_index(
    items: &[PickerItem],
    context: PickerContext,
    preferred_selected_id: Option<&str>,
    preferred_index: usize,
    state: &CoreState,
) -> usize {
    if items.is_empty() {
        return 0;
    }

    let preferred_selected_id =
        preferred_selected_id
            .map(str::to_string)
            .or_else(|| match context {
                PickerContext::DefaultMemory => state.saved_default_memory_id.clone(),
                PickerContext::InsertTarget => {
                    let insert_memory_id = state.insert_memory_id.trim();
                    if insert_memory_id.is_empty() {
                        state.saved_default_memory_id.clone()
                    } else {
                        Some(insert_memory_id.to_string())
                    }
                }
                PickerContext::InsertTag => {
                    let insert_tag = state.insert_tag.trim();
                    (!insert_tag.is_empty()).then(|| insert_tag.to_string())
                }
                PickerContext::TagManagement
                | PickerContext::AddTag
                | PickerContext::ChatResultLimit
                | PickerContext::ChatPerMemoryLimit
                | PickerContext::ChatDiversity => None,
            });

    if let Some(selected_id) = preferred_selected_id
        && let Some(index) = items.iter().position(|item| item.id == selected_id)
    {
        return index;
    }

    preferred_index.min(items.len().saturating_sub(1))
}

fn open_picker(state: &mut CoreState, context: PickerContext) {
    state.picker = PickerState::List {
        context,
        items: Vec::new(),
        selected_index: 0,
        selected_id: None,
        mode: PickerListMode::Browsing,
    };
}

fn close_picker(state: &mut CoreState) {
    match &mut state.picker {
        PickerState::List { mode, .. } if matches!(mode, PickerListMode::Confirm { .. }) => {
            *mode = PickerListMode::Browsing;
        }
        _ => {
            state.picker = PickerState::Closed;
        }
    }
}

fn move_picker_next(state: &mut CoreState) {
    let PickerState::List {
        items,
        selected_index,
        selected_id,
        mode,
        ..
    } = &mut state.picker
    else {
        return;
    };

    if !matches!(mode, PickerListMode::Browsing) {
        return;
    }

    if items.is_empty() {
        *selected_index = 0;
        *selected_id = None;
        return;
    }

    *selected_index = (*selected_index + 1).min(items.len() - 1);
    *selected_id = items.get(*selected_index).and_then(|item| match item.kind {
        PickerItemKind::Option => Some(item.id.clone()),
        PickerItemKind::AddAction => None,
    });
}

fn move_picker_prev(state: &mut CoreState) {
    let PickerState::List {
        items,
        selected_index,
        selected_id,
        mode,
        ..
    } = &mut state.picker
    else {
        return;
    };

    if !matches!(mode, PickerListMode::Browsing) {
        return;
    }

    *selected_index = selected_index.saturating_sub(1);
    *selected_id = items.get(*selected_index).and_then(|item| match item.kind {
        PickerItemKind::Option => Some(item.id.clone()),
        PickerItemKind::AddAction => None,
    });
}

fn begin_picker_delete_confirm(state: &mut CoreState) {
    let PickerState::List {
        context,
        items,
        selected_index,
        mode,
        ..
    } = &mut state.picker
    else {
        return;
    };

    if *context != PickerContext::TagManagement || !matches!(mode, PickerListMode::Browsing) {
        return;
    }

    let Some(item) = items.get(*selected_index) else {
        return;
    };
    if item.kind != PickerItemKind::Option {
        return;
    }

    *mode = PickerListMode::Confirm {
        kind: PickerConfirmKind::DeleteTag {
            tag_id: item.id.clone(),
        },
    };
}

fn submit_picker(state: &mut CoreState) {
    match &mut state.picker {
        PickerState::Closed => {}
        PickerState::Input {
            context,
            origin_context,
            value,
        } => {
            if *context == PickerContext::AddTag {
                let tag = value.trim().to_string();
                if !tag.is_empty() && *origin_context == Some(PickerContext::InsertTag) {
                    state.insert_tag = tag;
                }
            }
        }
        PickerState::List {
            context,
            items,
            selected_index,
            selected_id,
            mode,
        } => {
            if let PickerListMode::Confirm { .. } = mode {
                return;
            }
            let Some(item) = items.get(*selected_index).cloned() else {
                return;
            };
            match item.kind {
                PickerItemKind::AddAction => {
                    state.picker = PickerState::Input {
                        context: PickerContext::AddTag,
                        origin_context: Some(*context),
                        value: String::new(),
                    };
                }
                PickerItemKind::Option => {
                    *selected_id = Some(item.id.clone());
                    if *context == PickerContext::InsertTag {
                        state.insert_tag = item.id;
                    }
                }
            }
        }
    }
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
    fn focus_content_resets_memory_content_action_index_on_memories_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            memory_content_action_index: 3,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusContent);

        assert_eq!(state.focus, PaneFocus::Content);
        assert_eq!(state.memory_content_action_index, 0);
    }

    #[test]
    fn open_selected_resets_memory_content_action_index_on_memories_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            memory_content_action_index: 2,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::OpenSelected);

        assert_eq!(state.focus, PaneFocus::Content);
        assert_eq!(state.memory_content_action_index, 0);
    }

    #[test]
    fn open_selected_keeps_focus_on_chat_when_memories_chat_replaces_content() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Items,
            chat_open: true,
            memory_content_action_index: 2,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::OpenSelected);

        assert_eq!(state.focus, PaneFocus::Extra);
        assert_eq!(state.memory_content_action_index, 2);
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
    fn toggle_chat_is_ignored_on_create_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ToggleChat);
        assert_eq!(state.focus, PaneFocus::Form);
        assert!(!state.chat_open);
    }

    #[test]
    fn switching_away_from_memories_closes_chat() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Extra,
            chat_open: true,
            ..CoreState::default()
        };

        apply_core_action(
            &mut state,
            &CoreAction::SetTab(CoreTabId::new(kinic_tabs::KINIC_CREATE_TAB_ID)),
        );

        assert_eq!(state.current_tab_id, kinic_tabs::KINIC_CREATE_TAB_ID);
        assert_eq!(state.focus, PaneFocus::Form);
        assert!(!state.chat_open);
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
    fn create_next_field_wraps_back_to_name() {
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
    fn rename_memory_next_field_wraps_back_to_name() {
        let mut state = CoreState {
            rename_memory: RenameMemoryModalState {
                form: TextInputModalState {
                    open: true,
                    ..TextInputModalState::default()
                },
                focus: RenameModalFocus::Submit,
                ..RenameMemoryModalState::default()
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::RenameMemoryNextField);

        assert_eq!(state.rename_memory.focus, RenameModalFocus::Name);
    }

    #[test]
    fn rename_memory_input_clears_error_state() {
        let mut state = CoreState {
            rename_memory: RenameMemoryModalState {
                form: TextInputModalState {
                    open: true,
                    submit_state: CreateSubmitState::Error,
                    error: Some("boom".to_string()),
                    ..TextInputModalState::default()
                },
                focus: RenameModalFocus::Name,
                ..RenameMemoryModalState::default()
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::RenameMemoryInput('A'));

        assert_eq!(state.rename_memory.form.value, "A");
        assert_eq!(
            state.rename_memory.form.submit_state,
            CreateSubmitState::Idle
        );
        assert_eq!(state.rename_memory.form.error, None);
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
            Some(CoreAction::SelectPrevTab)
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
    fn tabs_right_switches_to_next_tab() {
        assert_eq!(
            action_for_key(
                CoreKey::Right,
                PaneFocus::Tabs,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::SelectNextTab)
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
    fn memories_content_tab_uses_boundary_navigation_actions() {
        assert_eq!(
            action_for_key(
                CoreKey::Tab,
                PaneFocus::Content,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::MemoryContentJumpNext)
        );
        assert_eq!(
            action_for_key(
                CoreKey::BackTab,
                PaneFocus::Content,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::MemoryContentJumpPrev)
        );
    }

    #[test]
    fn memories_items_tab_moves_focus_to_content() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Items,
            selected_index: Some(0),
            list_items: vec![UiItemSummary {
                id: "1".to_string(),
                name: "a".to_string(),
                leading_marker: None,
                kind: tui_kit_model::UiItemKind::Custom("x".to_string()),
                visibility: tui_kit_model::UiVisibility::Private,
                qualified_name: None,
                subtitle: None,
                tags: vec![],
            }],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusNext);
        assert_eq!(state.focus, PaneFocus::Content);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn memories_items_tab_moves_focus_to_chat_when_chat_replaces_content() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Items,
            chat_open: true,
            selected_index: Some(0),
            list_items: vec![UiItemSummary {
                id: "1".to_string(),
                name: "a".to_string(),
                leading_marker: None,
                kind: tui_kit_model::UiItemKind::Custom("x".to_string()),
                visibility: tui_kit_model::UiVisibility::Private,
                qualified_name: None,
                subtitle: None,
                tags: vec![],
            }],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusNext);
        assert_eq!(state.focus, PaneFocus::Extra);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn memories_items_backtab_moves_focus_to_search() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Items,
            selected_index: Some(0),
            list_items: vec![UiItemSummary {
                id: "1".to_string(),
                name: "a".to_string(),
                leading_marker: None,
                kind: tui_kit_model::UiItemKind::Custom("x".to_string()),
                visibility: tui_kit_model::UiVisibility::Private,
                qualified_name: None,
                subtitle: None,
                tags: vec![],
            }],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusPrev);
        assert_eq!(state.focus, PaneFocus::Search);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn memories_chat_backtab_moves_focus_to_items_when_chat_replaces_content() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Extra,
            chat_open: true,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::FocusPrev);
        assert_eq!(state.focus, PaneFocus::Items);
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
    fn memories_chat_focus_maps_left_right_to_scope_changes() {
        assert_eq!(
            action_for_key(
                CoreKey::Left,
                PaneFocus::Extra,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::ChatScopePrev)
        );
        assert_eq!(
            action_for_key(
                CoreKey::Right,
                PaneFocus::Extra,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::ChatScopeNext)
        );
        assert_eq!(
            action_for_key(
                CoreKey::Left,
                PaneFocus::Extra,
                kinic_tabs::KINIC_CREATE_TAB_ID
            ),
            None
        );
    }

    #[test]
    fn chat_scope_actions_only_mutate_memories_tab() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            chat_scope: ChatScope::Selected,
            selected_index: Some(0),
            list_items: vec![UiItemSummary {
                id: "aaaaa-aa".to_string(),
                name: "Alpha Memory".to_string(),
                leading_marker: None,
                kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                visibility: tui_kit_model::UiVisibility::Private,
                qualified_name: None,
                subtitle: None,
                tags: vec![],
            }],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ChatScopeNext);
        assert_eq!(state.chat_scope, ChatScope::All);

        apply_core_action(&mut state, &CoreAction::ChatScopePrev);
        assert_eq!(state.chat_scope, ChatScope::Selected);

        state.current_tab_id = kinic_tabs::KINIC_CREATE_TAB_ID.to_string();
        apply_core_action(&mut state, &CoreAction::ChatScopeNext);
        assert_eq!(state.chat_scope, ChatScope::Selected);
    }

    #[test]
    fn chat_scope_selected_persists_while_list_selection_moves() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            chat_scope: ChatScope::Selected,
            selected_index: Some(0),
            list_items: vec![
                UiItemSummary {
                    id: "aaaaa-aa".to_string(),
                    name: "Alpha Memory".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "bbbbb-bb".to_string(),
                    name: "Beta Memory".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
            ],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::MoveNext);

        assert_eq!(state.selected_index, Some(1));
        assert_eq!(state.chat_scope, ChatScope::Selected);
    }

    #[test]
    fn chat_scope_cycles_through_visible_memory_items() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            chat_scope: ChatScope::Selected,
            selected_index: Some(0),
            list_items: vec![
                UiItemSummary {
                    id: "aaaaa-aa".to_string(),
                    name: "Alpha Memory".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "kinic-action-add-memory".to_string(),
                    name: "+ Add Existing Memory Canister".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("action".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "bbbbb-bb".to_string(),
                    name: "Beta Memory".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
            ],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ChatScopeNext);
        assert_eq!(state.chat_scope, ChatScope::Selected);
        assert_eq!(state.selected_index, Some(2));
        assert_eq!(
            state
                .selected_index
                .and_then(|i| state.list_items.get(i))
                .map(|item| item.id.as_str()),
            Some("bbbbb-bb")
        );

        apply_core_action(&mut state, &CoreAction::ChatScopeNext);
        assert_eq!(state.chat_scope, ChatScope::All);

        apply_core_action(&mut state, &CoreAction::ChatScopePrev);
        assert_eq!(state.chat_scope, ChatScope::Selected);
        assert_eq!(state.selected_index, Some(2));
        assert_eq!(
            state
                .selected_index
                .and_then(|i| state.list_items.get(i))
                .map(|item| item.id.as_str()),
            Some("bbbbb-bb")
        );
    }

    #[test]
    fn chat_scope_next_from_all_uses_first_visible_memory_not_browser_cursor() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            chat_scope: ChatScope::All,
            selected_index: Some(2),
            list_items: vec![
                UiItemSummary {
                    id: "aaaaa-aa".to_string(),
                    name: "Alpha Memory".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "kinic-action-add-memory".to_string(),
                    name: "+ Add Existing Memory Canister".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("action".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "bbbbb-bb".to_string(),
                    name: "Beta Memory".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
            ],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ChatScopeNext);

        assert_eq!(state.chat_scope, ChatScope::Selected);
        assert_eq!(state.selected_index, Some(0));
        assert_eq!(
            state
                .selected_index
                .and_then(|i| state.list_items.get(i))
                .map(|item| item.id.as_str()),
            Some("aaaaa-aa")
        );
    }

    #[test]
    fn chat_scope_all_switches_selected_scope_back_to_all() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            chat_scope: ChatScope::Selected,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ChatScopeAll);

        assert_eq!(state.chat_scope, ChatScope::All);
    }

    #[test]
    fn chat_scope_next_returns_to_all_when_no_visible_memories() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            chat_scope: ChatScope::Selected,
            list_items: vec![UiItemSummary {
                id: "kinic-action-add-memory".to_string(),
                name: "+ Add Existing Memory Canister".to_string(),
                leading_marker: None,
                kind: tui_kit_model::UiItemKind::Custom("action".to_string()),
                visibility: tui_kit_model::UiVisibility::Private,
                qualified_name: None,
                subtitle: None,
                tags: vec![],
            }],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ChatScopeNext);

        assert_eq!(state.chat_scope, ChatScope::All);
    }

    #[test]
    fn chat_scope_prev_returns_to_all_when_no_visible_memories() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            chat_scope: ChatScope::Selected,
            list_items: vec![UiItemSummary {
                id: "kinic-action-add-memory".to_string(),
                name: "+ Add Existing Memory Canister".to_string(),
                leading_marker: None,
                kind: tui_kit_model::UiItemKind::Custom("action".to_string()),
                visibility: tui_kit_model::UiVisibility::Private,
                qualified_name: None,
                subtitle: None,
                tags: vec![],
            }],
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ChatScopePrev);

        assert_eq!(state.chat_scope, ChatScope::All);
    }

    #[test]
    fn memories_chat_focus_maps_regular_chars_to_chat_input() {
        assert_eq!(
            action_for_key(
                CoreKey::Char('N'),
                PaneFocus::Extra,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::ChatInput('N'))
        );
        assert_eq!(
            action_for_key(
                CoreKey::Char('t'),
                PaneFocus::Extra,
                kinic_tabs::KINIC_MEMORIES_TAB_ID
            ),
            Some(CoreAction::ChatInput('t'))
        );
    }

    #[test]
    fn chat_new_thread_does_not_mutate_runtime_state_directly() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            chat_messages: vec![("user".to_string(), "hello".to_string())],
            chat_loading: true,
            chat_scroll: 7,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ChatNewThread);
        assert_eq!(
            state.chat_messages,
            vec![("user".to_string(), "hello".to_string())]
        );
        assert!(state.chat_loading);
        assert_eq!(state.chat_scroll, 7);

        state.current_tab_id = kinic_tabs::KINIC_CREATE_TAB_ID.to_string();
        state.chat_messages = vec![("user".to_string(), "keep".to_string())];
        apply_core_action(&mut state, &CoreAction::ChatNewThread);
        assert_eq!(
            state.chat_messages,
            vec![("user".to_string(), "keep".to_string())]
        );
    }

    #[test]
    fn selected_settings_row_behavior_matches_default_memory_row() {
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

        assert_eq!(
            selected_settings_row_behavior(&state),
            Some(SettingsRowBehavior {
                enter_action: Some(CoreAction::OpenPicker(PickerContext::DefaultMemory)),
                status_hint: " open Default memory ",
            })
        );
        assert_eq!(
            selected_settings_row_behavior(&other_row_state),
            Some(SettingsRowBehavior {
                enter_action: None,
                status_hint: " row details ",
            })
        );
    }

    #[test]
    fn selected_settings_row_behavior_matches_transfer_row() {
        let settings = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![SettingsSection {
                title: "Account".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: "kinic_balance".to_string(),
                        label: "KINIC balance".to_string(),
                        value: "1.00000000 KINIC".to_string(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "principal_id".to_string(),
                        label: "Principal ID".to_string(),
                        value: "aaaaa-aa".to_string(),
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

        assert_eq!(
            selected_settings_row_behavior(&state),
            Some(SettingsRowBehavior {
                enter_action: Some(CoreAction::OpenTransferModal),
                status_hint: " send KINIC ",
            })
        );
        assert_eq!(
            selected_settings_row_behavior(&other_row_state),
            Some(SettingsRowBehavior {
                enter_action: None,
                status_hint: " row details ",
            })
        );
    }

    #[test]
    fn selected_settings_row_behavior_matches_saved_tags_row() {
        let settings = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![SettingsSection {
                title: "Saved tags".to_string(),
                entries: vec![SettingsEntry {
                    id: SETTINGS_ENTRY_SAVED_TAGS_ID.to_string(),
                    label: "Saved tags".to_string(),
                    value: "2".to_string(),
                    note: None,
                }],
                footer: None,
            }],
        };
        let state = CoreState {
            current_tab_id: kinic_tabs::KINIC_SETTINGS_TAB_ID.to_string(),
            focus: PaneFocus::Content,
            selected_index: Some(0),
            settings,
            ..CoreState::default()
        };

        assert_eq!(
            selected_settings_row_behavior(&state),
            Some(SettingsRowBehavior {
                enter_action: Some(CoreAction::OpenPicker(PickerContext::TagManagement)),
                status_hint: " manage saved tags ",
            })
        );
    }

    #[test]
    fn selected_settings_row_behavior_matches_chat_result_limit_row() {
        let settings = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![SettingsSection {
                title: "Chat retrieval".to_string(),
                entries: vec![SettingsEntry {
                    id: SETTINGS_ENTRY_CHAT_RESULT_LIMIT_ID.to_string(),
                    label: "Chat result limit".to_string(),
                    value: "8 docs".to_string(),
                    note: None,
                }],
                footer: None,
            }],
        };
        let state = CoreState {
            current_tab_id: kinic_tabs::KINIC_SETTINGS_TAB_ID.to_string(),
            focus: PaneFocus::Content,
            selected_index: Some(0),
            settings,
            ..CoreState::default()
        };

        assert_eq!(
            selected_settings_row_behavior(&state),
            Some(SettingsRowBehavior {
                enter_action: Some(CoreAction::OpenPicker(PickerContext::ChatResultLimit)),
                status_hint: " adjust chat limit ",
            })
        );
    }

    #[test]
    fn transfer_apply_max_sets_amount_to_balance_minus_fee() {
        let mut state = CoreState {
            transfer_modal: TransferModalState {
                open: true,
                mode: TransferModalMode::Edit,
                available_balance_base_units: Some(1_000_000_000),
                fee_base_units: Some(100_000),
                focus: TransferModalFocus::Amount,
                ..TransferModalState::default()
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::TransferApplyMax);

        assert_eq!(state.transfer_modal.amount, "9.99900000");
        assert_eq!(state.transfer_modal.focus, TransferModalFocus::Submit);
    }

    #[test]
    fn transfer_amount_input_rejects_non_numeric_characters() {
        let mut state = CoreState {
            transfer_modal: TransferModalState {
                open: true,
                mode: TransferModalMode::Edit,
                focus: TransferModalFocus::Amount,
                ..TransferModalState::default()
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::TransferInput('1'));
        apply_core_action(&mut state, &CoreAction::TransferInput('x'));
        apply_core_action(&mut state, &CoreAction::TransferInput('.'));
        apply_core_action(&mut state, &CoreAction::TransferInput('2'));
        apply_core_action(&mut state, &CoreAction::TransferInput('.'));

        assert_eq!(state.transfer_modal.amount, "1.2");
    }

    #[test]
    fn transfer_amount_input_caps_fractional_precision_at_eight_digits() {
        let mut state = CoreState {
            transfer_modal: TransferModalState {
                open: true,
                mode: TransferModalMode::Edit,
                focus: TransferModalFocus::Amount,
                amount: "0.".to_string(),
                ..TransferModalState::default()
            },
            ..CoreState::default()
        };

        for next in "123456789".chars() {
            apply_core_action(&mut state, &CoreAction::TransferInput(next));
        }

        assert_eq!(state.transfer_modal.amount, "0.12345678");
    }

    #[test]
    fn transfer_principal_input_rejects_non_principal_characters() {
        let mut state = CoreState {
            transfer_modal: TransferModalState {
                open: true,
                mode: TransferModalMode::Edit,
                focus: TransferModalFocus::Principal,
                ..TransferModalState::default()
            },
            ..CoreState::default()
        };

        for next in "abC-1_+".chars() {
            apply_core_action(&mut state, &CoreAction::TransferInput(next));
        }

        assert_eq!(state.transfer_modal.principal_id, "ab-1");
    }

    #[test]
    fn access_principal_input_rejects_non_principal_characters() {
        let mut state = CoreState {
            access_control: AccessControlModalState {
                open: true,
                mode: AccessControlMode::Add,
                focus: AccessControlFocus::Principal,
                ..AccessControlModalState::default()
            },
            ..CoreState::default()
        };

        for next in "anonyMous!?-1".chars() {
            apply_core_action(&mut state, &CoreAction::AccessInput(next));
        }

        assert_eq!(state.access_control.principal_id, "anonyous-1");
    }

    #[test]
    fn add_memory_input_rejects_non_principal_characters() {
        let mut state = CoreState {
            add_memory: TextInputModalState {
                open: true,
                ..TextInputModalState::default()
            },
            ..CoreState::default()
        };

        for next in "aaaaA-aa\n_1".chars() {
            apply_core_action(&mut state, &CoreAction::AddMemoryInput(next));
        }

        assert_eq!(state.add_memory.value, "aaaa-aa1");
    }

    #[test]
    fn close_transfer_modal_resets_submit_error_and_focus() {
        let mut state = CoreState {
            transfer_modal: TransferModalState {
                open: true,
                mode: TransferModalMode::Confirm,
                confirm_yes: false,
                submit_state: CreateSubmitState::Error,
                error: Some("boom".to_string()),
                focus: TransferModalFocus::Submit,
                ..TransferModalState::default()
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::CloseTransferModal);

        assert!(!state.transfer_modal.open);
        assert_eq!(state.transfer_modal.mode, TransferModalMode::Edit);
        assert!(state.transfer_modal.confirm_yes);
        assert_eq!(state.transfer_modal.submit_state, CreateSubmitState::Idle);
        assert_eq!(state.transfer_modal.error, None);
        assert_eq!(state.transfer_modal.focus, TransferModalFocus::Principal);
    }

    #[test]
    fn close_add_memory_resets_submit_error() {
        let mut state = CoreState {
            add_memory: TextInputModalState {
                open: true,
                submit_state: CreateSubmitState::Error,
                error: Some("boom".to_string()),
                ..TextInputModalState::default()
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::CloseAddMemory);

        assert!(!state.add_memory.open);
        assert_eq!(state.add_memory.submit_state, CreateSubmitState::Idle);
        assert_eq!(state.add_memory.error, None);
    }

    #[test]
    fn close_access_control_resets_submit_error_and_confirm() {
        let mut state = CoreState {
            access_control: AccessControlModalState {
                open: true,
                mode: AccessControlMode::Confirm,
                confirm_yes: false,
                submit_state: CreateSubmitState::Error,
                error: Some("boom".to_string()),
                ..AccessControlModalState::default()
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::CloseAccessControl);

        assert!(!state.access_control.open);
        assert_eq!(state.access_control.mode, AccessControlMode::None);
        assert!(state.access_control.confirm_yes);
        assert_eq!(state.access_control.submit_state, CreateSubmitState::Idle);
        assert_eq!(state.access_control.error, None);
    }

    #[test]
    fn transfer_and_access_confirm_toggles_still_flip_selection() {
        let mut transfer_state = CoreState {
            transfer_modal: TransferModalState {
                open: true,
                mode: TransferModalMode::Confirm,
                confirm_yes: true,
                ..TransferModalState::default()
            },
            ..CoreState::default()
        };
        apply_core_action(&mut transfer_state, &CoreAction::TransferConfirmToggle);
        assert!(!transfer_state.transfer_modal.confirm_yes);

        let mut access_state = CoreState {
            access_control: AccessControlModalState {
                open: true,
                mode: AccessControlMode::Confirm,
                confirm_yes: true,
                ..AccessControlModalState::default()
            },
            ..CoreState::default()
        };
        apply_core_action(&mut access_state, &CoreAction::AccessCycleAction);
        assert!(!access_state.access_control.confirm_yes);

        let mut remove_memory_state = CoreState {
            remove_memory: RemoveMemoryModalState {
                open: true,
                confirm_yes: true,
                ..RemoveMemoryModalState::default()
            },
            ..CoreState::default()
        };
        apply_core_action(
            &mut remove_memory_state,
            &CoreAction::RemoveMemoryToggleConfirm,
        );
        assert!(!remove_memory_state.remove_memory.confirm_yes);
    }

    #[test]
    fn open_insert_target_picker_uses_selected_anchor() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_INSERT_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            insert_focus: InsertFormFocus::MemoryId,
            picker: PickerState::List {
                context: PickerContext::InsertTarget,
                items: vec![
                    PickerItem::option("aaaaa-aa", "Alpha Memory", false),
                    PickerItem::option("bbbbb-bb", "Beta Memory", false),
                ],
                selected_index: 0,
                selected_id: Some("bbbbb-bb".to_string()),
                mode: PickerListMode::Browsing,
            },
            ..CoreState::default()
        };

        apply_core_action(
            &mut state,
            &CoreAction::OpenPicker(PickerContext::InsertTarget),
        );

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::InsertTarget,
                items: Vec::new(),
                selected_index: 0,
                selected_id: None,
                mode: PickerListMode::Browsing,
            }
        );
    }

    #[test]
    fn open_insert_target_picker_prefers_explicit_insert_target_selection() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_INSERT_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            insert_focus: InsertFormFocus::MemoryId,
            insert_memory_id: "aaaaa-aa".to_string(),
            ..CoreState::default()
        };

        apply_core_action(
            &mut state,
            &CoreAction::OpenPicker(PickerContext::InsertTarget),
        );
        apply_snapshot(
            &mut state,
            ProviderSnapshot {
                picker: PickerState::List {
                    context: PickerContext::InsertTarget,
                    items: vec![
                        PickerItem::option("aaaaa-aa", "Alpha Memory", false),
                        PickerItem::option("bbbbb-bb", "Beta Memory", false),
                    ],
                    selected_index: 0,
                    selected_id: None,
                    mode: PickerListMode::Browsing,
                },
                ..ProviderSnapshot::default()
            },
        );

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::InsertTarget,
                items: vec![
                    PickerItem::option("aaaaa-aa", "Alpha Memory", false),
                    PickerItem::option("bbbbb-bb", "Beta Memory", false),
                ],
                selected_index: 0,
                selected_id: Some("aaaaa-aa".to_string()),
                mode: PickerListMode::Browsing,
            }
        );
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
    fn apply_snapshot_aligns_selected_index_with_selected_content_id() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_MEMORIES_TAB_ID.to_string(),
            selected_index: Some(0),
            ..CoreState::default()
        };
        let snapshot = ProviderSnapshot {
            items: vec![
                UiItemSummary {
                    id: "aaaaa-aa".to_string(),
                    name: "Alpha".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "bbbbb-bb".to_string(),
                    name: "Beta".to_string(),
                    leading_marker: None,
                    kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
            ],
            selected_content: Some(UiItemContent {
                id: "bbbbb-bb".to_string(),
                title: "Beta".to_string(),
                subtitle: None,
                kind: tui_kit_model::UiItemKind::Custom("memory".to_string()),
                definition: String::new(),
                location: None,
                docs: None,
                badges: vec![],
                sections: vec![],
            }),
            total_count: 2,
            ..ProviderSnapshot::default()
        };

        apply_snapshot(&mut state, snapshot);

        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn apply_snapshot_updates_insert_placeholder() {
        let mut state = CoreState::default();
        let snapshot = ProviderSnapshot {
            saved_default_memory_id: Some("aaaaa-aa".to_string()),
            insert_memory_placeholder: Some("Alpha Memory".to_string()),
            ..ProviderSnapshot::default()
        };

        apply_snapshot(&mut state, snapshot);

        assert_eq!(state.saved_default_memory_id.as_deref(), Some("aaaaa-aa"));
        assert_eq!(
            state.insert_memory_placeholder.as_deref(),
            Some("Alpha Memory")
        );
    }

    #[test]
    fn apply_snapshot_keeps_insert_memory_id_empty_when_only_default_exists() {
        let mut state = CoreState {
            current_tab_id: kinic_tabs::KINIC_INSERT_TAB_ID.to_string(),
            ..CoreState::default()
        };
        let snapshot = ProviderSnapshot {
            saved_default_memory_id: Some("aaaaa-aa".to_string()),
            ..ProviderSnapshot::default()
        };

        apply_snapshot(&mut state, snapshot);

        assert_eq!(state.insert_memory_id, "");
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
        assert_eq!(state.selected_index, Some(0));

        apply_core_action(&mut state, &CoreAction::MovePrev);
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
    fn is_insert_form_locked_only_while_submit_is_running() {
        let idle = CoreState::default();
        assert!(!is_insert_form_locked(&idle));

        let submitting = CoreState {
            insert_submit_state: CreateSubmitState::Submitting,
            ..CoreState::default()
        };
        assert!(is_insert_form_locked(&submitting));

        let error = CoreState {
            insert_submit_state: CreateSubmitState::Error,
            ..CoreState::default()
        };
        assert!(!is_insert_form_locked(&error));
    }

    #[test]
    fn insert_memory_id_ignores_direct_text_editing() {
        let mut state = CoreState {
            insert_focus: InsertFormFocus::MemoryId,
            insert_memory_id: "aaaaa-aa".to_string(),
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertInput('x'));
        assert_eq!(state.insert_memory_id, "aaaaa-aa");

        apply_core_action(&mut state, &CoreAction::InsertBackspace);

        assert_eq!(state.insert_memory_id, "aaaaa-aa");
    }

    #[test]
    fn insert_file_path_backspace_edits_selected_path_buffer() {
        let mut state = CoreState {
            insert_focus: InsertFormFocus::FilePath,
            insert_file_path_input: "/tmp/doc.pdf".to_string(),
            insert_selected_file_path: Some(PathBuf::from("/tmp/doc.pdf")),
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertBackspace);

        assert_eq!(state.insert_selected_file_path, None);
        assert_eq!(state.insert_file_path_input, "/tmp/doc.pd");
    }

    #[test]
    fn insert_file_path_input_appends_to_selected_path_buffer() {
        let mut state = CoreState {
            insert_focus: InsertFormFocus::FilePath,
            insert_file_path_input: "/tmp/doc.pdf".to_string(),
            insert_selected_file_path: Some(PathBuf::from("/tmp/doc.pdf")),
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertInput('x'));

        assert_eq!(state.insert_selected_file_path, None);
        assert_eq!(state.insert_file_path_input, "/tmp/doc.pdfx");
    }

    #[test]
    fn insert_navigation_is_ignored_while_submit_is_running() {
        let mut state = CoreState {
            insert_mode: InsertMode::InlineText,
            insert_focus: InsertFormFocus::Text,
            insert_submit_state: CreateSubmitState::Submitting,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertNextField);
        apply_core_action(&mut state, &CoreAction::InsertNextMode);

        assert_eq!(state.insert_focus, InsertFormFocus::Text);
        assert_eq!(state.insert_mode, InsertMode::InlineText);
    }

    #[test]
    fn picker_move_next_can_land_on_add_action_for_tag_management() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Browsing,
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::MovePickerNext);

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 1,
                selected_id: None,
                mode: PickerListMode::Browsing,
            }
        );
    }

    #[test]
    fn picker_submit_turns_add_action_into_input_mode() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::InsertTag,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 1,
                selected_id: None,
                mode: PickerListMode::Browsing,
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::SubmitPicker);

        assert_eq!(
            state.picker,
            PickerState::Input {
                context: PickerContext::AddTag,
                origin_context: Some(PickerContext::InsertTag),
                value: String::new(),
            }
        );
    }

    #[test]
    fn apply_snapshot_preserves_picker_add_action_selection() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 1,
                selected_id: None,
                mode: PickerListMode::Browsing,
            },
            ..CoreState::default()
        };
        let snapshot = ProviderSnapshot {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Browsing,
            },
            ..ProviderSnapshot::default()
        };

        apply_snapshot(&mut state, snapshot);

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 1,
                selected_id: None,
                mode: PickerListMode::Browsing,
            }
        );
    }

    #[test]
    fn open_picker_uses_saved_default_memory_selection_after_snapshot() {
        let mut state = CoreState {
            saved_default_memory_id: Some("bbbbb-bb".to_string()),
            ..CoreState::default()
        };
        apply_core_action(
            &mut state,
            &CoreAction::OpenPicker(PickerContext::DefaultMemory),
        );
        apply_snapshot(
            &mut state,
            ProviderSnapshot {
                picker: PickerState::List {
                    context: PickerContext::DefaultMemory,
                    items: vec![
                        PickerItem::option("aaaaa-aa", "Alpha Memory", false),
                        PickerItem::option("bbbbb-bb", "Beta Memory", true),
                    ],
                    selected_index: 0,
                    selected_id: None,
                    mode: PickerListMode::Browsing,
                },
                ..ProviderSnapshot::default()
            },
        );

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::DefaultMemory,
                items: vec![
                    PickerItem::option("aaaaa-aa", "Alpha Memory", false),
                    PickerItem::option("bbbbb-bb", "Beta Memory", true),
                ],
                selected_index: 1,
                selected_id: Some("bbbbb-bb".to_string()),
                mode: PickerListMode::Browsing,
            }
        );
    }

    #[test]
    fn open_picker_uses_insert_target_selection_after_snapshot() {
        let mut state = CoreState {
            insert_memory_id: "bbbbb-bb".to_string(),
            ..CoreState::default()
        };

        apply_core_action(
            &mut state,
            &CoreAction::OpenPicker(PickerContext::InsertTarget),
        );
        apply_snapshot(
            &mut state,
            ProviderSnapshot {
                picker: PickerState::List {
                    context: PickerContext::InsertTarget,
                    items: vec![
                        PickerItem::option("aaaaa-aa", "Alpha Memory", false),
                        PickerItem::option("bbbbb-bb", "Beta Memory", false),
                    ],
                    selected_index: 0,
                    selected_id: None,
                    mode: PickerListMode::Browsing,
                },
                ..ProviderSnapshot::default()
            },
        );

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::InsertTarget,
                items: vec![
                    PickerItem::option("aaaaa-aa", "Alpha Memory", false),
                    PickerItem::option("bbbbb-bb", "Beta Memory", false),
                ],
                selected_index: 1,
                selected_id: Some("bbbbb-bb".to_string()),
                mode: PickerListMode::Browsing,
            }
        );
    }

    #[test]
    fn picker_submit_updates_insert_tag_for_selected_item() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::InsertTag,
                items: vec![PickerItem::option("docs", "docs", false)],
                selected_index: 0,
                selected_id: None,
                mode: PickerListMode::Browsing,
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::SubmitPicker);

        assert_eq!(state.insert_tag, "docs");
    }

    #[test]
    fn picker_input_submit_keeps_local_tag_value() {
        let mut state = CoreState {
            picker: PickerState::Input {
                context: PickerContext::AddTag,
                origin_context: Some(PickerContext::InsertTag),
                value: "research".to_string(),
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::SubmitPicker);

        assert_eq!(state.insert_tag, "research");
    }

    #[test]
    fn picker_delete_key_enters_confirm_mode_for_tag_management_option() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Browsing,
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::DeleteSelectedPickerItem);

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag {
                        tag_id: "docs".to_string(),
                    },
                },
            }
        );
    }

    #[test]
    fn picker_delete_confirm_blocks_navigation_until_canceled() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::option("research", "research", false),
                ],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag {
                        tag_id: "docs".to_string(),
                    },
                },
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::MovePickerNext);

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::option("research", "research", false),
                ],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag {
                        tag_id: "docs".to_string(),
                    },
                },
            }
        );
    }

    #[test]
    fn apply_snapshot_preserves_delete_confirm_when_tag_still_exists() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::option("research", "research", false),
                ],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag {
                        tag_id: "docs".to_string(),
                    },
                },
            },
            ..CoreState::default()
        };

        apply_snapshot(
            &mut state,
            ProviderSnapshot {
                picker: PickerState::List {
                    context: PickerContext::TagManagement,
                    items: vec![
                        PickerItem::option("docs", "docs", false),
                        PickerItem::option("research", "research", false),
                    ],
                    selected_index: 1,
                    selected_id: Some("research".to_string()),
                    mode: PickerListMode::Browsing,
                },
                ..ProviderSnapshot::default()
            },
        );

        assert!(matches!(
            state.picker,
            PickerState::List {
                context: PickerContext::TagManagement,
                mode: PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag { ref tag_id },
                },
                ..
            } if tag_id == "docs"
        ));
    }

    #[test]
    fn apply_snapshot_clears_delete_confirm_when_tag_disappears() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::option("research", "research", false),
                ],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag {
                        tag_id: "docs".to_string(),
                    },
                },
            },
            ..CoreState::default()
        };

        apply_snapshot(
            &mut state,
            ProviderSnapshot {
                picker: PickerState::List {
                    context: PickerContext::TagManagement,
                    items: vec![PickerItem::option("research", "research", false)],
                    selected_index: 0,
                    selected_id: Some("research".to_string()),
                    mode: PickerListMode::Browsing,
                },
                ..ProviderSnapshot::default()
            },
        );

        assert!(matches!(
            state.picker,
            PickerState::List {
                context: PickerContext::TagManagement,
                mode: PickerListMode::Browsing,
                ..
            }
        ));
    }

    #[test]
    fn picker_delete_key_ignores_add_action_rows() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![
                    PickerItem::option("docs", "docs", false),
                    PickerItem::add_action("+ Add new tag"),
                ],
                selected_index: 1,
                selected_id: None,
                mode: PickerListMode::Browsing,
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::DeleteSelectedPickerItem);

        assert!(matches!(
            state.picker,
            PickerState::List {
                context: PickerContext::TagManagement,
                selected_index: 1,
                selected_id: None,
                mode: PickerListMode::Browsing,
                ..
            }
        ));
    }

    #[test]
    fn close_picker_cancels_delete_confirm_before_closing() {
        let mut state = CoreState {
            picker: PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![PickerItem::option("docs", "docs", false)],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Confirm {
                    kind: PickerConfirmKind::DeleteTag {
                        tag_id: "docs".to_string(),
                    },
                },
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::ClosePicker);

        assert_eq!(
            state.picker,
            PickerState::List {
                context: PickerContext::TagManagement,
                items: vec![PickerItem::option("docs", "docs", false)],
                selected_index: 0,
                selected_id: Some("docs".to_string()),
                mode: PickerListMode::Browsing,
            }
        );
    }

    #[test]
    fn picker_input_submit_from_tag_management_does_not_touch_insert_tag() {
        let mut state = CoreState {
            insert_tag: "docs".to_string(),
            picker: PickerState::Input {
                context: PickerContext::AddTag,
                origin_context: Some(PickerContext::TagManagement),
                value: "research".to_string(),
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::SubmitPicker);

        assert_eq!(state.insert_tag, "docs");
    }

    #[test]
    fn picker_input_submit_without_origin_does_not_touch_insert_tag() {
        let mut state = CoreState {
            insert_tag: "docs".to_string(),
            picker: PickerState::Input {
                context: PickerContext::AddTag,
                origin_context: None,
                value: "research".to_string(),
            },
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::SubmitPicker);

        assert_eq!(state.insert_tag, "docs");
    }

    #[test]
    fn insert_prev_mode_moves_to_inline_text_and_resets_focus() {
        let mut state = CoreState {
            insert_mode: InsertMode::ManualEmbedding,
            insert_focus: InsertFormFocus::Embedding,
            insert_error: Some("boom".to_string()),
            insert_submit_state: CreateSubmitState::Error,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertPrevMode);

        assert_eq!(state.insert_mode, InsertMode::InlineText);
        assert_eq!(state.insert_focus, InsertFormFocus::Mode);
        assert_eq!(state.insert_error, None);
        assert_eq!(state.insert_submit_state, CreateSubmitState::Idle);
    }

    #[test]
    fn insert_mode_wraps_between_first_and_last_modes() {
        let mut state = CoreState {
            insert_mode: InsertMode::File,
            insert_focus: InsertFormFocus::Mode,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertPrevMode);
        assert_eq!(state.insert_mode, InsertMode::ManualEmbedding);

        apply_core_action(&mut state, &CoreAction::InsertNextMode);
        assert_eq!(state.insert_mode, InsertMode::File);
        assert_eq!(state.insert_focus, InsertFormFocus::Mode);
    }

    #[test]
    fn insert_file_mode_skips_text_and_embedding_fields() {
        let mut state = CoreState {
            insert_mode: InsertMode::File,
            insert_focus: InsertFormFocus::Tag,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertNextField);
        assert_eq!(state.insert_focus, InsertFormFocus::FilePath);

        apply_core_action(&mut state, &CoreAction::InsertNextField);
        assert_eq!(state.insert_focus, InsertFormFocus::Submit);
    }

    #[test]
    fn insert_next_mode_visits_file_then_inline_text_before_raw() {
        let mut state = CoreState {
            insert_mode: InsertMode::File,
            insert_focus: InsertFormFocus::Mode,
            ..CoreState::default()
        };

        apply_core_action(&mut state, &CoreAction::InsertNextMode);
        assert_eq!(state.insert_mode, InsertMode::InlineText);

        apply_core_action(&mut state, &CoreAction::InsertNextMode);
        assert_eq!(state.insert_mode, InsertMode::ManualEmbedding);

        apply_core_action(&mut state, &CoreAction::InsertNextMode);
        assert_eq!(state.insert_mode, InsertMode::File);
    }

    #[test]
    fn core_action_to_form_command_preserves_insert_picker_and_mode_commands() {
        assert_eq!(
            core_action_to_form_command(&CoreAction::OpenPicker(PickerContext::InsertTarget)),
            Some((
                FormKind::Insert,
                FormCommand::OpenPicker(PickerContext::InsertTarget),
            ))
        );
        assert_eq!(
            core_action_to_form_command(&CoreAction::InsertNextMode),
            Some((FormKind::Insert, FormCommand::HorizontalChangeNext))
        );
    }

    #[test]
    fn form_command_to_action_preserves_create_and_insert_submit_actions() {
        assert_eq!(
            form_command_to_action(FormKind::Create, FormCommand::Submit),
            Some(CoreAction::CreateSubmit)
        );
        assert_eq!(
            form_command_to_action(FormKind::Insert, FormCommand::Submit),
            Some(CoreAction::InsertSubmit)
        );
    }
}
