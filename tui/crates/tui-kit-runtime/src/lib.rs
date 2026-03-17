//! Domain-agnostic core contracts for driving tui-kit style UIs.
//!
//! This crate defines generic actions/effects, shared runtime state, and the
//! `DataProvider` trait so multiple domains can plug into the same UI shell.

use tui_kit_model::{UiContextNode, UiItemDetail, UiItemSummary};

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
    List,
    Tabs,
    Detail,
    Extra,
}

/// Domain-agnostic runtime state owned by the core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreState {
    pub current_tab_id: String,
    pub focus: PaneFocus,
    pub query: String,
    pub selected_index: Option<usize>,
    pub list_items: Vec<UiItemSummary>,
    pub selected_detail: Option<UiItemDetail>,
    pub selected_context: Option<UiContextNode>,
    pub total_count: usize,
    pub status_message: Option<String>,
    pub chat_open: bool,
    pub chat_messages: Vec<(String, String)>,
    pub chat_input: String,
    pub chat_loading: bool,
    pub chat_scroll: usize,
}

impl Default for CoreState {
    fn default() -> Self {
        Self {
            current_tab_id: "default".to_string(),
            focus: PaneFocus::Search,
            query: String::new(),
            selected_index: None,
            list_items: Vec::new(),
            selected_detail: None,
            selected_context: None,
            total_count: 0,
            status_message: None,
            chat_open: false,
            chat_messages: Vec::new(),
            chat_input: String::new(),
            chat_loading: false,
            chat_scroll: 0,
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
    FocusNext,
    FocusPrev,
    FocusSearch,
    FocusList,
    FocusDetail,
    OpenSelected,
    Back,
    SearchInput(char),
    SearchBackspace,
    SearchSubmit,
    SetQuery(String),
    SelectTabIndex(usize),
    SelectNextTab,
    SelectPrevTab,
    SetTab(CoreTabId),
    ToggleHelp,
    ToggleSettings,
    ToggleChat,
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
    Custom { id: String, payload: Option<String> },
}

/// Provider-owned snapshot sent to core/UI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderSnapshot {
    pub items: Vec<UiItemSummary>,
    pub selected_detail: Option<UiItemDetail>,
    pub selected_context: Option<UiContextNode>,
    pub total_count: usize,
    pub status_message: Option<String>,
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
}

/// Apply one core action to local runtime state.
///
/// Providers may further modify visible data via snapshots; this reducer handles
/// local interaction state (query, tab, focus, selection).
pub fn apply_core_action(state: &mut CoreState, action: &CoreAction) {
    match action {
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
        CoreAction::SetTab(tab_id) => {
            state.current_tab_id = tab_id.0.clone();
            state.selected_index = Some(0);
        }
        CoreAction::SelectTabIndex(index) => {
            state.current_tab_id = format!("tab-{}", index + 1);
            state.selected_index = Some(0);
        }
        CoreAction::FocusNext => {
            state.focus = match state.focus {
                PaneFocus::Search => PaneFocus::List,
                PaneFocus::List => PaneFocus::Detail,
                PaneFocus::Detail => {
                    if state.chat_open {
                        PaneFocus::Extra
                    } else {
                        PaneFocus::Tabs
                    }
                }
                PaneFocus::Extra => PaneFocus::Tabs,
                PaneFocus::Tabs => PaneFocus::Search,
            };
        }
        CoreAction::FocusPrev => {
            state.focus = match state.focus {
                PaneFocus::Search => PaneFocus::Tabs,
                PaneFocus::List => PaneFocus::Search,
                PaneFocus::Detail => PaneFocus::List,
                PaneFocus::Extra => PaneFocus::Detail,
                PaneFocus::Tabs => {
                    if state.chat_open {
                        PaneFocus::Extra
                    } else {
                        PaneFocus::Detail
                    }
                }
            };
        }
        CoreAction::FocusSearch => state.focus = PaneFocus::Search,
        CoreAction::FocusList => state.focus = PaneFocus::List,
        CoreAction::FocusDetail => state.focus = PaneFocus::Detail,
        CoreAction::OpenSelected => state.focus = PaneFocus::Detail,
        CoreAction::Back => {
            state.focus = if state.focus == PaneFocus::Extra {
                PaneFocus::Detail
            } else {
                PaneFocus::List
            };
        }
        CoreAction::ToggleChat => {
            state.chat_open = !state.chat_open;
            if state.chat_open {
                state.focus = PaneFocus::Extra;
            } else if state.focus == PaneFocus::Extra {
                state.focus = PaneFocus::Detail;
            }
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
            let len = state.list_items.len();
            if len == 0 {
                state.selected_index = None;
            } else {
                let idx = state.selected_index.unwrap_or(0);
                state.selected_index = Some((idx + 1).min(len - 1));
            }
        }
        CoreAction::MovePrev => {
            let idx = state.selected_index.unwrap_or(0);
            state.selected_index = Some(idx.saturating_sub(1));
        }
        CoreAction::MoveHome => {
            state.selected_index = if state.list_items.is_empty() {
                None
            } else {
                Some(0)
            };
        }
        CoreAction::MoveEnd => {
            state.selected_index = if state.list_items.is_empty() {
                None
            } else {
                Some(state.list_items.len() - 1)
            };
        }
        CoreAction::MovePageDown => {
            let len = state.list_items.len();
            if len == 0 {
                state.selected_index = None;
            } else {
                let idx = state.selected_index.unwrap_or(0);
                state.selected_index = Some((idx + 10).min(len - 1));
            }
        }
        CoreAction::MovePageUp => {
            let idx = state.selected_index.unwrap_or(0);
            state.selected_index = Some(idx.saturating_sub(10));
        }
        _ => {}
    }
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
pub fn action_for_key(key: CoreKey, focus: PaneFocus) -> Option<CoreAction> {
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
                CoreKey::Enter | CoreKey::Down => Some(CoreAction::FocusList),
                CoreKey::Char(c) if !c.is_control() => Some(CoreAction::SearchInput(c)),
                _ => None,
            },
            PaneFocus::List => match key {
                CoreKey::Down | CoreKey::Char('j') => Some(CoreAction::MoveNext),
                CoreKey::Up | CoreKey::Char('k') => Some(CoreAction::MovePrev),
                CoreKey::PageDown => Some(CoreAction::MovePageDown),
                CoreKey::PageUp => Some(CoreAction::MovePageUp),
                CoreKey::Home | CoreKey::Char('g') => Some(CoreAction::MoveHome),
                CoreKey::End | CoreKey::Char('G') => Some(CoreAction::MoveEnd),
                CoreKey::Enter | CoreKey::Right | CoreKey::Char('l') => {
                    Some(CoreAction::OpenSelected)
                }
                _ => None,
            },
            PaneFocus::Tabs => match key {
                CoreKey::Up | CoreKey::Char('k') => {
                    Some(CoreAction::SelectPrevTab)
                }
                CoreKey::Down | CoreKey::Char('j') => {
                    Some(CoreAction::SelectNextTab)
                }
                CoreKey::Left | CoreKey::Char('h') => Some(CoreAction::FocusList),
                CoreKey::Right | CoreKey::Char('l') | CoreKey::Enter => {
                    Some(CoreAction::FocusDetail)
                }
                _ => None,
            },
            PaneFocus::Detail => match key {
                CoreKey::Left | CoreKey::Char('h') => Some(CoreAction::Back),
                CoreKey::Down | CoreKey::Char('j') => Some(CoreAction::MovePageDown),
                CoreKey::Up | CoreKey::Char('k') => Some(CoreAction::MovePageUp),
                CoreKey::PageDown => Some(CoreAction::MovePageDown),
                CoreKey::PageUp => Some(CoreAction::MovePageUp),
                CoreKey::Home | CoreKey::Char('g') => Some(CoreAction::MoveHome),
                _ => None,
            },
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
    state.selected_detail = snapshot.selected_detail;
    state.selected_context = snapshot.selected_context;
    state.total_count = snapshot.total_count;
    state.status_message = snapshot.status_message;

    if let Some(sel) = state.selected_index {
        if sel >= state.list_items.len() {
            state.selected_index = if state.list_items.is_empty() {
                None
            } else {
                Some(0)
            };
        }
    } else if !state.list_items.is_empty() {
        state.selected_index = Some(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyProvider;

    impl DataProvider for DummyProvider {
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

    #[test]
    fn test_apply_snapshot_sets_total_and_selection() {
        let mut state = CoreState::default();
        let snapshot = ProviderSnapshot {
            items: vec![UiItemSummary {
                id: "1".to_string(),
                name: "item".to_string(),
                kind: tui_kit_model::UiItemKind::Custom("x".to_string()),
                visibility: tui_kit_model::UiVisibility::Private,
                qualified_name: None,
                subtitle: None,
                tags: vec![],
            }],
            selected_detail: None,
            selected_context: None,
            total_count: 1,
            status_message: Some("ok".to_string()),
        };

        apply_snapshot(&mut state, snapshot);
        assert_eq!(state.total_count, 1);
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn test_dummy_provider_contract() {
        let mut p = DummyProvider;
        let init = p.initialize().unwrap();
        assert_eq!(init.total_count, 1);
    }

    #[test]
    fn test_apply_core_action_updates_selection() {
        let mut state = CoreState {
            list_items: vec![
                UiItemSummary {
                    id: "1".to_string(),
                    name: "a".to_string(),
                    kind: tui_kit_model::UiItemKind::Custom("x".to_string()),
                    visibility: tui_kit_model::UiVisibility::Private,
                    qualified_name: None,
                    subtitle: None,
                    tags: vec![],
                },
                UiItemSummary {
                    id: "2".to_string(),
                    name: "b".to_string(),
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
        let mut provider = DummyProvider;
        let mut state = CoreState::default();
        let effects = dispatch_action(&mut provider, &mut state, &CoreAction::FocusList).unwrap();
        assert!(effects.is_empty());
    }
}
