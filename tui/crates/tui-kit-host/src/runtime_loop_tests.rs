use super::*;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::path::PathBuf;
use tui_kit_render::ui::UiConfig;
use tui_kit_runtime::kinic_tabs::{
    KINIC_CREATE_TAB_ID, KINIC_INSERT_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID,
};
use tui_kit_runtime::{
    CoreError, CoreResult, InsertFormFocus, InsertMode, PaneFocus, PickerContext, PickerListMode,
    PickerState, ProviderOutput, ProviderSnapshot, RenameMemoryModalState, TextInputModalState,
};

struct TestProvider {
    result: CoreResult<ProviderOutput>,
}

impl TestProvider {
    fn err(message: &str) -> Self {
        Self {
            result: Err(CoreError::new(message)),
        }
    }

    fn ok() -> Self {
        Self {
            result: Ok(ProviderOutput::default()),
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

fn test_ui_config() -> UiConfig {
    UiConfig::default()
}

fn test_runtime_config() -> RuntimeLoopConfig {
    RuntimeLoopConfig {
        initial_tab_id: KINIC_INSERT_TAB_ID,
        tab_ids: &[
            KINIC_MEMORIES_TAB_ID,
            KINIC_CREATE_TAB_ID,
            KINIC_INSERT_TAB_ID,
            KINIC_MARKET_TAB_ID,
        ],
        initial_focus: PaneFocus::Form,
        ui_config: test_ui_config,
        file_picker: None,
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
fn picker_overlay_action_maps_generic_picker_keys() {
    assert_eq!(
        picker_overlay_action(
            &PickerState::List {
                context: PickerContext::DefaultMemory,
                items: Vec::new(),
                selected_index: 0,
                selected_id: None,
                mode: PickerListMode::Browsing,
            },
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE
        ),
        Some(CoreAction::MovePickerNext)
    );
    assert_eq!(
        picker_overlay_action(
            &PickerState::List {
                context: PickerContext::TagManagement,
                items: Vec::new(),
                selected_index: 0,
                selected_id: None,
                mode: PickerListMode::Browsing,
            },
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE
        ),
        Some(CoreAction::DeleteSelectedPickerItem)
    );
    assert_eq!(
        picker_overlay_action(
            &PickerState::Input {
                context: PickerContext::AddTag,
                origin_context: Some(PickerContext::InsertTag),
                value: String::new(),
            },
            crossterm::event::KeyCode::Backspace,
            crossterm::event::KeyModifiers::NONE
        ),
        Some(CoreAction::PickerBackspace)
    );
}

#[test]
fn build_ui_renders_rename_overlay_contents() {
    let theme = Theme::default();
    let cfg = test_runtime_config();
    let state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Content,
        rename_memory: RenameMemoryModalState {
            form: TextInputModalState {
                open: true,
                value: "Alpha Memory".to_string(),
                ..TextInputModalState::default()
            },
            focus: tui_kit_runtime::RenameModalFocus::Name,
            ..RenameMemoryModalState::default()
        },
        ..CoreState::default()
    };
    let textareas = FormTextareas::default();
    let animation = AnimationState::new();
    let ui = build_ui(
        &theme, &cfg, &state, &textareas, 0, 0, false, false, &animation,
    );
    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    Widget::render(ui, area, &mut buf);
    let rendered = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("Rename Memory"));
    assert!(rendered.contains("Alpha Memory"));
}

#[test]
fn handle_overlay_input_returns_dispatch_error_when_selector_action_fails() {
    let mut provider = TestProvider::err("selector failed");
    let mut state = CoreState {
        picker: PickerState::List {
            context: PickerContext::DefaultMemory,
            items: Vec::new(),
            selected_index: 0,
            selected_id: None,
            mode: PickerListMode::Browsing,
        },
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
        picker: PickerState::List {
            context: PickerContext::DefaultMemory,
            items: Vec::new(),
            selected_index: 0,
            selected_id: None,
            mode: PickerListMode::Browsing,
        },
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
fn textarea_input_updates_create_description_with_newlines() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        create_focus: tui_kit_runtime::CreateModalFocus::Description,
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("textarea input");
    assert!(handled);

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("textarea input");
    assert!(handled);

    assert_eq!(state.create_description, "a\n");
}

#[test]
fn textarea_up_on_first_create_description_row_moves_to_previous_field() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        create_focus: tui_kit_runtime::CreateModalFocus::Description,
        create_description: "first\nsecond".into(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("textarea input");

    assert!(handled);
    assert_eq!(state.create_focus, tui_kit_runtime::CreateModalFocus::Name);
    assert_eq!(state.create_description, "first\nsecond");
}

#[test]
fn textarea_down_on_last_create_description_row_moves_to_submit() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        create_focus: tui_kit_runtime::CreateModalFocus::Description,
        create_description: "first\nsecond".into(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);
    textareas
        .create_description
        .input(textarea_input_from_key_event(
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Down,
                crossterm::event::KeyModifiers::NONE,
            ),
        ));

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("textarea input");

    assert!(handled);
    assert_eq!(
        state.create_focus,
        tui_kit_runtime::CreateModalFocus::Submit
    );
    assert_eq!(state.create_description, "first\nsecond");
}

#[test]
fn textarea_down_inside_insert_text_moves_cursor_before_leaving_field() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: tui_kit_runtime::kinic_tabs::KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_mode: tui_kit_runtime::InsertMode::InlineText,
        insert_focus: tui_kit_runtime::InsertFormFocus::Text,
        insert_text: "first\nsecond".into(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("textarea input");

    assert!(handled);
    assert_eq!(state.insert_focus, tui_kit_runtime::InsertFormFocus::Text);
    assert_eq!(textareas.insert_text.cursor().0, 1);
}

#[test]
fn textarea_down_on_last_insert_text_row_moves_to_next_field() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: tui_kit_runtime::kinic_tabs::KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_mode: tui_kit_runtime::InsertMode::InlineText,
        insert_focus: tui_kit_runtime::InsertFormFocus::Text,
        insert_text: "first\nsecond".into(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);
    textareas.insert_text.input(textarea_input_from_key_event(
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ),
    ));

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("textarea input");

    assert!(handled);
    assert_eq!(state.insert_focus, tui_kit_runtime::InsertFormFocus::Submit);
    assert_eq!(state.insert_text, "first\nsecond");
}

#[test]
fn chat_textarea_shift_enter_submits_instead_of_inserting_newline() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "a".to_string(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::SHIFT,
        ),
    )
    .expect("chat textarea input");
    assert!(handled);

    assert_eq!(state.chat_input, "");
}

#[test]
fn chat_textarea_enter_submits_without_inserting_text() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "hello".to_string(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("chat submit");

    assert!(handled);
    assert_eq!(state.chat_input, "");
}

#[test]
fn chat_textarea_multiline_paste_keeps_raw_widget_with_normalized_state() {
    let mut state = CoreState::default();
    let mut textareas = FormTextareas::default();
    textareas.chat_input = textarea_from_text("first line\nsecond line");

    sync_state_from_textareas(&mut state, &textareas);
    sync_form_textareas_from_state(&mut textareas, &state);

    assert_eq!(state.chat_input, "first line second line");
    assert_eq!(
        textareas.chat_input.lines().join("\n"),
        "first line\nsecond line"
    );
}

#[test]
fn chat_textarea_sync_state_preserves_trailing_space() {
    let mut state = CoreState::default();
    let mut textareas = FormTextareas::default();
    textareas.chat_input = textarea_from_text("hello ");

    sync_state_from_textareas(&mut state, &textareas);

    assert_eq!(state.chat_input, "hello ");
}

#[test]
fn chat_textarea_display_value_preserves_trailing_space() {
    let textareas = FormTextareas {
        chat_input: textarea_from_text("hello "),
        ..FormTextareas::default()
    };

    assert_eq!(
        normalized_chat_textarea_value(&textareas.chat_input),
        "hello "
    );
}

#[test]
fn chat_textarea_sync_from_state_preserves_equivalent_trailing_space_in_widget() {
    let mut textareas = FormTextareas::default();
    textareas.chat_input = textarea_from_text("hello ");
    let state = CoreState {
        chat_input: "hello ".to_string(),
        ..CoreState::default()
    };

    sync_form_textareas_from_state(&mut textareas, &state);

    assert_eq!(textareas.chat_input.lines().join("\n"), "hello ");
}

#[test]
fn chat_textarea_sync_from_state_keeps_cursor_when_text_is_unchanged() {
    let mut textareas = FormTextareas::default();
    textareas.chat_input = textarea_from_text("hello ");
    textareas.chat_input.input(textarea_input_from_key_event(
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::NONE,
        ),
    ));
    let cursor_before = textareas.chat_input.cursor();
    let state = CoreState {
        chat_input: "hello ".to_string(),
        ..CoreState::default()
    };

    sync_form_textareas_from_state(&mut textareas, &state);

    assert_eq!(textareas.chat_input.lines().join("\n"), "hello ");
    assert_eq!(textareas.chat_input.cursor(), cursor_before);
}

#[test]
fn chat_input_cursor_keeps_trailing_space_visible() {
    let mut textarea = textarea_from_text("hello ");
    textarea.move_cursor(ratatui_textarea::CursorMove::End);

    assert_eq!(
        chat_input_cursor(Some(ActiveTextarea::ChatInput), &textarea),
        Some((0, 6))
    );
}

#[test]
fn chat_input_cursor_flattens_multiline_widget_position_for_ui() {
    let mut textarea = textarea_from_text("first line\nsecond line");
    textarea.move_cursor(ratatui_textarea::CursorMove::Jump(1, 6));

    assert_eq!(
        chat_input_cursor(Some(ActiveTextarea::ChatInput), &textarea),
        Some((0, 17))
    );
}

#[test]
fn chat_textarea_display_value_keeps_space_before_next_char() {
    let textareas = FormTextareas {
        chat_input: textarea_from_text("hello w"),
        ..FormTextareas::default()
    };

    assert_eq!(
        normalized_chat_textarea_value(&textareas.chat_input),
        "hello w"
    );
}

#[test]
fn chat_textarea_space_input_updates_state_and_widget_cursor() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "hello".to_string(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);

    let handled = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(' '),
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("chat textarea input");

    assert!(handled);
    assert_eq!(state.chat_input, "hello ");
    assert_eq!(textareas.chat_input.lines().join("\n"), "hello ");
    assert_eq!(textareas.chat_input.cursor(), (0, 6));
    assert_eq!(
        chat_input_cursor(Some(ActiveTextarea::ChatInput), &textareas.chat_input),
        Some((0, 6))
    );
}

#[test]
fn build_ui_places_chat_cursor_after_trailing_space() {
    let theme = Theme::default();
    let cfg = test_runtime_config();
    let animation = AnimationState::new();
    let state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "hello ".to_string(),
        chat_open: true,
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);

    let ui = build_ui(
        &theme, &cfg, &state, &textareas, 0, 0, false, false, &animation,
    );
    let cursor = ui.cursor_position_for_area(Rect::new(0, 0, 120, 40));

    assert!(cursor.is_some());
    let (x, _) = cursor.expect("chat cursor");
    assert!(x > 0);
}

#[test]
fn chat_textarea_up_down_moves_slash_command_selection() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "/".to_string(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    sync_form_textareas_from_state(&mut textareas, &state);

    let moved_down = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("chat slash down");
    assert!(moved_down);
    assert_eq!(textareas.chat_command_selected, 1);

    let moved_up = handle_textarea_input(
        &mut provider,
        &mut state,
        &mut hooks,
        &mut textareas,
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        ),
    )
    .expect("chat slash up");

    assert!(moved_up);
    assert_eq!(textareas.chat_command_selected, 0);
}

#[test]
fn chat_textarea_shift_arrows_switch_scope() {
    let action_prev = chat_textarea_action(
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::SHIFT,
        ),
        crossterm::event::KeyCode::Left,
    );
    let action_next = chat_textarea_action(
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Right,
            crossterm::event::KeyModifiers::SHIFT,
        ),
        crossterm::event::KeyCode::Right,
    );

    assert_eq!(action_prev, Some(CoreAction::ChatScopePrev));
    assert_eq!(action_next, Some(CoreAction::ChatScopeNext));
}

#[test]
fn chat_textarea_shift_arrows_switch_scope_with_extra_modifier_bits() {
    let shift_super = crossterm::event::KeyModifiers::SHIFT | crossterm::event::KeyModifiers::SUPER;
    let prev = chat_textarea_action(
        &crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Left, shift_super),
        crossterm::event::KeyCode::Left,
    );
    let next = chat_textarea_action(
        &crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Right, shift_super),
        crossterm::event::KeyCode::Right,
    );
    assert_eq!(prev, Some(CoreAction::ChatScopePrev));
    assert_eq!(next, Some(CoreAction::ChatScopeNext));
}

#[test]
fn chat_textarea_shift_arrow_with_control_or_alt_does_not_switch_scope() {
    assert_eq!(
        chat_textarea_action(
            &crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Left,
                crossterm::event::KeyModifiers::SHIFT | crossterm::event::KeyModifiers::CONTROL,
            ),
            crossterm::event::KeyCode::Left,
        ),
        None
    );
    assert_eq!(
        chat_textarea_action(
            &crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Right,
                crossterm::event::KeyModifiers::SHIFT | crossterm::event::KeyModifiers::ALT,
            ),
            crossterm::event::KeyCode::Right,
        ),
        None
    );
}

#[test]
fn chat_textarea_brackets_remain_regular_input() {
    let action_prev = chat_textarea_action(
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('['),
            crossterm::event::KeyModifiers::NONE,
        ),
        crossterm::event::KeyCode::Char('['),
    );
    let action_next = chat_textarea_action(
        &crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(']'),
            crossterm::event::KeyModifiers::NONE,
        ),
        crossterm::event::KeyCode::Char(']'),
    );

    assert_eq!(action_prev, None);
    assert_eq!(action_next, None);
}

#[test]
fn chat_submit_slash_new_starts_new_thread_without_sending_message() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "/new".to_string(),
        chat_messages: vec![("user".to_string(), "alpha".to_string())],
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();

    let handled =
        handle_chat_submit_or_command(&mut provider, &mut state, &mut hooks, &mut textareas)
            .expect("slash command");

    assert!(handled);
    assert_eq!(
        state.chat_messages,
        vec![("user".to_string(), "alpha".to_string())]
    );
    assert!(state.chat_input.is_empty());
}

#[test]
fn chat_submit_uses_selected_slash_command_candidate() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "/".to_string(),
        chat_scope: tui_kit_runtime::ChatScope::Selected,
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();
    textareas.chat_command_selected = 1;

    let handled =
        handle_chat_submit_or_command(&mut provider, &mut state, &mut hooks, &mut textareas)
            .expect("slash command");

    assert!(handled);
    assert_eq!(state.chat_scope, tui_kit_runtime::ChatScope::All);
    assert!(state.chat_input.is_empty());
}

#[test]
fn chat_submit_slash_all_switches_scope_without_sending_message() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "/all".to_string(),
        chat_scope: tui_kit_runtime::ChatScope::Selected,
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();

    let handled =
        handle_chat_submit_or_command(&mut provider, &mut state, &mut hooks, &mut textareas)
            .expect("slash command");

    assert!(handled);
    assert_eq!(state.chat_scope, tui_kit_runtime::ChatScope::All);
    assert!(state.chat_messages.is_empty());
    assert!(state.chat_input.is_empty());
}

#[test]
fn chat_submit_normalizes_multiline_input_before_sending_message() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        ..CoreState::default()
    };
    let mut textareas = FormTextareas {
        chat_input: textarea_from_text("first line\nsecond line"),
        ..FormTextareas::default()
    };
    sync_state_from_textareas(&mut state, &textareas);

    let handled =
        handle_chat_submit_or_command(&mut provider, &mut state, &mut hooks, &mut textareas)
            .expect("chat submit");

    assert!(handled);
    assert_eq!(
        state.chat_messages,
        vec![("user".to_string(), "first line second line".to_string())]
    );
    assert!(state.chat_input.is_empty());
}

#[test]
fn chat_submit_slash_all_is_idempotent_when_already_on_all_scope() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "/all".to_string(),
        chat_scope: tui_kit_runtime::ChatScope::All,
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();

    let handled =
        handle_chat_submit_or_command(&mut provider, &mut state, &mut hooks, &mut textareas)
            .expect("slash command");

    assert!(handled);
    assert_eq!(state.chat_scope, tui_kit_runtime::ChatScope::All);
    assert!(state.chat_messages.is_empty());
    assert!(state.chat_input.is_empty());
}

#[test]
fn chat_submit_slash_all_trims_trailing_space_before_matching_command() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "/all ".to_string(),
        chat_scope: tui_kit_runtime::ChatScope::Selected,
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();

    let handled =
        handle_chat_submit_or_command(&mut provider, &mut state, &mut hooks, &mut textareas)
            .expect("slash command");

    assert!(handled);
    assert_eq!(state.chat_scope, tui_kit_runtime::ChatScope::All);
    assert!(state.chat_messages.is_empty());
    assert!(state.chat_input.is_empty());
}

#[test]
fn chat_submit_unknown_slash_command_keeps_input_and_sets_status() {
    let mut provider = TestProvider::ok();
    let mut hooks = NoopRuntimeHooks;
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        focus: PaneFocus::Extra,
        chat_input: "/wat".to_string(),
        ..CoreState::default()
    };
    let mut textareas = FormTextareas::default();

    let handled =
        handle_chat_submit_or_command(&mut provider, &mut state, &mut hooks, &mut textareas)
            .expect("slash command");

    assert!(handled);
    assert_eq!(state.chat_input, "/wat");
    assert_eq!(
        state.status_message.as_deref(),
        Some("Unknown chat command. Try /new or /all.")
    );
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
        insert_file_path_input: "/tmp/doc.pdf".into(),
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
    assert_eq!(state.insert_file_path_input, "/tmp/doc.pdf");
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
fn build_ui_forwards_insert_validation_fields_to_render_tree() {
    let theme = Theme::default();
    let animation = AnimationState::new();
    let state = CoreState {
        current_tab_id: KINIC_INSERT_TAB_ID.to_string(),
        focus: PaneFocus::Form,
        insert_mode: InsertMode::ManualEmbedding,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_embedding: "[0.1, 0.2]".to_string(),
        insert_current_dim: Some("2".to_string()),
        insert_validation_message: Some(
            "Embedding dimension mismatch. Received 2 values, expected 4.".to_string(),
        ),
        ..CoreState::default()
    };
    let textareas = FormTextareas::default();

    let ui = build_ui(
        &theme,
        &test_runtime_config(),
        &state,
        &textareas,
        0,
        0,
        false,
        false,
        &animation,
    );
    let area = Rect::new(0, 0, 120, 40);
    let mut buf = Buffer::empty(area);
    ui.render(area, &mut buf);

    let rendered = (0..area.height)
        .map(|y| {
            (0..area.width)
                .filter_map(|x| buf.cell((x, y)).map(|cell| cell.symbol()))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("Current Dim"));
    assert!(rendered.contains("2"));
    assert!(rendered.contains("Embedding dimension mismatch. Received 2 values, expected 4."));
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
    let mut state = CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        ..CoreState::default()
    };

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
        insert_file_path_input: "stale".into(),
        insert_error: Some("bad path".into()),
        insert_submit_state: tui_kit_runtime::CreateSubmitState::Error,
        ..CoreState::default()
    };

    let effects =
        apply_insert_file_dialog_selection(&mut state, Some(PathBuf::from("/tmp/doc.pdf")));

    assert_eq!(state.insert_file_path_input, "/tmp/doc.pdf");
    assert_eq!(
        state.insert_selected_file_path,
        Some(PathBuf::from("/tmp/doc.pdf"))
    );
    assert_eq!(state.insert_error, None);
    assert_eq!(
        state.insert_submit_state,
        tui_kit_runtime::CreateSubmitState::Idle
    );
    assert_eq!(state.insert_focus, InsertFormFocus::Submit);
    assert_eq!(
        effects,
        vec![CoreEffect::Notify(
            "Selected file: /tmp/doc.pdf".to_string()
        )]
    );
}

#[test]
fn apply_insert_file_dialog_selection_keeps_existing_path_on_cancel() {
    let mut state = CoreState {
        insert_selected_file_path: Some(PathBuf::from("/tmp/existing.md")),
        insert_focus: InsertFormFocus::FilePath,
        ..CoreState::default()
    };

    let effects = apply_insert_file_dialog_selection(&mut state, None);

    assert_eq!(
        state.insert_selected_file_path,
        Some(PathBuf::from("/tmp/existing.md"))
    );
    assert_eq!(state.insert_focus, InsertFormFocus::FilePath);
    assert_eq!(
        effects,
        vec![CoreEffect::Notify("File selection canceled.".to_string())]
    );
}

#[test]
fn should_open_insert_file_dialog_is_false_while_insert_submit_is_in_flight() {
    let state = CoreState {
        insert_submit_state: tui_kit_runtime::CreateSubmitState::Submitting,
        insert_file_path_input: "/tmp/existing.md".into(),
        insert_error: Some("keep".into()),
        status_message: Some("ready".into()),
        ..CoreState::default()
    };

    assert!(!should_open_insert_file_dialog(
        &CoreAction::InsertOpenFileDialog,
        &state
    ));
    assert_eq!(state.insert_file_path_input, "/tmp/existing.md");
    assert_eq!(state.insert_error.as_deref(), Some("keep"));
    assert_eq!(state.status_message.as_deref(), Some("ready"));
}
