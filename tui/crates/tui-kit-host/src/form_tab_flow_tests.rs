use super::*;
use tui_kit_runtime::{
    CoreAction, CreateModalFocus, InsertFormFocus, InsertMode, PickerContext,
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
fn reset_create_form_state_preserves_insert_transient_state() {
    let mut state = CoreState {
        insert_submit_state: tui_kit_runtime::CreateSubmitState::Submitting,
        insert_spinner_frame: 3,
        insert_error: Some("stale".to_string()),
        ..CoreState::default()
    };

    reset_form_state_for_tab(&mut state, KINIC_CREATE_TAB_ID);

    assert_eq!(
        state.insert_submit_state,
        tui_kit_runtime::CreateSubmitState::Submitting
    );
    assert_eq!(state.insert_spinner_frame, 3);
    assert_eq!(state.insert_error.as_deref(), Some("stale"));
}

#[test]
fn insert_form_routes_chars_into_active_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Text,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Char('x'), &mut state);

    assert_eq!(action, Some(CoreAction::InsertInput('x')));
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
        Some(CoreAction::OpenPicker(PickerContext::InsertTag))
    );
}

#[test]
fn insert_mode_focus_uses_enter_to_move_to_next_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Mode,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

    assert_eq!(action, Some(CoreAction::InsertNextField));
}

#[test]
fn insert_memory_id_focus_uses_enter_to_open_picker() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::MemoryId,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

    assert_eq!(
        action,
        Some(CoreAction::OpenPicker(PickerContext::InsertTarget))
    );
}

#[test]
fn insert_mode_focus_uses_left_to_move_to_prev_mode() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Mode,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Left, &mut state);

    assert_eq!(action, Some(CoreAction::InsertPrevMode));
}

#[test]
fn insert_mode_focus_uses_right_to_move_to_next_mode() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Mode,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Right, &mut state);

    assert_eq!(action, Some(CoreAction::InsertNextMode));
}

#[test]
fn insert_form_down_moves_to_next_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Tag,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Down, &mut state);

    assert_eq!(action, Some(CoreAction::InsertNextField));
}

#[test]
fn create_form_down_moves_to_next_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        create_focus: CreateModalFocus::Name,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Down, &mut state);

    assert_eq!(action, Some(CoreAction::CreateNextField));
}

#[test]
fn insert_submit_focus_uses_enter_to_submit() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Submit,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

    assert_eq!(action, Some(CoreAction::InsertSubmit));
}

#[test]
fn insert_file_path_focus_uses_enter_to_open_file_dialog() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::FilePath,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

    assert_eq!(action, Some(CoreAction::InsertOpenFileDialog));
}

#[test]
fn reset_insert_form_state_clears_insert_fields() {
    let mut state = CoreState {
        saved_default_memory_id: Some("bbbbb-bb".to_string()),
        insert_mode: InsertMode::File,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_file_path_input: "/tmp/doc.pdf".to_string(),
        insert_selected_file_path: Some(std::path::PathBuf::from("/tmp/doc.pdf")),
        insert_submit_state: tui_kit_runtime::CreateSubmitState::Submitting,
        insert_error: Some("boom".to_string()),
        insert_focus: InsertFormFocus::Submit,
        ..CoreState::default()
    };

    reset_form_state_for_tab(&mut state, KINIC_INSERT_TAB_ID);

    assert_eq!(state.insert_mode, InsertMode::File);
    assert_eq!(state.insert_memory_id, "bbbbb-bb");
    assert_eq!(state.insert_tag, "");
    assert_eq!(state.insert_file_path_input, "");
    assert_eq!(state.insert_selected_file_path, None);
    assert_eq!(state.insert_error, None);
    assert_eq!(state.insert_focus, InsertFormFocus::Mode);
}

#[test]
fn reset_insert_form_state_preserves_create_transient_state() {
    let mut state = CoreState {
        create_submit_state: tui_kit_runtime::CreateSubmitState::Submitting,
        create_spinner_frame: 5,
        create_error: Some("stale".to_string()),
        ..CoreState::default()
    };

    reset_form_state_for_tab(&mut state, KINIC_INSERT_TAB_ID);

    assert_eq!(
        state.create_submit_state,
        tui_kit_runtime::CreateSubmitState::Submitting
    );
    assert_eq!(state.create_spinner_frame, 5);
    assert_eq!(state.create_error.as_deref(), Some("stale"));
}

#[test]
fn submit_tab_wraps_back_to_name() {
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
fn name_backtab_wraps_to_submit() {
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

#[test]
fn create_name_focus_uses_enter_to_move_to_next_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        create_focus: CreateModalFocus::Name,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

    assert_eq!(action, Some(CoreAction::CreateNextField));
}

#[test]
fn insert_tag_focus_uses_enter_to_move_to_next_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Tag,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

    assert_eq!(action, Some(CoreAction::InsertNextField));
}
