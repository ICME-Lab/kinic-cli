//! Shared helpers for tab-scoped form screens such as Kinic Create.

use crossterm::event::KeyCode;
use tui_kit_runtime::{
    CoreAction, CoreState, CreateModalFocus, PaneFocus, kinic_tabs::is_form_tab,
};

/// Route key input to create-tab actions while preserving normal text entry.
///
/// The dedicated create form owns `Tab` and `Shift+Tab` so field order stays
/// predictable: `Name -> Description -> Submit -> Name`.
pub fn form_tab_action_from_key(code: KeyCode, state: &mut CoreState) -> Option<CoreAction> {
    if is_form_tab(state.current_tab_id.as_str()) && state.focus == PaneFocus::Tabs {
        return match code {
            // Create uses a full-body form, so "open content" enters the form instead of
            // borrowing the memories inspector focus model.
            KeyCode::Tab | KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                reset_create_focus(state);
                state.focus = PaneFocus::Form;
                None
            }
            KeyCode::Left | KeyCode::Char('h') => None,
            _ => None,
        };
    }

    if !is_create_form_input_mode(state) {
        return None;
    }

    match code {
        KeyCode::Tab => Some(CoreAction::CreateNextField),
        KeyCode::BackTab => Some(CoreAction::CreatePrevField),
        KeyCode::Backspace => Some(CoreAction::CreateBackspace),
        KeyCode::Enter => Some(CoreAction::CreateSubmit),
        KeyCode::Char(c) if !c.is_control() => Some(CoreAction::CreateInput(c)),
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
    state.create_submitting = false;
    state.create_error = None;
    reset_create_focus(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::kinic_tabs::KINIC_CREATE_TAB_ID;

    #[test]
    fn create_form_uses_t_as_input() {
        let mut state = CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            create_focus: CreateModalFocus::Name,
            ..CoreState::default()
        };

        let action = form_tab_action_from_key(KeyCode::Char('t'), &mut state);

        assert_eq!(action, Some(CoreAction::CreateInput('t')));
    }

    #[test]
    fn reset_create_form_state_resets_focus_to_name() {
        let mut state = CoreState {
            create_name: "stale".to_string(),
            create_description: "stale".to_string(),
            create_submitting: true,
            create_error: Some("boom".to_string()),
            create_focus: CreateModalFocus::Submit,
            ..CoreState::default()
        };

        reset_create_form_state(&mut state);

        assert_eq!(state.create_name, "");
        assert_eq!(state.create_description, "");
        assert!(!state.create_submitting);
        assert_eq!(state.create_error, None);
        assert_eq!(state.create_focus, CreateModalFocus::Name);
    }

    #[test]
    fn card_entry_from_tabs_restarts_at_name() {
        let mut state = CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Tabs,
            create_focus: CreateModalFocus::Submit,
            ..CoreState::default()
        };

        let action = form_tab_action_from_key(KeyCode::Tab, &mut state);

        assert_eq!(action, None);
        assert_eq!(state.focus, PaneFocus::Form);
        assert_eq!(state.create_focus, CreateModalFocus::Name);
    }

    #[test]
    fn submit_tab_cycles_back_to_name() {
        let mut state = CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            create_focus: CreateModalFocus::Submit,
            ..CoreState::default()
        };

        let action = form_tab_action_from_key(KeyCode::Tab, &mut state);

        assert_eq!(action, Some(CoreAction::CreateNextField));
        assert_eq!(state.create_focus, CreateModalFocus::Submit);
        assert_eq!(state.focus, PaneFocus::Form);
    }

    #[test]
    fn name_backtab_cycles_to_submit() {
        let mut state = CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            create_focus: CreateModalFocus::Name,
            ..CoreState::default()
        };

        let action = form_tab_action_from_key(KeyCode::BackTab, &mut state);

        assert_eq!(action, Some(CoreAction::CreatePrevField));
        assert_eq!(state.create_focus, CreateModalFocus::Name);
        assert_eq!(state.focus, PaneFocus::Form);
    }
}
