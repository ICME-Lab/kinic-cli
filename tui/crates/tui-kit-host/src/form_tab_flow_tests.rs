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
        create_submit_state: tui_kit_runtime::CreateSubmitState::Submitting,
        create_error: Some("boom".to_string()),
        create_focus: CreateModalFocus::Submit,
        ..CoreState::default()
    };

    reset_create_form_state(&mut state);

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
