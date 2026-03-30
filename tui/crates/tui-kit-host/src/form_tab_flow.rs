//! Shared helpers for tab-scoped form screens such as Kinic Create and Insert.

use crossterm::event::KeyCode;
use tui_kit_runtime::{
    CoreAction, CoreState, CreateModalFocus, InsertFormFocus, PaneFocus,
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
        KeyCode::Left => match state.insert_focus {
            InsertFormFocus::Mode => Some(CoreAction::InsertCycleModePrev),
            _ => None,
        },
        KeyCode::Right => match state.insert_focus {
            InsertFormFocus::Mode => Some(CoreAction::InsertCycleMode),
            _ => None,
        },
        KeyCode::Enter => match state.insert_focus {
            InsertFormFocus::Mode => Some(CoreAction::InsertCycleMode),
            InsertFormFocus::MemoryId => Some(CoreAction::OpenDefaultMemoryPicker),
            InsertFormFocus::Submit => Some(CoreAction::InsertSubmit),
            _ => None,
        },
        KeyCode::Char(c) if !c.is_control() => match state.insert_focus {
            InsertFormFocus::Tag
            | InsertFormFocus::Text
            | InsertFormFocus::FilePath
            | InsertFormFocus::Embedding => Some(CoreAction::InsertInput(c)),
            InsertFormFocus::Mode | InsertFormFocus::MemoryId | InsertFormFocus::Submit => None,
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
#[path = "form_tab_flow_tests.rs"]
mod tests;
