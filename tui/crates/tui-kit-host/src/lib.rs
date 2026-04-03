//! Host-side helpers for integrating terminal input/output with `tui-kit-runtime`.

pub mod picker;
pub mod runtime_loop;
pub mod settings;
pub mod terminal;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreKey, CoreState, CoreTabId, CreateCostState, CreateModalFocus,
    CreateSubmitState, PaneFocus, PickerContext, PickerState, action_for_key, apply_modal_error,
    close_access_control_modal, close_add_memory_modal, close_remove_memory_modal,
    close_rename_memory_modal, close_transfer_modal, open_access_control_modal,
    open_add_memory_modal, open_remove_memory_modal, open_rename_memory_modal,
    open_transfer_confirm, open_transfer_modal, tab_focus_policy,
};

/// Fallback tab ids used when host does not provide explicit tabs.
pub const DEFAULT_TAB_IDS: [&str; 4] = ["tab-1", "tab-2", "tab-3", "tab-4"];

/// Host-level normalized input event used by app loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostInputEvent {
    pub key_event: KeyEvent,
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

/// Poll and normalize crossterm events for host loops.
pub fn poll_host_input(timeout: Duration) -> std::io::Result<Option<HostInputEvent>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    Ok(normalize_host_input_event(event::read()?))
}

fn normalize_host_input_event(event: Event) -> Option<HostInputEvent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => Some(HostInputEvent {
            key_event: key,
            code: key.code,
            modifiers: key.modifiers,
        }),
        _ => None,
    }
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
    BackToTabs,
    BackFromFormToTabs,
    BackToMemoriesTab,
    OpenCreateTab,
    ToggleChat,
    ToggleHelp,
    ToggleSettings,
    SetDefaultFromSelection,
    OpenRenameMemory,
    BackFromContent,
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
        } else if current_tab_id == tui_kit_runtime::kinic_tabs::KINIC_MEMORIES_TAB_ID
            && focus == PaneFocus::Content
        {
            HostGlobalCommand::BackFromContent
        } else if focus == PaneFocus::Content
            || (current_tab_id == tui_kit_runtime::kinic_tabs::KINIC_MEMORIES_TAB_ID
                && matches!(focus, PaneFocus::Search | PaneFocus::Items)
                && query_is_empty)
        {
            HostGlobalCommand::BackToTabs
        } else if focus == PaneFocus::Tabs && !focus_policy.allows_search {
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
    if code == KeyCode::Char('D')
        && modifiers.contains(KeyModifiers::SHIFT)
        && current_tab_id == tui_kit_runtime::kinic_tabs::KINIC_MEMORIES_TAB_ID
        && matches!(focus, PaneFocus::Items | PaneFocus::Content)
    {
        return HostGlobalCommand::SetDefaultFromSelection;
    }
    if code == KeyCode::Char('R')
        && modifiers.contains(KeyModifiers::SHIFT)
        && current_tab_id == tui_kit_runtime::kinic_tabs::KINIC_MEMORIES_TAB_ID
        && matches!(focus, PaneFocus::Items | PaneFocus::Content)
    {
        return HostGlobalCommand::OpenRenameMemory;
    }
    if code == KeyCode::Char('n') && modifiers.contains(KeyModifiers::CONTROL) {
        return HostGlobalCommand::OpenCreateTab;
    }
    if code == KeyCode::Char('r') && modifiers.contains(KeyModifiers::CONTROL) {
        return HostGlobalCommand::RefreshCurrentView;
    }
    if code == KeyCode::Esc {
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
                state.persistent_status_message = None;
                state.status_message = Some(message);
            }
            CoreEffect::NotifyPersistent(message) => {
                state.persistent_status_message = Some(message.clone());
                state.status_message = Some(message);
            }
            CoreEffect::OpenExternal(url) => match open_external(&url) {
                Ok(()) => {
                    state.persistent_status_message = None;
                    state.status_message = Some(format!("Opened: {url}"));
                }
                Err(e) => {
                    state.persistent_status_message = None;
                    state.status_message = Some(format!("Failed to open URL: {url} ({e})"));
                }
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
            CoreEffect::InsertFormError(message) => {
                state.insert_submit_state = if message.is_some() {
                    CreateSubmitState::Error
                } else {
                    CreateSubmitState::Idle
                };
                state.insert_error = message.clone();
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
                    PaneFocus::Items => focus_policy.allows_items,
                    PaneFocus::Tabs => focus_policy.allows_tabs,
                    PaneFocus::Content => focus_policy.allows_content,
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
            CoreEffect::ResetInsertFormForRepeat => {
                state.insert_text.clear();
                state.insert_file_path_input.clear();
                state.insert_selected_file_path = None;
                state.insert_embedding.clear();
                state.insert_submit_state = CreateSubmitState::Idle;
                state.insert_spinner_frame = 0;
                state.insert_error = None;
            }
            CoreEffect::SetInsertMemoryId(memory_id) => {
                state.insert_memory_id = memory_id.clone();
                state.insert_memory_placeholder = None;
                if let PickerState::List {
                    context: PickerContext::InsertTarget,
                    selected_id,
                    ..
                } = &mut state.picker
                {
                    *selected_id = Some(memory_id);
                }
                state.insert_error = None;
            }
            CoreEffect::SetInsertTag(tag) => {
                state.insert_tag = tag.clone();
                state.insert_error = None;
            }
            CoreEffect::SetAccessListIndex(index) => {
                state.access_list_index = index;
            }
            CoreEffect::SetMemoryContentActionIndex(index) => {
                state.memory_content_action_index = index;
            }
            CoreEffect::OpenAccessAction {
                memory_id,
                principal_id,
                role,
            } => {
                open_access_control_modal(state);
                state.access_control.memory_id = memory_id;
                state.access_control.mode = tui_kit_runtime::AccessControlMode::Action;
                state.access_control.action = tui_kit_runtime::AccessControlAction::Change;
                state.access_control.role = match role {
                    tui_kit_runtime::AccessControlRole::Admin => {
                        tui_kit_runtime::AccessControlRole::Writer
                    }
                    tui_kit_runtime::AccessControlRole::Writer => {
                        tui_kit_runtime::AccessControlRole::Admin
                    }
                    tui_kit_runtime::AccessControlRole::Reader => {
                        tui_kit_runtime::AccessControlRole::Admin
                    }
                };
                state.access_control.current_role = role;
                state.access_control.principal_id = principal_id;
                state.access_control.focus = tui_kit_runtime::AccessControlFocus::Principal;
            }
            CoreEffect::OpenAccessConfirm {
                memory_id,
                principal_id,
                action,
                role,
            } => {
                open_access_control_modal(state);
                state.access_control.mode = tui_kit_runtime::AccessControlMode::Confirm;
                state.access_control.memory_id = memory_id;
                state.access_control.action = action;
                state.access_control.principal_id = principal_id;
                state.access_control.role = role;
                state.access_control.focus = tui_kit_runtime::AccessControlFocus::Principal;
            }
            CoreEffect::OpenAccessAdd { memory_id } => {
                open_access_control_modal(state);
                state.access_control.mode = tui_kit_runtime::AccessControlMode::Add;
                state.access_control.memory_id = memory_id;
                state.access_control.action = tui_kit_runtime::AccessControlAction::Add;
                state.access_control.role = tui_kit_runtime::AccessControlRole::Reader;
                state.access_control.current_role = tui_kit_runtime::AccessControlRole::Reader;
                state.access_control.principal_id.clear();
                state.access_control.focus = tui_kit_runtime::AccessControlFocus::Principal;
            }
            CoreEffect::CloseAccessControl => {
                close_access_control_modal(state);
            }
            CoreEffect::AccessFormError(message) => {
                apply_modal_error(
                    &mut state.access_control.submit_state,
                    &mut state.access_control.error,
                    message,
                );
            }
            CoreEffect::OpenTransferModal {
                fee_base_units,
                available_balance_base_units,
            } => {
                open_transfer_modal(state);
                state.transfer_modal.fee_base_units = Some(fee_base_units);
                state.transfer_modal.available_balance_base_units =
                    Some(available_balance_base_units);
            }
            CoreEffect::OpenTransferConfirm => {
                open_transfer_confirm(state);
            }
            CoreEffect::CloseTransferModal => {
                close_transfer_modal(state);
            }
            CoreEffect::TransferFormError(message) => {
                state.transfer_modal.prerequisites_loading = false;
                apply_modal_error(
                    &mut state.transfer_modal.submit_state,
                    &mut state.transfer_modal.error,
                    message,
                );
            }
            CoreEffect::OpenAddMemory => {
                open_add_memory_modal(state);
            }
            CoreEffect::CloseAddMemory => {
                close_add_memory_modal(state);
            }
            CoreEffect::AddMemoryFormError(message) => {
                apply_modal_error(
                    &mut state.add_memory.submit_state,
                    &mut state.add_memory.error,
                    message,
                );
            }
            CoreEffect::OpenRemoveMemory => {
                open_remove_memory_modal(state);
            }
            CoreEffect::CloseRemoveMemory => {
                close_remove_memory_modal(state);
            }
            CoreEffect::RemoveMemoryFormError(message) => {
                apply_modal_error(
                    &mut state.remove_memory.submit_state,
                    &mut state.remove_memory.error,
                    message,
                );
            }
            CoreEffect::OpenRenameMemory {
                memory_id,
                current_name,
            } => {
                open_rename_memory_modal(state);
                state.rename_memory.memory_id = memory_id;
                state.rename_memory.form.value = current_name;
            }
            CoreEffect::CloseRenameMemory => {
                close_rename_memory_modal(state);
            }
            CoreEffect::RenameFormError(message) => {
                apply_modal_error(
                    &mut state.rename_memory.form.submit_state,
                    &mut state.rename_memory.form.error,
                    message,
                );
            }
            CoreEffect::Custom { id, payload } => {
                state.persistent_status_message = None;
                state.status_message = Some(match payload {
                    Some(p) => format!("Custom effect: {id} ({p})"),
                    None => format!("Custom effect: {id}"),
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
