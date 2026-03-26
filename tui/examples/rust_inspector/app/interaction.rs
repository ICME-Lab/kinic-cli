//! Input interaction reducer: key -> intent -> state transition (+ effects).
//!
//! Boundary policy for this module:
//! - Runtime-first (`tui-kit-runtime` + `tui-kit-host`):
//!   common navigation, focus movement, tab switching, query mutation.
//! - App-specific fallback (`UiIntent` in this file):
//!   completion popup interactions, crates-tab special flows, chat panel behavior,
//!   and app-specific shortcuts/effects.
//!
//! This keeps Oracle TUI behavior compatible while converging toward shared runtime contracts.

use crate::app::intent_domain::{
    apply_crates_qualified_enter, apply_escape_domain_fallback, apply_inspector_open_crates_io,
    apply_inspector_open_docs, apply_list_enter, apply_list_left, apply_list_open_crates_io,
    apply_list_open_docs,
};
use crate::app::{App, Tab};
use crate::ui::{AnimationState, Focus, TabId};
use crossterm::event::{KeyCode, KeyModifiers};
use tui_kit_host::{
    HostGlobalCommand, action_from_keycode, global_command_for_key, resolve_tab_action_with_current,
};
use tui_kit_runtime::{
    CoreAction, CoreState, CoreTabId, CreateCostState, CreateSubmitState, PaneFocus,
    apply_core_action,
};

/// Side effects triggered by reducer (executed by outer runtime).
#[derive(Debug, Clone)]
pub enum UiEffect {
    OpenUrl {
        url: String,
        success_message: Option<String>,
        failure_message: Option<String>,
    },
}

/// User intent derived from key input and current state.
#[derive(Debug, Clone)]
pub enum UiIntent {
    Quit,
    ToggleHelp,
    ToggleSettings,
    ToggleChat,
    OpenCreateModal,
    OpenGithub,
    OpenSponsor,
    EscapePressed,
    CompletionInput(char),
    CompletionBackspace,
    CompletionDown,
    CompletionUp,
    CompletionEnter,
    CompletionTab,
    CompletionBackTab,
    CratesQualifiedEnter,
    ListEnter,
    ListOpenDocs,
    ListOpenCratesIo,
    ListLeft,
    InspectorOpenDocs,
    InspectorOpenCratesIo,
    ChatEsc,
    ChatEnter,
    ChatBackspace,
    ChatChar(char),
    ChatDown,
    ChatUp,
    ChatPageDown,
    ChatPageUp,
    ChatHome,
    ChatEnd,
    ChatFocusChar(char),
    ChatFocusBackspace,
    ChatFocusEnter,
}

/// Convert key input into one or more intents based on current app state.
pub fn intents_for_key(app: &App, code: KeyCode, modifiers: KeyModifiers) -> Vec<UiIntent> {
    let mut out = Vec::new();
    let in_chat_panel = app.focus == Focus::Chat;

    // Chat panel has global scroll/input shortcuts while open.
    if app.chat_open {
        match code {
            KeyCode::PageDown => return vec![UiIntent::ChatPageDown],
            KeyCode::PageUp => return vec![UiIntent::ChatPageUp],
            KeyCode::Down => return vec![UiIntent::ChatDown],
            KeyCode::Up => return vec![UiIntent::ChatUp],
            KeyCode::Home => return vec![UiIntent::ChatHome],
            KeyCode::End => return vec![UiIntent::ChatEnd],
            KeyCode::Char(c) => {
                if !(modifiers == KeyModifiers::SHIFT && c == 'C') {
                    return vec![UiIntent::ChatFocusChar(c)];
                }
            }
            KeyCode::Backspace => return vec![UiIntent::ChatFocusBackspace],
            KeyCode::Enter if modifiers.is_empty() => return vec![UiIntent::ChatFocusEnter],
            _ => {}
        }
    }

    // Global shortcuts via shared host command helper.
    match global_command_for_key(
        code,
        modifiers,
        focus_to_pane(app.focus),
        app.current_tab.id().0.as_str(),
        app.show_help,
        app.show_settings,
        app.search_input.is_empty(),
    ) {
        HostGlobalCommand::None => {}
        HostGlobalCommand::CloseHelp | HostGlobalCommand::ToggleHelp => {
            if !in_chat_panel && app.focus != Focus::Search {
                return vec![UiIntent::ToggleHelp];
            }
        }
        HostGlobalCommand::CloseSettings | HostGlobalCommand::ToggleSettings => {
            if !in_chat_panel && app.focus != Focus::Search {
                return vec![UiIntent::ToggleSettings];
            }
        }
        HostGlobalCommand::ToggleChat => {
            if !in_chat_panel && app.focus != Focus::Search {
                return vec![UiIntent::ToggleChat];
            }
        }
        HostGlobalCommand::CloseChat => {
            if app.chat_open && !in_chat_panel && app.focus != Focus::Search {
                return vec![UiIntent::ToggleChat];
            }
        }
        HostGlobalCommand::BackFromFormToTabs => {
            return vec![UiIntent::EscapePressed];
        }
        HostGlobalCommand::OpenCreateTab => return vec![UiIntent::OpenCreateModal],
        HostGlobalCommand::BackToMemoriesTab
        | HostGlobalCommand::BackFromDetail
        | HostGlobalCommand::ClearQuery => {
            return vec![UiIntent::EscapePressed];
        }
        HostGlobalCommand::RefreshCurrentView => {}
        HostGlobalCommand::Quit => {
            if !in_chat_panel && app.focus != Focus::Search {
                return vec![UiIntent::Quit];
            }
        }
    }

    // App-specific globals not part of shared host command helper.
    match code {
        KeyCode::Char('g')
            if modifiers.is_empty() && !in_chat_panel && app.focus != Focus::Search =>
        {
            return vec![UiIntent::OpenGithub];
        }
        KeyCode::Char('C')
            if modifiers.contains(KeyModifiers::SHIFT)
                && !in_chat_panel
                && app.focus != Focus::Search =>
        {
            return vec![UiIntent::ToggleChat];
        }
        KeyCode::Char('s')
            if modifiers.is_empty() && !in_chat_panel && app.focus != Focus::Search =>
        {
            return vec![UiIntent::OpenSponsor];
        }
        KeyCode::Esc => return vec![UiIntent::EscapePressed],
        _ => {}
    }

    // Overlay handling.
    if app.show_settings {
        return out;
    }
    if app.show_help {
        // Any key closes help.
        return vec![UiIntent::ToggleHelp];
    }

    // Focus-local app-specific fallbacks.
    //
    // Runtime-first path handles common navigation/search/focus keys.
    // Here we keep only behaviors that are not yet represented in runtime:
    // - search completion interactions
    // - crates qualified-path enter behavior
    // - domain-specific shortcuts (docs/crates.io, crate-left, chat)
    match app.focus {
        Focus::Search => {
            if app.show_completion {
                match code {
                    KeyCode::Char(c) => out.push(UiIntent::CompletionInput(c)),
                    KeyCode::Backspace => out.push(UiIntent::CompletionBackspace),
                    KeyCode::Down => out.push(UiIntent::CompletionDown),
                    KeyCode::Up => out.push(UiIntent::CompletionUp),
                    KeyCode::Tab if modifiers.is_empty() => out.push(UiIntent::CompletionTab),
                    KeyCode::BackTab if modifiers.is_empty() => {
                        out.push(UiIntent::CompletionBackTab)
                    }
                    KeyCode::Enter => out.push(UiIntent::CompletionEnter),
                    _ => {}
                }
            } else if matches!(code, KeyCode::Enter)
                && app.current_tab == Tab::Crates
                && app.selected_installed_crate.is_some()
            {
                out.push(UiIntent::CratesQualifiedEnter);
            }
        }
        Focus::List => match code {
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => out.push(UiIntent::ListEnter),
            KeyCode::Char('o') if modifiers.is_empty() => out.push(UiIntent::ListOpenDocs),
            KeyCode::Char('c') if modifiers.is_empty() => out.push(UiIntent::ListOpenCratesIo),
            KeyCode::Left | KeyCode::Char('h') => out.push(UiIntent::ListLeft),
            _ => {}
        },
        Focus::Tabs => {}
        Focus::Form => {}
        Focus::Inspector => match code {
            KeyCode::Char('o') if modifiers.is_empty() => out.push(UiIntent::InspectorOpenDocs),
            KeyCode::Char('c') if modifiers.is_empty() => out.push(UiIntent::InspectorOpenCratesIo),
            _ => {}
        },
        Focus::Chat => match code {
            KeyCode::Esc => out.push(UiIntent::ChatEsc),
            KeyCode::Enter if modifiers.is_empty() => out.push(UiIntent::ChatEnter),
            KeyCode::Backspace if modifiers.is_empty() => out.push(UiIntent::ChatBackspace),
            KeyCode::Char(c) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
                out.push(UiIntent::ChatChar(c))
            }
            KeyCode::Down | KeyCode::Char('j') => out.push(UiIntent::ChatDown),
            KeyCode::Up | KeyCode::Char('k') => out.push(UiIntent::ChatUp),
            KeyCode::PageDown => out.push(UiIntent::ChatPageDown),
            KeyCode::PageUp => out.push(UiIntent::ChatPageUp),
            KeyCode::Home | KeyCode::Char('g') => out.push(UiIntent::ChatHome),
            KeyCode::End => out.push(UiIntent::ChatEnd),
            _ => {}
        },
    }
    out
}

fn focus_to_pane(focus: Focus) -> PaneFocus {
    match focus {
        Focus::Search => PaneFocus::Search,
        Focus::List => PaneFocus::List,
        Focus::Tabs => PaneFocus::Tabs,
        Focus::Form => PaneFocus::Form,
        Focus::Inspector => PaneFocus::Detail,
        Focus::Chat => PaneFocus::Extra,
    }
}

fn pane_to_focus(focus: PaneFocus) -> Focus {
    match focus {
        PaneFocus::Search => Focus::Search,
        PaneFocus::List => Focus::List,
        PaneFocus::Tabs => Focus::Tabs,
        PaneFocus::Form => Focus::Form,
        PaneFocus::Detail => Focus::Inspector,
        PaneFocus::Extra => Focus::Chat,
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeApplyResult {
    pub consumed: bool,
    pub tab_changed: bool,
}

/// Try runtime-first handling for common navigation/search actions.
///
/// Returns `consumed=false` when the key should fall back to the legacy
/// UiIntent pipeline (domain-specific or compatibility-sensitive behavior).
pub fn try_apply_runtime_action(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    inspector_scroll: &mut usize,
) -> RuntimeApplyResult {
    // Keep ctrl/alt combos on legacy path for now.
    if modifiers.contains(KeyModifiers::CONTROL) || modifiers.contains(KeyModifiers::ALT) {
        return RuntimeApplyResult::default();
    }

    if app.create_modal_open {
        let Some(action) = (match code {
            KeyCode::Tab => Some(CoreAction::CreateNextField),
            KeyCode::BackTab => Some(CoreAction::CreatePrevField),
            KeyCode::Backspace => Some(CoreAction::CreateBackspace),
            KeyCode::Enter => Some(CoreAction::CreateSubmit),
            KeyCode::Char(c) if !c.is_control() => Some(CoreAction::CreateInput(c)),
            _ => None,
        }) else {
            return RuntimeApplyResult::default();
        };

        let mut core = core_state_from_app(app);
        apply_core_action(&mut core, &action);
        if matches!(action, CoreAction::CreateSubmit) {
            app.create_modal_open = false;
            core.create_submit_state = CreateSubmitState::Idle;
            core.create_error = Some("Create action is not implemented in this example.".into());
            core.status_message =
                Some("Create action is not implemented in this example.".to_string());
        }
        apply_core_state_to_app(app, core);
        return RuntimeApplyResult {
            consumed: true,
            tab_changed: false,
        };
    }

    let Some(base_action) = action_from_keycode(
        code,
        focus_to_pane(app.focus),
        app.current_tab.id().0.as_str(),
    ) else {
        return RuntimeApplyResult::default();
    };
    let tab_ids = tab_id_refs(app);
    let Some(action) = resolve_tab_action_with_current(
        base_action,
        &tab_ids,
        Some(app.current_tab.id().0.as_str()),
    ) else {
        return RuntimeApplyResult::default();
    };

    // Preserve completion behavior on search focus until completion is modeled in runtime.
    if app.focus == Focus::Search {
        if app.show_completion
            && matches!(
                action,
                CoreAction::SearchInput(_)
                    | CoreAction::SearchBackspace
                    | CoreAction::FocusNext
                    | CoreAction::FocusPrev
                    | CoreAction::FocusList
            )
        {
            return RuntimeApplyResult::default();
        }

        // Keep qualified-path search behavior for crates on app-specific fallback path.
        if matches!(code, KeyCode::Enter)
            && app.current_tab == Tab::Crates
            && app.selected_installed_crate.is_some()
        {
            return RuntimeApplyResult::default();
        }
    }

    // Inspector scroll is local UI state; apply it here and consume the key.
    if app.focus == Focus::Inspector
        && matches!(
            action,
            CoreAction::MovePageDown | CoreAction::MovePageUp | CoreAction::MoveHome
        )
    {
        match action {
            CoreAction::MovePageDown => *inspector_scroll = inspector_scroll.saturating_add(10),
            CoreAction::MovePageUp => *inspector_scroll = inspector_scroll.saturating_sub(10),
            CoreAction::MoveHome => *inspector_scroll = 0,
            _ => {}
        }
        return RuntimeApplyResult {
            consumed: true,
            tab_changed: false,
        };
    }

    // Preserve crates-tab special open behavior in legacy reducer.
    if app.current_tab == Tab::Crates
        && app.selected_installed_crate.is_none()
        && matches!(action, CoreAction::OpenSelected)
    {
        return RuntimeApplyResult::default();
    }

    let mut core = core_state_from_app(app);
    let prev_tab = app.current_tab;
    apply_core_action(&mut core, &action);
    apply_core_state_to_app(app, core.clone());

    let mut tab_changed = false;
    let tab_id = TabId::new(core.current_tab_id.clone());
    if let Some(tab) = Tab::from_id(&tab_id) {
        if tab != app.current_tab {
            app.current_tab = tab;
            tab_changed = true;
            if app.current_tab == Tab::Crates && app.installed_crates_list.is_empty() {
                let _ = app.scan_installed_crates();
            }
        }
    }

    match action {
        CoreAction::SetQuery(_)
        | CoreAction::SearchInput(_)
        | CoreAction::SearchBackspace
        | CoreAction::SetTab(_) => {
            app.filter_items();
            if app.focus == Focus::Search {
                app.show_completion = app.search_input.len() >= 2
                    && !(app.current_tab == Tab::Crates && app.selected_installed_crate.is_some());
            }
        }
        _ => {
            app.list_state.select(core.selected_index);
            app.refresh_ui_cache();
        }
    }

    RuntimeApplyResult {
        consumed: prev_tab != app.current_tab
            || !matches!(action, CoreAction::Custom(_) | CoreAction::OpenExternal(_)),
        tab_changed,
    }
}

/// Apply runtime tab switch directly (used by mouse tab click path).
pub fn apply_runtime_set_tab(app: &mut App, tab_id: TabId) -> RuntimeApplyResult {
    let mut core = core_state_from_app(app);
    apply_core_action(&mut core, &CoreAction::SetTab(CoreTabId::new(tab_id.0)));
    apply_core_state_to_app(app, core.clone());

    let mut tab_changed = false;
    let tab_id = TabId::new(core.current_tab_id.clone());
    if let Some(tab) = Tab::from_id(&tab_id) {
        if tab != app.current_tab {
            app.current_tab = tab;
            tab_changed = true;
            if app.current_tab == Tab::Crates && app.installed_crates_list.is_empty() {
                let _ = app.scan_installed_crates();
            }
        }
    }
    app.filter_items();

    RuntimeApplyResult {
        consumed: true,
        tab_changed,
    }
}

/// Apply one intent and mutate app state. Returns optional side effects.
pub fn apply_intent(
    app: &mut App,
    intent: UiIntent,
    _inspector_scroll: &mut usize,
    _animation: &mut AnimationState,
) -> Vec<UiEffect> {
    let mut effects = Vec::new();
    match intent {
        UiIntent::Quit => app.should_quit = true,
        UiIntent::ToggleHelp => app.toggle_help(),
        UiIntent::ToggleSettings => app.toggle_settings(),
        UiIntent::OpenGithub => effects.push(UiEffect::OpenUrl {
            url: "https://github.com/yashksaini-coder/oracle".to_string(),
            success_message: None,
            failure_message: None,
        }),
        UiIntent::OpenSponsor => effects.push(UiEffect::OpenUrl {
            url: "https://github.com/sponsors/yashksaini-coder".to_string(),
            success_message: None,
            failure_message: None,
        }),
        UiIntent::ToggleChat => {
            if app.selected_item().is_some() {
                app.toggle_chat_panel();
            } else {
                app.status_message = "Select an item in the list to ask Chat about it".into();
            }
        }
        UiIntent::OpenCreateModal => {
            app.create_modal_open = true;
            app.create_name.clear();
            app.create_description.clear();
            app.create_submitting = false;
            app.create_error = None;
            app.create_focus = tui_kit_runtime::CreateModalFocus::Name;
        }
        UiIntent::EscapePressed => {
            if app.show_settings {
                app.toggle_settings();
            } else if app.show_help {
                app.show_help = false;
            } else if app.show_completion {
                app.show_completion = false;
            } else if app.focus == Focus::Chat {
                app.toggle_chat_panel();
            } else if apply_escape_domain_fallback(app) {
            } else {
                app.should_quit = true;
            }
        }
        UiIntent::CompletionInput(c) => app.on_char(c),
        UiIntent::CompletionBackspace => app.on_backspace(),
        UiIntent::CompletionDown => {
            if app.show_completion {
                app.next_completion();
            } else {
                app.focus = Focus::List;
            }
        }
        UiIntent::CompletionUp => {
            if app.show_completion {
                app.prev_completion();
            }
        }
        UiIntent::CompletionEnter => {
            if app.show_completion {
                app.select_completion();
            } else {
                if app.current_tab == Tab::Crates && app.selected_installed_crate.is_some() {
                    app.search_qualified_path();
                }
                app.filter_items();
                app.focus = Focus::List;
            }
        }
        UiIntent::CompletionTab => {
            if app.show_completion {
                app.select_completion();
            }
            app.next_focus();
        }
        UiIntent::CompletionBackTab => app.prev_focus(),
        UiIntent::CratesQualifiedEnter => {
            apply_crates_qualified_enter(app);
        }
        UiIntent::ListEnter => apply_list_enter(app),
        UiIntent::ListOpenDocs => apply_list_open_docs(app, &mut effects),
        UiIntent::InspectorOpenDocs => apply_inspector_open_docs(app, &mut effects),
        UiIntent::ListOpenCratesIo => apply_list_open_crates_io(app, &mut effects),
        UiIntent::InspectorOpenCratesIo => apply_inspector_open_crates_io(app, &mut effects),
        UiIntent::ListLeft => apply_list_left(app),
        UiIntent::ChatEsc => app.toggle_chat_panel(),
        UiIntent::ChatEnter | UiIntent::ChatFocusEnter => app.submit_chat_message(),
        UiIntent::ChatBackspace | UiIntent::ChatFocusBackspace => {
            app.chat_input.pop();
        }
        UiIntent::ChatChar(c) => {
            app.chat_input.push(c);
        }
        UiIntent::ChatFocusChar(c) => {
            app.focus = Focus::Chat;
            app.chat_input.push(c);
        }
        UiIntent::ChatDown => app.chat_scroll = app.chat_scroll.saturating_add(1),
        UiIntent::ChatUp => app.chat_scroll = app.chat_scroll.saturating_sub(1),
        UiIntent::ChatPageDown => app.chat_scroll = app.chat_scroll.saturating_add(10),
        UiIntent::ChatPageUp => app.chat_scroll = app.chat_scroll.saturating_sub(10),
        UiIntent::ChatHome => app.chat_scroll = 0,
        UiIntent::ChatEnd => app.chat_scroll = app.chat_scroll.saturating_add(9999),
    }
    effects
}

fn core_state_from_app(app: &App) -> CoreState {
    CoreState {
        current_tab_id: app.current_tab.id().0,
        focus: focus_to_pane(app.focus),
        query: app.search_input.clone(),
        selected_index: app.list_state.selected(),
        list_items: app.ui_summaries.clone(),
        selected_detail: app.ui_selected_detail.clone(),
        selected_context: app.ui_dependency_node.clone(),
        total_count: app.ui_total_count,
        status_message: Some(app.status_message.clone()),
        chat_open: app.chat_open,
        chat_messages: app.chat_messages.clone(),
        chat_input: app.chat_input.clone(),
        chat_loading: app.chat_loading,
        chat_scroll: app.chat_scroll,
        create_name: app.create_name.clone(),
        create_description: app.create_description.clone(),
        create_submit_state: if app.create_submitting {
            CreateSubmitState::Submitting
        } else {
            CreateSubmitState::Idle
        },
        create_spinner_frame: 0,
        create_error: app.create_error.clone(),
        create_focus: app.create_focus,
        create_cost_state: CreateCostState::Hidden,
    }
}

fn apply_core_state_to_app(app: &mut App, core: CoreState) {
    app.search_input = core.query;
    app.focus = pane_to_focus(core.focus);
    app.list_state.select(core.selected_index);
    app.chat_open = core.chat_open;
    app.chat_messages = core.chat_messages;
    app.chat_input = core.chat_input;
    app.chat_loading = core.chat_loading;
    app.chat_scroll = core.chat_scroll;
    app.create_name = core.create_name;
    app.create_description = core.create_description;
    app.create_submitting = core.create_submit_state == CreateSubmitState::Submitting;
    app.create_error = core.create_error;
    app.create_focus = core.create_focus;
    if let Some(status_message) = core.status_message {
        app.status_message = status_message;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn ctrl_n_opens_create_modal() {
        let app = App::default();
        let intents = intents_for_key(&app, KeyCode::Char('n'), KeyModifiers::CONTROL);
        assert!(matches!(intents.as_slice(), [UiIntent::OpenCreateModal]));
    }

    #[test]
    fn create_submit_sets_example_status_and_closes_modal() {
        let mut app = App::default();
        app.create_modal_open = true;
        let mut inspector_scroll = 0;

        let result = try_apply_runtime_action(
            &mut app,
            KeyCode::Enter,
            KeyModifiers::NONE,
            &mut inspector_scroll,
        );

        assert!(result.consumed);
        assert!(!app.create_modal_open);
        assert_eq!(
            app.status_message,
            "Create action is not implemented in this example."
        );
        assert_eq!(
            app.create_error.as_deref(),
            Some("Create action is not implemented in this example.")
        );
    }
}

fn tab_id_refs(app: &App) -> Vec<&str> {
    app.ui_config.tabs.iter().map(|t| t.id.0.as_str()).collect()
}
