use super::*;
use std::path::PathBuf;
use tui_kit_runtime::kinic_tabs::{
    KINIC_CREATE_TAB_ID, KINIC_INSERT_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID,
};
use tui_kit_runtime::{CoreError, CoreResult, ProviderOutput, ProviderSnapshot};

struct TestProvider {
    result: CoreResult<ProviderOutput>,
}

impl TestProvider {
    fn err(message: &str) -> Self {
        Self {
            result: Err(CoreError::new(message)),
        }
    }
}

impl DataProvider for TestProvider {
    fn initialize(&mut self) -> CoreResult<ProviderSnapshot> {
        Ok(ProviderSnapshot::default())
    }

    fn handle_action(
        &mut self,
        _action: &CoreAction,
        _state: &CoreState,
    ) -> CoreResult<ProviderOutput> {
        self.result.clone()
    }
}

#[test]
fn normalize_focus_keeps_memories_on_tabs_after_tab_switch() {
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Items,
        ..CoreState::default()
    };

    normalize_focus_after_set_tab(&mut state);

    assert_eq!(state.focus, PaneFocus::Tabs);
}

#[test]
fn normalize_focus_resets_create_tab_to_tabs_and_name_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        create_focus: tui_kit_runtime::CreateModalFocus::Submit,
        ..CoreState::default()
    };

    normalize_focus_after_set_tab(&mut state);

    assert_eq!(state.focus, PaneFocus::Tabs);
    assert_eq!(state.create_focus, tui_kit_runtime::CreateModalFocus::Name);
}

#[test]
fn normalize_focus_resets_insert_tab_to_tabs_and_mode_field() {
    let mut state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        insert_focus: tui_kit_runtime::InsertFormFocus::Submit,
        ..CoreState::default()
    };

    normalize_focus_after_set_tab(&mut state);

    assert_eq!(state.focus, PaneFocus::Tabs);
    assert_eq!(state.insert_focus, tui_kit_runtime::InsertFormFocus::Mode);
}

#[test]
fn normalize_focus_keeps_placeholder_tabs_on_tabs() {
    let mut state = CoreState {
        current_tab_id: KINIC_MARKET_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        ..CoreState::default()
    };

    normalize_focus_after_set_tab(&mut state);

    assert_eq!(state.focus, PaneFocus::Tabs);
}

#[test]
fn selector_overlay_action_maps_arrow_keys_only() {
    assert_eq!(
        selector_overlay_action(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE
        ),
        Some(CoreAction::MoveDefaultMemoryPickerNext)
    );
    assert_eq!(
        selector_overlay_action(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::NONE
        ),
        Some(CoreAction::MoveDefaultMemoryPickerPrev)
    );
    assert_eq!(
        selector_overlay_action(
            crossterm::event::KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE
        ),
        None
    );
}

#[test]
fn handle_overlay_input_returns_dispatch_error_when_selector_action_fails() {
    let mut provider = TestProvider::err("selector failed");
    let mut state = CoreState {
        default_memory_selector_open: true,
        ..CoreState::default()
    };

    let result = handle_overlay_input(
        &mut provider,
        &mut state,
        false,
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );

    match result {
        OverlayInputResult::DispatchError(message) => {
            assert_eq!(message, "Dispatch error: selector failed");
        }
        _ => panic!("expected overlay dispatch error"),
    }
}

#[test]
fn handle_overlay_input_closes_settings_without_provider_dispatch() {
    let mut provider = TestProvider::err("should not run");
    let mut state = CoreState::default();

    let result = handle_overlay_input(
        &mut provider,
        &mut state,
        true,
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    );

    assert!(matches!(result, OverlayInputResult::CloseSettings));
}

#[test]
fn handle_overlay_input_consumes_unknown_selector_keys() {
    let mut provider = TestProvider::err("should not run");
    let mut state = CoreState {
        default_memory_selector_open: true,
        ..CoreState::default()
    };

    let result = handle_overlay_input(
        &mut provider,
        &mut state,
        false,
        crossterm::event::KeyCode::Char('x'),
        crossterm::event::KeyModifiers::NONE,
    );

    assert!(matches!(result, OverlayInputResult::Consumed));
}

#[test]
fn open_insert_tab_failure_keeps_insert_form_state_and_focus() {
    let mut provider = TestProvider::err("tab failed");
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        insert_mode: tui_kit_runtime::InsertMode::File,
        insert_memory_id: "aaaaa-aa".into(),
        insert_tag: "docs".into(),
        insert_file_path: "/tmp/doc.pdf".into(),
        insert_focus: tui_kit_runtime::InsertFormFocus::Submit,
        status_message: Some("ready".into()),
        ..CoreState::default()
    };

    open_form_tab(
        &mut provider,
        &mut state,
        &mut hooks,
        KINIC_INSERT_TAB_ID,
        true,
    );

    assert_eq!(state.focus, PaneFocus::Content);
    assert_eq!(state.insert_mode, tui_kit_runtime::InsertMode::File);
    assert_eq!(state.insert_memory_id, "aaaaa-aa");
    assert_eq!(state.insert_tag, "docs");
    assert_eq!(state.insert_file_path, "/tmp/doc.pdf");
    assert_eq!(state.insert_focus, tui_kit_runtime::InsertFormFocus::Submit);
    assert_eq!(
        state.status_message.as_deref(),
        Some("Dispatch error: tab failed")
    );
}

#[test]
fn open_form_tab_failure_keeps_form_state_and_existing_focus() {
    let mut provider = TestProvider::err("tab failed");
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        create_name: "draft".into(),
        create_description: "notes".into(),
        create_focus: tui_kit_runtime::CreateModalFocus::Submit,
        status_message: Some("ready".into()),
        ..CoreState::default()
    };

    open_form_tab(
        &mut provider,
        &mut state,
        &mut hooks,
        KINIC_CREATE_TAB_ID,
        true,
    );

    assert_eq!(state.focus, PaneFocus::Content);
    assert_eq!(state.create_name, "draft");
    assert_eq!(state.create_description, "notes");
    assert_eq!(
        state.create_focus,
        tui_kit_runtime::CreateModalFocus::Submit
    );
    assert_eq!(
        state.status_message.as_deref(),
        Some("Dispatch error: tab failed")
    );
}

#[test]
fn switch_to_tab_failure_keeps_existing_focus_when_target_tab_allows_it() {
    let mut provider = TestProvider::err("tab failed");
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MARKET_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        ..CoreState::default()
    };

    let result = switch_to_tab(&mut provider, &mut state, &mut hooks, KINIC_MEMORIES_TAB_ID);

    assert_eq!(result, Err("Dispatch error: tab failed".into()));
    assert_eq!(state.focus, PaneFocus::Content);
}

#[test]
fn dispatch_with_effects_returns_error_message_on_failure() {
    let mut provider = TestProvider::err("settings failed");
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState::default();

    let result = dispatch_with_effects(
        &mut provider,
        &mut state,
        &mut hooks,
        &CoreAction::ToggleSettings,
    );

    assert_eq!(result, Err("Dispatch error: settings failed".into()));
}

#[test]
fn dispatch_with_effects_keeps_non_tab_reducer_state_on_failure() {
    let mut provider = TestProvider::err("chat failed");
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState::default();

    let result = dispatch_with_effects(
        &mut provider,
        &mut state,
        &mut hooks,
        &CoreAction::ToggleChat,
    );

    assert_eq!(result, Err("Dispatch error: chat failed".into()));
    assert!(state.chat_open);
    assert_eq!(state.focus, PaneFocus::Extra);
}

#[test]
fn content_scroll_helper_handles_scroll_end_only() {
    let mut inspector_scroll = 3usize;

    assert!(apply_content_scroll_action(
        &CoreAction::ScrollContentEnd,
        &mut inspector_scroll
    ));
    assert_eq!(inspector_scroll, 10002);

    assert!(!apply_content_scroll_action(
        &CoreAction::MoveEnd,
        &mut inspector_scroll
    ));
    assert_eq!(inspector_scroll, 10002);
}

#[test]
fn dispatch_action_with_persistent_clear_clears_for_edit_actions() {
    let mut provider = TestProvider {
        result: Ok(ProviderOutput::default()),
    };
    let mut state = CoreState {
        persistent_status_message: Some("done".into()),
        status_message: Some("done".into()),
        insert_focus: tui_kit_runtime::InsertFormFocus::Text,
        ..CoreState::default()
    };

    let effects = dispatch_action_with_persistent_clear(
        &mut provider,
        &mut state,
        &CoreAction::InsertInput('x'),
    )
    .expect("dispatch should succeed");

    assert!(effects.is_empty());
    assert_eq!(state.persistent_status_message, None);
    assert_eq!(state.status_message, None);
}

#[test]
fn dispatch_action_with_persistent_clear_keeps_for_navigation_actions() {
    let mut provider = TestProvider {
        result: Ok(ProviderOutput::default()),
    };
    let mut state = CoreState {
        persistent_status_message: Some("done".into()),
        status_message: Some("done".into()),
        ..CoreState::default()
    };

    let effects =
        dispatch_action_with_persistent_clear(&mut provider, &mut state, &CoreAction::MoveNext)
            .expect("dispatch should succeed");

    assert!(effects.is_empty());
    assert_eq!(state.persistent_status_message.as_deref(), Some("done"));
    assert_eq!(state.status_message.as_deref(), Some("done"));
}

#[test]
fn apply_insert_file_dialog_selection_updates_file_path_and_clears_insert_error() {
    let mut state = CoreState {
        insert_file_path: "stale".into(),
        insert_error: Some("bad path".into()),
        insert_submit_state: tui_kit_runtime::CreateSubmitState::Error,
        ..CoreState::default()
    };

    apply_insert_file_dialog_selection(&mut state, Some(PathBuf::from("/tmp/doc.pdf")));

    assert_eq!(state.insert_file_path, "/tmp/doc.pdf");
    assert_eq!(state.insert_error, None);
    assert_eq!(
        state.insert_submit_state,
        tui_kit_runtime::CreateSubmitState::Idle
    );
    assert_eq!(
        state.status_message.as_deref(),
        Some("Selected file: /tmp/doc.pdf")
    );
}

#[test]
fn apply_insert_file_dialog_selection_keeps_existing_path_on_cancel() {
    let mut state = CoreState {
        insert_file_path: "/tmp/existing.md".into(),
        ..CoreState::default()
    };

    apply_insert_file_dialog_selection(&mut state, None);

    assert_eq!(state.insert_file_path, "/tmp/existing.md");
    assert_eq!(
        state.status_message.as_deref(),
        Some("File selection canceled.")
    );
}
