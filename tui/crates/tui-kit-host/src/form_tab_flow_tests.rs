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
fn insert_form_routes_chars_into_active_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Tag,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Char('x'), &mut state);

    assert_eq!(action, Some(CoreAction::InsertInput('x')));
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
fn insert_memory_id_focus_uses_enter_to_open_picker() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::MemoryId,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Enter, &mut state);

    assert_eq!(action, Some(CoreAction::OpenDefaultMemoryPicker));
}

#[test]
fn insert_mode_focus_uses_left_to_cycle_modes_backwards() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Mode,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Left, &mut state);

    assert_eq!(action, Some(CoreAction::InsertCycleModePrev));
}

#[test]
fn insert_mode_focus_uses_right_to_cycle_modes_forwards() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_focus: InsertFormFocus::Mode,
        ..CoreState::default()
    };

    let action = form_tab_action_from_key(KeyCode::Right, &mut state);

    assert_eq!(action, Some(CoreAction::InsertCycleMode));
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

    assert_eq!(state.insert_mode, InsertMode::Markdown);
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
