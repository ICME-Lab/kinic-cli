//! Shared helpers for tab-scoped form screens such as Kinic Create.

use crossterm::event::KeyCode;
use tui_kit_runtime::{
    CoreAction, CoreState, CreateModalFocus, PaneFocus, kinic_tabs::is_form_tab,
};

/// Route create-form key input while preserving normal text entry semantics.
///
/// The dedicated create form owns `Tab` and `Shift+Tab` so field order stays
/// predictable: `Name -> Description -> Submit -> Name`.
pub fn form_tab_action_from_key(code: KeyCode, state: &mut CoreState) -> Option<CoreAction> {
    if !is_create_form_input_mode(state) {
        return None;
    }

    match code {
        KeyCode::Tab => Some(CoreAction::CreateNextField),
        KeyCode::BackTab => Some(CoreAction::CreatePrevField),
        KeyCode::Backspace => Some(CoreAction::CreateBackspace),
        KeyCode::Enter => Some(CoreAction::CreateSubmit),
        KeyCode::Char(c) if !c.is_control() => match state.create_focus {
            CreateModalFocus::Name | CreateModalFocus::Description => {
                Some(CoreAction::CreateInput(c))
            }
            CreateModalFocus::Submit => None,
        },
        _ => None,
    }
}

/// Whether the current state should treat typed characters as create-form input.
pub fn is_create_form_input_mode(state: &CoreState) -> bool {
    is_form_tab(state.current_tab_id.as_str()) && state.focus == PaneFocus::Form
}

/// Reset field focus back to the name field when entering the create form.
pub fn reset_create_focus(state: &mut CoreState) {
    state.create_focus = CreateModalFocus::Name;
}

/// Clear stale create-tab form state before opening a fresh create flow.
pub fn reset_create_form_state(state: &mut CoreState) {
    state.create_name.clear();
    state.create_description.clear();
    state.create_submit_state = tui_kit_runtime::CreateSubmitState::Idle;
    state.create_spinner_frame = 0;
    state.create_error = None;
    reset_create_focus(state);
}

#[cfg(test)]
#[path = "form_tab_flow_tests.rs"]
mod tests;
