use super::*;
use tui_kit_runtime::kinic_tabs::{
    KINIC_CREATE_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID,
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
fn open_form_tab_keeps_form_state_when_dispatch_fails() {
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

    assert_eq!(state.focus, PaneFocus::Tabs);
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
fn switch_to_tab_normalizes_focus_only_on_success() {
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
