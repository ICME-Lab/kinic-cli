//! Host-side helpers for integrating terminal input/output with `tui-kit-runtime`.

pub mod settings;
pub mod runtime_loop;
pub mod terminal;

use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use tui_kit_runtime::{
    action_for_key, CoreAction, CoreEffect, CoreKey, CoreState, CoreTabId, PaneFocus,
};
use std::time::Duration;

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
pub fn action_from_keycode(code: KeyCode, focus: PaneFocus) -> Option<CoreAction> {
    key_to_core_key(code).and_then(|k| action_for_key(k, focus))
}

/// Resolve number-based tab action to concrete tab id using host configuration.
pub fn resolve_tab_action(action: CoreAction, tab_ids: &[&str]) -> Option<CoreAction> {
    resolve_tab_action_with_current(action, tab_ids, None)
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
            let prev = if current_idx == 0 { len - 1 } else { current_idx - 1 };
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
    CloseCreateModal,
    OpenCreateModal,
    ToggleTheme,
    ToggleChat,
    ToggleHelp,
    ToggleSettings,
    BackFromDetail,
    ClearQuery,
    Quit,
}

/// Shared decision helper for global and overlay key handling.
pub fn global_command_for_key(
    code: KeyCode,
    modifiers: KeyModifiers,
    focus: PaneFocus,
    show_help: bool,
    show_settings: bool,
    show_create_modal: bool,
    query_is_empty: bool,
) -> HostGlobalCommand {
    if show_help {
        return HostGlobalCommand::CloseHelp;
    }

    if show_create_modal {
        return match code {
            KeyCode::Esc => HostGlobalCommand::CloseCreateModal,
            _ => HostGlobalCommand::None,
        };
    }

    if show_settings {
        return match code {
            KeyCode::Char('t') if modifiers.is_empty() => HostGlobalCommand::ToggleTheme,
            KeyCode::Esc => HostGlobalCommand::CloseSettings,
            _ => HostGlobalCommand::None,
        };
    }

    if code == KeyCode::Char('q')
        && modifiers.is_empty()
        && focus != PaneFocus::Search
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::Quit;
    }
    if code == KeyCode::Char('?')
        && modifiers.is_empty()
        && focus != PaneFocus::Search
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::ToggleHelp;
    }
    if code == KeyCode::Char('t')
        && modifiers.is_empty()
        && focus != PaneFocus::Search
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::ToggleTheme;
    }
    if code == KeyCode::Char('C')
        && modifiers.contains(KeyModifiers::SHIFT)
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::ToggleChat;
    }
    if code == KeyCode::Char('S')
        && modifiers.contains(KeyModifiers::SHIFT)
        && focus != PaneFocus::Extra
    {
        return HostGlobalCommand::ToggleSettings;
    }
    if code == KeyCode::Char('n') && modifiers.is_empty() {
        return HostGlobalCommand::OpenCreateModal;
    }
    if code == KeyCode::Esc {
        if focus == PaneFocus::Detail {
            return HostGlobalCommand::BackFromDetail;
        }
        if !query_is_empty {
            return HostGlobalCommand::ClearQuery;
        }
        return HostGlobalCommand::Quit;
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
            CoreEffect::Custom { id, payload } => {
                match id.as_str() {
                    "create_modal_close" => {
                        state.create_modal_open = false;
                        state.create_name.clear();
                        state.create_description.clear();
                        state.create_submitting = false;
                        state.create_error = None;
                    }
                    "create_modal_error" => {
                        state.create_submitting = false;
                        state.create_error = payload.clone();
                    }
                    "select_first" => {
                        state.selected_index = if state.list_items.is_empty() { None } else { Some(0) };
                    }
                    _ => {
                        state.status_message = Some(match payload {
                            Some(p) => format!("Custom effect: {id} ({p})"),
                            None => format!("Custom effect: {id}"),
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::CoreState;

    #[test]
    fn key_map_includes_navigation() {
        assert_eq!(key_to_core_key(KeyCode::Tab), Some(CoreKey::Tab));
        assert_eq!(key_to_core_key(KeyCode::Down), Some(CoreKey::Down));
        assert_eq!(
            key_to_core_key(KeyCode::Char('a')),
            Some(CoreKey::Char('a'))
        );
    }

    #[test]
    fn action_from_keycode_maps_search_input() {
        assert_eq!(
            action_from_keycode(KeyCode::Char('x'), PaneFocus::Search),
            Some(CoreAction::SearchInput('x'))
        );
    }

    #[test]
    fn global_command_handles_escape_clear_query() {
        assert_eq!(
            global_command_for_key(
                KeyCode::Esc,
                KeyModifiers::NONE,
                PaneFocus::List,
                false,
                false,
                false,
            ),
            HostGlobalCommand::ClearQuery
        );
    }

    #[test]
    fn execute_effects_updates_status_message() {
        let mut state = CoreState::default();
        execute_effects_to_status(&mut state, vec![CoreEffect::Notify("hello".to_string())]);
        assert_eq!(state.status_message.as_deref(), Some("hello"));
    }

    #[test]
    fn global_command_quit_on_q() {
        assert_eq!(
            global_command_for_key(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                PaneFocus::List,
                false,
                false,
                true,
            ),
            HostGlobalCommand::Quit
        );
    }

    #[test]
    fn resolve_tab_action_maps_index_with_host_tabs() {
        let mapped = resolve_tab_action(CoreAction::SelectTabIndex(1), &["a", "b", "c"]);
        assert_eq!(mapped, Some(CoreAction::SetTab(CoreTabId::new("b"))));
    }

    #[test]
    fn resolve_tab_action_falls_back_to_default_ids() {
        let mapped = resolve_tab_action(CoreAction::SelectTabIndex(2), &[]);
        assert_eq!(mapped, Some(CoreAction::SetTab(CoreTabId::new("tab-3"))));
    }

    #[test]
    fn global_command_theme_toggle_on_t() {
        assert_eq!(
            global_command_for_key(
                KeyCode::Char('t'),
                KeyModifiers::NONE,
                PaneFocus::List,
                false,
                false,
                true,
            ),
            HostGlobalCommand::ToggleTheme
        );
    }
}
