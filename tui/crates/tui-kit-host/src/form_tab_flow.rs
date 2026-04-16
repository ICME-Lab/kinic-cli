//! Shared helpers for tab-scoped form screens such as Kinic Create and Insert.

use crossterm::event::KeyCode;
use tui_kit_runtime::{
    CoreState, FormFocus, FormResetKind, PaneFocus, apply_form_focus, current_form_focus,
    form_char_input_command, form_command_to_action, form_descriptor, form_enter_command,
    form_horizontal_change_command, kinic_tabs::is_form_tab,
};

pub fn form_tab_action_from_key(
    code: KeyCode,
    state: &mut CoreState,
) -> Option<tui_kit_runtime::CoreAction> {
    if !is_form_input_mode(state) {
        return None;
    }

    let descriptor = form_descriptor(state.current_tab_id.as_str())?;
    let focus = current_form_focus(state)?;
    let command = match code {
        KeyCode::Tab | KeyCode::Down => Some(descriptor.next_command()),
        KeyCode::BackTab | KeyCode::Up => Some(descriptor.prev_command()),
        KeyCode::Backspace => Some(descriptor.backspace_command()),
        KeyCode::Left => form_horizontal_change_command(focus, false),
        KeyCode::Right => form_horizontal_change_command(focus, true),
        KeyCode::Enter => form_enter_command(state),
        KeyCode::Char(c) if !c.is_control() => form_char_input_command(state, c),
        _ => None,
    }?;

    form_command_to_action(descriptor.kind, command)
}

pub fn is_form_input_mode(state: &CoreState) -> bool {
    is_form_tab(state.current_tab_id.as_str()) && state.focus == PaneFocus::Form
}

pub fn reset_form_focus(state: &mut CoreState) {
    let Some(descriptor) = form_descriptor(state.current_tab_id.as_str()) else {
        return;
    };
    apply_form_focus(state, descriptor.first_focus);
}

pub fn reset_form_state_for_tab(state: &mut CoreState, tab_id: &str) {
    let Some(descriptor) = form_descriptor(tab_id) else {
        return;
    };

    match descriptor.reset_kind {
        FormResetKind::Create => {
            reset_create_fields(state);
            reset_create_common_state(state, descriptor.first_focus);
        }
        FormResetKind::Insert => {
            reset_insert_fields(state);
            reset_insert_common_state(state, descriptor.first_focus);
        }
    }
}

fn reset_create_fields(state: &mut CoreState) {
    state.create_name.clear();
    state.create_description.clear();
}

fn reset_insert_fields(state: &mut CoreState) {
    state.insert_mode = tui_kit_runtime::InsertMode::default();
    state.insert_tag.clear();
    state.insert_text.clear();
    state.insert_file_path_input.clear();
    state.insert_selected_file_path = None;
    state.insert_embedding.clear();
}

fn reset_create_common_state(state: &mut CoreState, first_focus: FormFocus) {
    state.create_submit_state = tui_kit_runtime::CreateSubmitState::Idle;
    state.create_spinner_frame = 0;
    state.create_error = None;
    apply_form_focus(state, first_focus);
}

fn reset_insert_common_state(state: &mut CoreState, first_focus: FormFocus) {
    state.insert_submit_state = tui_kit_runtime::CreateSubmitState::Idle;
    state.insert_spinner_frame = 0;
    state.insert_error = None;
    apply_form_focus(state, first_focus);
}

#[cfg(test)]
#[path = "form_tab_flow_tests.rs"]
mod tests;
