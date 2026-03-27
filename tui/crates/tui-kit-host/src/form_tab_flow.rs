//! Shared helpers for tab-scoped form screens such as Kinic Create and Insert.

use crossterm::event::KeyCode;
use tui_kit_runtime::{
    CoreAction, CoreState, CreateModalFocus, InsertFormFocus, PaneFocus, SelectorContext,
    kinic_tabs::{is_form_tab, is_kinic_create_tab, is_kinic_insert_tab},
};

pub fn form_tab_action_from_key(code: KeyCode, state: &mut CoreState) -> Option<CoreAction> {
    if !is_form_input_mode(state) {
        return None;
    }

    if is_kinic_create_tab(state.current_tab_id.as_str()) {
        return match code {
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
        };
    }

    match code {
        KeyCode::Tab => Some(CoreAction::InsertNextField),
        KeyCode::BackTab => Some(CoreAction::InsertPrevField),
        KeyCode::Backspace => Some(CoreAction::InsertBackspace),
        KeyCode::Enter => match state.insert_focus {
            InsertFormFocus::Mode => Some(CoreAction::InsertCycleMode),
            InsertFormFocus::Tag => Some(CoreAction::OpenSelector(SelectorContext::InsertTag)),
            InsertFormFocus::Submit => Some(CoreAction::InsertSubmit),
            _ => None,
        },
        KeyCode::Char(c) if !c.is_control() => match state.insert_focus {
            InsertFormFocus::MemoryId
            | InsertFormFocus::Text
            | InsertFormFocus::FilePath
            | InsertFormFocus::Embedding => {
                Some(CoreAction::InsertInput(c))
            }
            InsertFormFocus::Mode | InsertFormFocus::Submit | InsertFormFocus::Tag => None,
        },
        _ => None,
    }
}

pub fn is_form_input_mode(state: &CoreState) -> bool {
    is_form_tab(state.current_tab_id.as_str()) && state.focus == PaneFocus::Form
}

pub fn reset_form_focus(state: &mut CoreState) {
    if is_kinic_insert_tab(state.current_tab_id.as_str()) {
        state.insert_focus = InsertFormFocus::Mode;
        return;
    }
    state.create_focus = CreateModalFocus::Name;
}

pub fn reset_form_state_for_tab(state: &mut CoreState, tab_id: &str) {
    if is_kinic_insert_tab(tab_id) {
        state.insert_mode = tui_kit_runtime::InsertMode::Normal;
        state.insert_memory_id.clear();
        state.insert_tag.clear();
        state.insert_text.clear();
        state.insert_file_path.clear();
        state.insert_embedding.clear();
        state.insert_submit_state = tui_kit_runtime::CreateSubmitState::Idle;
        state.insert_spinner_frame = 0;
        state.insert_error = None;
        state.insert_focus = InsertFormFocus::Mode;
        return;
    }

    state.create_name.clear();
    state.create_description.clear();
    state.create_submit_state = tui_kit_runtime::CreateSubmitState::Idle;
    state.create_spinner_frame = 0;
    state.create_error = None;
    state.create_focus = CreateModalFocus::Name;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::{
        InsertMode,
        kinic_tabs::{KINIC_CREATE_TAB_ID, KINIC_INSERT_TAB_ID},
    };

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
            create_submit_state: tui_kit_runtime::CreateSubmitState::Submitting,
            create_error: Some("boom".to_string()),
            create_focus: CreateModalFocus::Submit,
            ..CoreState::default()
        };

        reset_form_state_for_tab(&mut state, KINIC_CREATE_TAB_ID);

        assert_eq!(state.create_name, "");
        assert_eq!(state.create_description, "");
        assert_eq!(
            state.create_submit_state,
            tui_kit_runtime::CreateSubmitState::Idle
        );
        assert_eq!(state.create_error, None);
        assert_eq!(state.create_focus, CreateModalFocus::Name);
    }

    #[test]
    fn insert_tag_field_uses_selector_instead_of_free_input() {
        let mut state = CoreState {
            current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            insert_focus: InsertFormFocus::Tag,
            ..CoreState::default()
        };

        let action = form_tab_action_from_key(KeyCode::Char('x'), &mut state);

        assert_eq!(action, None);
    }

    #[test]
    fn insert_tag_field_opens_selector_on_enter() {
        let mut state = CoreState {
            current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            insert_focus: InsertFormFocus::Tag,
            ..CoreState::default()
        };

        let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

        assert_eq!(
            action,
            Some(CoreAction::OpenSelector(SelectorContext::InsertTag))
        );
    }

    #[test]
    fn insert_mode_focus_uses_enter_to_cycle_modes() {
        let mut state = CoreState {
            current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            insert_focus: InsertFormFocus::Mode,
            ..CoreState::default()
        };

        let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

        assert_eq!(action, Some(CoreAction::InsertCycleMode));
    }

    #[test]
    fn reset_insert_form_state_clears_insert_fields() {
        let mut state = CoreState {
            insert_mode: InsertMode::Pdf,
            insert_memory_id: "aaaaa-aa".to_string(),
            insert_tag: "docs".to_string(),
            insert_file_path: "/tmp/doc.pdf".to_string(),
            insert_submit_state: tui_kit_runtime::CreateSubmitState::Submitting,
            insert_error: Some("boom".to_string()),
            insert_focus: InsertFormFocus::Submit,
            ..CoreState::default()
        };

        reset_form_state_for_tab(&mut state, KINIC_INSERT_TAB_ID);

        assert_eq!(state.insert_mode, InsertMode::Normal);
        assert_eq!(state.insert_memory_id, "");
        assert_eq!(state.insert_tag, "");
        assert_eq!(state.insert_file_path, "");
        assert_eq!(state.insert_error, None);
        assert_eq!(state.insert_focus, InsertFormFocus::Mode);
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

    #[test]
    fn submit_focus_uses_submit_action_on_enter() {
        let mut state = CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            create_focus: CreateModalFocus::Submit,
            ..CoreState::default()
        };

        let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

        assert_eq!(action, Some(CoreAction::CreateSubmit));
    }
}
