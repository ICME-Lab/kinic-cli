//! Host-side helpers for integrating terminal input/output with `tui-kit-runtime`.

pub mod runtime_loop;
pub mod settings;
pub mod terminal;

use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use std::time::Duration;
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreKey, CoreState, CoreTabId, CreateCostState, CreateModalFocus,
    CreateSubmitState, PaneFocus, action_for_key, tab_focus_policy,
};

/// Fallback tab ids used when host does not provide explicit tabs.
pub const DEFAULT_TAB_IDS: [&str; 4] = ["tab-1", "tab-2", "tab-3", "tab-4"];

/// Host-level normalized input event used by app loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostInputEvent {
    KeyPress {
        code: KeyCode,
        modifiers: KeyModifiers,
    },
    MouseLeftDown {
        column: u16,
        row: u16,
    },
}

/// Poll and normalize crossterm events for host loops.
pub fn poll_host_input(timeout: Duration) -> std::io::Result<Option<HostInputEvent>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    let ev = match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Some(HostInputEvent::KeyPress {
            code: key.code,
            modifiers: key.modifiers,
        }),
        Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) => {
            Some(HostInputEvent::MouseLeftDown {
                column: mouse.column,
                row: mouse.row,
            })
        }
        _ => None,
    };
    Ok(ev)
}

/// Convert crossterm key codes into runtime-agnostic core keys.
pub fn key_to_core_key(code: KeyCode) -> Option<CoreKey> {
    match code {
        KeyCode::Char('/') => Some(CoreKey::Slash),
        KeyCode::Char(c) => Some(CoreKey::Char(c)),
        KeyCode::Tab => Some(CoreKey::Tab),
        KeyCode::BackTab => Some(CoreKey::BackTab),
        KeyCode::Backspace => Some(CoreKey::Backspace),
        KeyCode::Enter => Some(CoreKey::Enter),
        KeyCode::Down => Some(CoreKey::Down),
        KeyCode::Up => Some(CoreKey::Up),
        KeyCode::Left => Some(CoreKey::Left),
        KeyCode::Right => Some(CoreKey::Right),
        KeyCode::PageDown => Some(CoreKey::PageDown),
        KeyCode::PageUp => Some(CoreKey::PageUp),
        KeyCode::Home => Some(CoreKey::Home),
        KeyCode::End => Some(CoreKey::End),
        _ => None,
    }
}

/// Convert crossterm key code + runtime focus directly into core action.
pub fn action_from_keycode(
    code: KeyCode,
    focus: PaneFocus,
    current_tab_id: &str,
) -> Option<CoreAction> {
    key_to_core_key(code).and_then(|k| action_for_key(k, focus, current_tab_id))
}

/// Resolve number-based tab action to concrete tab id using host configuration.
pub fn resolve_tab_action(action: CoreAction, tab_ids: &[&str]) -> Option<CoreAction> {
    let resolved = if tab_ids.is_empty() {
        &DEFAULT_TAB_IDS[..]
    } else {
        tab_ids
    };
    resolve_tab_action_with_current(action, resolved, None)
}

/// Resolve tab-related actions using host tab ids and optional current tab id.
pub fn resolve_tab_action_with_current(
    action: CoreAction,
    tab_ids: &[&str],
    current_tab_id: Option<&str>,
) -> Option<CoreAction> {
    if tab_ids.is_empty() {
        return match action {
            CoreAction::SelectTabIndex(_)
            | CoreAction::SelectNextTab
            | CoreAction::SelectPrevTab => None,
            other => Some(other),
        };
    }
    let resolved = tab_ids;
    match action {
        CoreAction::SelectTabIndex(index) => resolved
            .get(index)
            .map(|id| CoreAction::SetTab(CoreTabId::new(*id))),
        CoreAction::SelectNextTab => {
            let current_idx = current_tab_id
                .and_then(|cur| resolved.iter().position(|id| *id == cur))
                .unwrap_or(0);
            let next = (current_idx + 1) % resolved.len().max(1);
            resolved
                .get(next)
                .map(|id| CoreAction::SetTab(CoreTabId::new(*id)))
        }
        CoreAction::SelectPrevTab => {
            let current_idx = current_tab_id
                .and_then(|cur| resolved.iter().position(|id| *id == cur))
                .unwrap_or(0);
            let len = resolved.len().max(1);
            let prev = if current_idx == 0 {
                len - 1
            } else {
                current_idx - 1
            };
            resolved
                .get(prev)
                .map(|id| CoreAction::SetTab(CoreTabId::new(*id)))
        }
        other => Some(other),
    }
}

/// Standard host-level command decisions for overlays/global shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostGlobalCommand {
    None,
    CloseHelp,
    CloseSettings,
    CloseChat,
    BackFromFormToTabs,
    BackToMemoriesTab,
    OpenCreateTab,
    ToggleChat,
    ToggleHelp,
    ToggleSettings,
    BackFromDetail,
    RefreshCurrentView,
    ClearQuery,
    Quit,
}

/// Shared decision helper for global and overlay key handling.
pub fn global_command_for_key(
    code: KeyCode,
    modifiers: KeyModifiers,
    focus: PaneFocus,
    current_tab_id: &str,
    show_help: bool,
    show_settings: bool,
    query_is_empty: bool,
) -> HostGlobalCommand {
    let focus_policy = tab_focus_policy(current_tab_id);

    if show_help {
        return HostGlobalCommand::CloseHelp;
    }

    if show_settings {
        return match code {
            KeyCode::Esc => HostGlobalCommand::CloseSettings,
            _ => HostGlobalCommand::None,
        };
    }

    if code == KeyCode::Esc && focus == PaneFocus::Extra {
        return HostGlobalCommand::CloseChat;
    }

    if code == KeyCode::Esc {
        let tab_specific = if focus == PaneFocus::Form && focus_policy.allows_form {
            HostGlobalCommand::BackFromFormToTabs
        } else if focus == PaneFocus::Tabs && !focus_policy.allows_search {
            HostGlobalCommand::BackToMemoriesTab
        } else if focus == PaneFocus::Detail && !focus_policy.allows_list {
            HostGlobalCommand::BackToMemoriesTab
        } else {
            HostGlobalCommand::None
        };
        if tab_specific != HostGlobalCommand::None {
            return tab_specific;
        }
    }

    if code == KeyCode::Char('q')
        && modifiers.is_empty()
        && focus != PaneFocus::Search
        && focus != PaneFocus::Form
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::Quit;
    }
    if code == KeyCode::Char('?')
        && modifiers.is_empty()
        && focus != PaneFocus::Search
        && focus != PaneFocus::Form
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::ToggleHelp;
    }
    if code == KeyCode::Char('C')
        && modifiers.contains(KeyModifiers::SHIFT)
        && focus != PaneFocus::Form
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::ToggleChat;
    }
    if code == KeyCode::Char('S')
        && modifiers.contains(KeyModifiers::SHIFT)
        && focus != PaneFocus::Form
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::ToggleSettings;
    }
    if code == KeyCode::Char('n') && modifiers.contains(KeyModifiers::CONTROL) {
        return HostGlobalCommand::OpenCreateTab;
    }
    if code == KeyCode::F(5) && modifiers.is_empty() {
        return HostGlobalCommand::RefreshCurrentView;
    }
    if code == KeyCode::Esc {
        if focus == PaneFocus::Detail {
            return HostGlobalCommand::BackFromDetail;
        }
        if !query_is_empty {
            return HostGlobalCommand::ClearQuery;
        }
        return HostGlobalCommand::None;
    }
    HostGlobalCommand::None
}

/// Execute OpenExternal effect via default browser.
pub fn open_external(url: &str) -> Result<(), String> {
    webbrowser::open(url).map(|_| ()).map_err(|e| e.to_string())
}

/// Default effect executor that reflects effects into `CoreState.status_message`.
///
/// Real hosts can replace this with richer integrations (webbrowser open, logging,
/// notifications, refresh scheduling, etc.).
pub fn execute_effects_to_status(state: &mut CoreState, effects: Vec<CoreEffect>) {
    for effect in effects {
        match effect {
            CoreEffect::Notify(message) => {
                state.status_message = Some(message);
            }
            CoreEffect::OpenExternal(url) => match open_external(&url) {
                Ok(()) => state.status_message = Some(format!("Opened: {url}")),
                Err(e) => state.status_message = Some(format!("Failed to open URL: {url} ({e})")),
            },
            CoreEffect::RequestRefresh => {}
            CoreEffect::CreateFormError(message) => {
                state.create_submit_state = if message.is_some() {
                    CreateSubmitState::Error
                } else {
                    CreateSubmitState::Idle
                };
                if !matches!(state.create_cost_state, CreateCostState::Unavailable) {
                    state.create_cost_state = match &state.create_cost_state {
                        CreateCostState::Loading => CreateCostState::Hidden,
                        other => other.clone(),
                    };
                }
                state.create_error = message.clone();
            }
            CoreEffect::SelectFirstListItem => {
                state.selected_index = if state.list_items.is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
            CoreEffect::FocusPane(pane) => {
                let focus_policy = tab_focus_policy(state.current_tab_id.as_str());
                let allows_focus = match pane {
                    PaneFocus::Search => focus_policy.allows_search,
                    PaneFocus::List => focus_policy.allows_list,
                    PaneFocus::Tabs => focus_policy.allows_tabs,
                    PaneFocus::Detail => focus_policy.allows_detail,
                    PaneFocus::Form => focus_policy.allows_form,
                    PaneFocus::Extra => focus_policy.allows_chat,
                };
                if allows_focus {
                    state.focus = pane;
                }
            }
            CoreEffect::ResetCreateFormAndSetTab { tab_id } => {
                state.current_tab_id = tab_id.clone();
                state.create_name.clear();
                state.create_description.clear();
                state.create_submit_state = CreateSubmitState::Idle;
                state.create_spinner_frame = 0;
                state.create_error = None;
                state.create_focus = CreateModalFocus::Name;
            }
            CoreEffect::Custom { id, payload } => {
                state.status_message = Some(match payload {
                    Some(p) => format!("Custom effect: {id} ({p})"),
                    None => format!("Custom effect: {id}"),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::CoreState;
    use tui_kit_runtime::kinic_tabs::{
        KINIC_CREATE_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID, KINIC_SETTINGS_TAB_ID,
    };

    mod key_mapping {
        use super::*;

        #[test]
        fn action_from_keycode_maps_search_input() {
            assert_eq!(
                action_from_keycode(KeyCode::Char('x'), PaneFocus::Search, KINIC_MEMORIES_TAB_ID),
                Some(CoreAction::SearchInput('x'))
            );
        }
    }

    mod tab_resolution {
        use super::*;

        #[test]
        fn resolve_tab_action_uses_host_and_default_ids() {
            let host_mapped = resolve_tab_action(CoreAction::SelectTabIndex(1), &["a", "b", "c"]);
            let default_mapped = resolve_tab_action(CoreAction::SelectTabIndex(2), &[]);

            assert_eq!(host_mapped, Some(CoreAction::SetTab(CoreTabId::new("b"))));
            assert_eq!(
                default_mapped,
                Some(CoreAction::SetTab(CoreTabId::new("tab-3")))
            );
        }
    }

    mod effect_application {
        use super::*;

        #[test]
        fn execute_effects_updates_status_message() {
            let mut state = CoreState::default();
            execute_effects_to_status(&mut state, vec![CoreEffect::Notify("hello".to_string())]);
            assert_eq!(state.status_message.as_deref(), Some("hello"));
        }

        #[test]
        fn execute_effects_sets_memories_tab() {
            let mut state = CoreState {
                current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
                create_name: "stale".to_string(),
                create_description: "stale".to_string(),
                create_submit_state: CreateSubmitState::Submitting,
                create_error: Some("boom".to_string()),
                ..CoreState::default()
            };

            execute_effects_to_status(
                &mut state,
                vec![CoreEffect::ResetCreateFormAndSetTab {
                    tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                }],
            );

            assert_eq!(state.current_tab_id, KINIC_MEMORIES_TAB_ID);
            assert_eq!(state.create_name, "");
            assert_eq!(state.create_description, "");
            assert_eq!(state.create_submit_state, CreateSubmitState::Idle);
            assert_eq!(state.create_error, None);
            assert_eq!(state.create_focus, CreateModalFocus::Name);
        }

        #[test]
        fn focus_pane_applies_visible_pane_on_memories_tab() {
            let mut state = CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                focus: PaneFocus::Search,
                ..CoreState::default()
            };

            execute_effects_to_status(&mut state, vec![CoreEffect::FocusPane(PaneFocus::List)]);

            assert_eq!(state.focus, PaneFocus::List);
        }

        #[test]
        fn focus_pane_ignores_hidden_pane_off_memories_tab() {
            let mut state = CoreState {
                current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
                focus: PaneFocus::Form,
                ..CoreState::default()
            };

            execute_effects_to_status(&mut state, vec![CoreEffect::FocusPane(PaneFocus::List)]);

            assert_eq!(state.focus, PaneFocus::Form);
        }
    }

    mod global_commands {
        use super::*;

        #[test]
        fn global_command_handles_primary_shortcuts() {
            let cases = [
                (
                    KeyCode::Esc,
                    KeyModifiers::NONE,
                    PaneFocus::List,
                    KINIC_MEMORIES_TAB_ID,
                    false,
                    HostGlobalCommand::ClearQuery,
                ),
                (
                    KeyCode::Char('n'),
                    KeyModifiers::CONTROL,
                    PaneFocus::List,
                    KINIC_MEMORIES_TAB_ID,
                    true,
                    HostGlobalCommand::OpenCreateTab,
                ),
                (
                    KeyCode::F(5),
                    KeyModifiers::NONE,
                    PaneFocus::List,
                    KINIC_MEMORIES_TAB_ID,
                    true,
                    HostGlobalCommand::RefreshCurrentView,
                ),
                (
                    KeyCode::Char('q'),
                    KeyModifiers::NONE,
                    PaneFocus::List,
                    KINIC_MEMORIES_TAB_ID,
                    true,
                    HostGlobalCommand::Quit,
                ),
                (
                    KeyCode::Esc,
                    KeyModifiers::NONE,
                    PaneFocus::Extra,
                    KINIC_MEMORIES_TAB_ID,
                    true,
                    HostGlobalCommand::CloseChat,
                ),
                (
                    KeyCode::Esc,
                    KeyModifiers::NONE,
                    PaneFocus::List,
                    KINIC_MEMORIES_TAB_ID,
                    true,
                    HostGlobalCommand::None,
                ),
            ];

            for (code, modifiers, focus, tab_id, query_is_empty, expected) in cases {
                assert_eq!(
                    global_command_for_key(
                        code,
                        modifiers,
                        focus,
                        tab_id,
                        false,
                        false,
                        query_is_empty,
                    ),
                    expected
                );
            }
        }

        #[test]
        fn global_command_resolves_escape_for_special_tabs() {
            let cases = [
                (
                    PaneFocus::Tabs,
                    KINIC_CREATE_TAB_ID,
                    HostGlobalCommand::BackToMemoriesTab,
                ),
                (
                    PaneFocus::Form,
                    KINIC_CREATE_TAB_ID,
                    HostGlobalCommand::BackFromFormToTabs,
                ),
                (
                    PaneFocus::Detail,
                    KINIC_MARKET_TAB_ID,
                    HostGlobalCommand::BackToMemoriesTab,
                ),
                (
                    PaneFocus::Detail,
                    KINIC_SETTINGS_TAB_ID,
                    HostGlobalCommand::BackToMemoriesTab,
                ),
            ];

            for (focus, tab_id, expected) in cases {
                assert_eq!(
                    global_command_for_key(
                        KeyCode::Esc,
                        KeyModifiers::NONE,
                        focus,
                        tab_id,
                        false,
                        false,
                        true,
                    ),
                    expected
                );
            }
        }

        #[test]
        fn global_command_leaves_q_unhandled_in_create_focus() {
            assert_eq!(
                global_command_for_key(
                    KeyCode::Char('q'),
                    KeyModifiers::NONE,
                    PaneFocus::Form,
                    KINIC_CREATE_TAB_ID,
                    false,
                    false,
                    true,
                ),
                HostGlobalCommand::None
            );
        }
    }
}
