use super::*;
use tui_kit_runtime::kinic_tabs::{
    KINIC_CREATE_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID, KINIC_SETTINGS_TAB_ID,
};
use tui_kit_runtime::{CoreState, ProviderSnapshot, TransferModalState, apply_snapshot};

mod key_mapping {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

    #[test]
    fn action_from_keycode_maps_search_input() {
        assert_eq!(
            action_from_keycode(KeyCode::Char('x'), PaneFocus::Search, KINIC_MEMORIES_TAB_ID),
            Some(CoreAction::SearchInput('x'))
        );
    }

    #[test]
    fn normalize_host_input_event_ignores_non_press_key_events() {
        let key_event = Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        });

        assert_eq!(normalize_host_input_event(key_event), None);
    }
}

mod tab_resolution {
    use super::*;

    #[test]
    fn resolve_tab_action_uses_host_and_default_ids() {
        let host_mapped = resolve_tab_action(CoreAction::SelectTabIndex(1), &["a", "b", "c"]);
        let default_mapped = resolve_tab_action(CoreAction::SelectTabIndex(2), &[]);

        assert_eq!(host_mapped, Some(CoreAction::SetTab(CoreTabId::new("b"))));
        assert_eq!(
            default_mapped,
            Some(CoreAction::SetTab(CoreTabId::new("tab-3")))
        );
    }
}

mod effect_application {
    use super::*;

    #[test]
    fn execute_effects_updates_status_message() {
        let mut state = CoreState::default();
        execute_effects_to_status(&mut state, vec![CoreEffect::Notify("hello".to_string())]);
        assert_eq!(state.status_message.as_deref(), Some("hello"));
    }

    #[test]
    fn persistent_notify_survives_snapshot_until_cleared() {
        let mut state = CoreState::default();
        execute_effects_to_status(
            &mut state,
            vec![CoreEffect::NotifyPersistent("done".to_string())],
        );
        assert_eq!(state.status_message.as_deref(), Some("done"));
        assert_eq!(state.persistent_status_message.as_deref(), Some("done"));

        apply_snapshot(
            &mut state,
            ProviderSnapshot {
                status_message: Some("generic".to_string()),
                ..ProviderSnapshot::default()
            },
        );
        assert_eq!(state.status_message.as_deref(), Some("done"));

        state.persistent_status_message = None;
        assert_eq!(state.persistent_status_message, None);

        apply_snapshot(
            &mut state,
            ProviderSnapshot {
                status_message: Some("generic".to_string()),
                ..ProviderSnapshot::default()
            },
        );
        assert_eq!(state.status_message.as_deref(), Some("generic"));
    }

    #[test]
    fn execute_effects_sets_memories_tab() {
        let mut state = CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            create_name: "stale".to_string(),
            create_description: "stale".to_string(),
            create_submit_state: CreateSubmitState::Submitting,
            create_error: Some("boom".to_string()),
            ..CoreState::default()
        };

        execute_effects_to_status(
            &mut state,
            vec![CoreEffect::ResetCreateFormAndSetTab {
                tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            }],
        );

        assert_eq!(state.current_tab_id, KINIC_MEMORIES_TAB_ID);
        assert_eq!(state.create_name, "");
        assert_eq!(state.create_description, "");
        assert_eq!(state.create_submit_state, CreateSubmitState::Idle);
        assert_eq!(state.create_error, None);
        assert_eq!(state.create_focus, CreateModalFocus::Name);
    }

    #[test]
    fn focus_pane_applies_visible_pane_on_memories_tab() {
        let mut state = CoreState {
            current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Search,
            ..CoreState::default()
        };

        execute_effects_to_status(&mut state, vec![CoreEffect::FocusPane(PaneFocus::Items)]);

        assert_eq!(state.focus, PaneFocus::Items);
    }

    #[test]
    fn focus_pane_ignores_hidden_pane_off_memories_tab() {
        let mut state = CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Form,
            ..CoreState::default()
        };

        execute_effects_to_status(&mut state, vec![CoreEffect::FocusPane(PaneFocus::Items)]);

        assert_eq!(state.focus, PaneFocus::Form);
    }

    #[test]
    fn set_insert_memory_id_effect_updates_insert_target() {
        let mut state = CoreState {
            insert_memory_id: "aaaaa-aa".to_string(),
            insert_error: Some("boom".to_string()),
            ..CoreState::default()
        };

        execute_effects_to_status(
            &mut state,
            vec![CoreEffect::SetInsertMemoryId("bbbbb-bb".to_string())],
        );

        assert_eq!(state.insert_memory_id, "bbbbb-bb");
        assert_eq!(state.insert_memory_placeholder, None);
        assert_eq!(state.insert_error, None);
    }

    #[test]
    fn set_insert_tag_effect_updates_insert_tag() {
        let mut state = CoreState {
            insert_tag: "docs".to_string(),
            insert_error: Some("boom".to_string()),
            ..CoreState::default()
        };

        execute_effects_to_status(
            &mut state,
            vec![CoreEffect::SetInsertTag("research".to_string())],
        );

        assert_eq!(state.insert_tag, "research");
        assert_eq!(state.insert_error, None);
    }

    #[test]
    fn transfer_effects_reset_modal_state_consistently() {
        let mut state = CoreState {
            transfer_modal: TransferModalState {
                open: true,
                mode: tui_kit_runtime::TransferModalMode::Confirm,
                prerequisites_loading: true,
                confirm_yes: false,
                submit_state: CreateSubmitState::Error,
                error: Some("boom".to_string()),
                ..TransferModalState::default()
            },
            ..CoreState::default()
        };

        execute_effects_to_status(
            &mut state,
            vec![CoreEffect::OpenTransferModal {
                fee_base_units: 100_000,
                available_balance_base_units: 500_000_000,
            }],
        );
        assert!(state.transfer_modal.open);
        assert_eq!(
            state.transfer_modal.mode,
            tui_kit_runtime::TransferModalMode::Edit
        );
        assert!(!state.transfer_modal.prerequisites_loading);
        assert_eq!(state.transfer_modal.fee_base_units, Some(100_000));
        assert_eq!(
            state.transfer_modal.available_balance_base_units,
            Some(500_000_000)
        );
        assert_eq!(state.transfer_modal.submit_state, CreateSubmitState::Idle);
        assert_eq!(state.transfer_modal.error, None);

        execute_effects_to_status(&mut state, vec![CoreEffect::OpenTransferConfirm]);
        assert_eq!(
            state.transfer_modal.mode,
            tui_kit_runtime::TransferModalMode::Confirm
        );
        assert!(state.transfer_modal.confirm_yes);

        execute_effects_to_status(&mut state, vec![CoreEffect::CloseTransferModal]);
        assert!(!state.transfer_modal.open);
        assert_eq!(
            state.transfer_modal.mode,
            tui_kit_runtime::TransferModalMode::Edit
        );
        assert!(!state.transfer_modal.prerequisites_loading);
        assert_eq!(state.transfer_modal.submit_state, CreateSubmitState::Idle);
        assert_eq!(state.transfer_modal.error, None);
    }
}

mod global_commands {
    use super::*;

    #[test]
    fn global_command_handles_primary_shortcuts() {
        let cases = [
            (
                KeyCode::Esc,
                KeyModifiers::NONE,
                PaneFocus::Items,
                KINIC_MEMORIES_TAB_ID,
                false,
                HostGlobalCommand::BackFromItems,
            ),
            (
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
                PaneFocus::Items,
                KINIC_MEMORIES_TAB_ID,
                true,
                HostGlobalCommand::OpenCreateTab,
            ),
            (
                KeyCode::Char('r'),
                KeyModifiers::CONTROL,
                PaneFocus::Items,
                KINIC_MEMORIES_TAB_ID,
                true,
                HostGlobalCommand::RefreshCurrentView,
            ),
            (
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                PaneFocus::Items,
                KINIC_MEMORIES_TAB_ID,
                true,
                HostGlobalCommand::Quit,
            ),
            (
                KeyCode::Esc,
                KeyModifiers::NONE,
                PaneFocus::Extra,
                KINIC_MEMORIES_TAB_ID,
                true,
                HostGlobalCommand::CloseChat,
            ),
            (
                KeyCode::Esc,
                KeyModifiers::NONE,
                PaneFocus::Items,
                KINIC_MEMORIES_TAB_ID,
                true,
                HostGlobalCommand::BackFromItems,
            ),
            (
                KeyCode::Esc,
                KeyModifiers::NONE,
                PaneFocus::Search,
                KINIC_MEMORIES_TAB_ID,
                false,
                HostGlobalCommand::ClearQuery,
            ),
            (
                KeyCode::Esc,
                KeyModifiers::NONE,
                PaneFocus::Search,
                KINIC_MEMORIES_TAB_ID,
                true,
                HostGlobalCommand::BackToTabs,
            ),
            (
                KeyCode::Esc,
                KeyModifiers::NONE,
                PaneFocus::Content,
                KINIC_MEMORIES_TAB_ID,
                true,
                HostGlobalCommand::BackFromContent,
            ),
        ];

        for (code, modifiers, focus, tab_id, query_is_empty, expected) in cases {
            assert_eq!(
                global_command_for_key(
                    code,
                    modifiers,
                    focus,
                    tab_id,
                    false,
                    false,
                    query_is_empty,
                ),
                expected
            );
        }
    }

    #[test]
    fn global_command_does_not_open_chat_from_search_focus() {
        assert_eq!(
            global_command_for_key(
                KeyCode::Char('C'),
                KeyModifiers::SHIFT,
                PaneFocus::Search,
                KINIC_MEMORIES_TAB_ID,
                false,
                false,
                false,
            ),
            HostGlobalCommand::None
        );
    }

    #[test]
    fn global_command_resolves_escape_for_special_tabs() {
        let cases = [
            (
                PaneFocus::Items,
                KINIC_MEMORIES_TAB_ID,
                HostGlobalCommand::BackFromItems,
            ),
            (
                PaneFocus::Search,
                KINIC_MEMORIES_TAB_ID,
                HostGlobalCommand::BackToTabs,
            ),
            (
                PaneFocus::Tabs,
                KINIC_CREATE_TAB_ID,
                HostGlobalCommand::BackToMemoriesTab,
            ),
            (
                PaneFocus::Form,
                KINIC_CREATE_TAB_ID,
                HostGlobalCommand::BackFromFormToTabs,
            ),
            (
                PaneFocus::Content,
                KINIC_MARKET_TAB_ID,
                HostGlobalCommand::BackToTabs,
            ),
            (
                PaneFocus::Content,
                KINIC_SETTINGS_TAB_ID,
                HostGlobalCommand::BackToTabs,
            ),
        ];

        for (focus, tab_id, expected) in cases {
            assert_eq!(
                global_command_for_key(
                    KeyCode::Esc,
                    KeyModifiers::NONE,
                    focus,
                    tab_id,
                    false,
                    false,
                    true,
                ),
                expected
            );
        }
    }

    #[test]
    fn global_command_leaves_q_unhandled_in_create_focus() {
        assert_eq!(
            global_command_for_key(
                KeyCode::Char('q'),
                KeyModifiers::NONE,
                PaneFocus::Form,
                KINIC_CREATE_TAB_ID,
                false,
                false,
                true,
            ),
            HostGlobalCommand::None
        );
    }

    #[test]
    fn replace_chat_messages_effect_replaces_visible_thread() {
        let mut state = CoreState {
            chat_messages: vec![("user".to_string(), "old".to_string())],
            chat_scroll: 5,
            ..CoreState::default()
        };

        execute_effects_to_status(
            &mut state,
            vec![CoreEffect::ReplaceChatMessages(vec![(
                "assistant".to_string(),
                "new".to_string(),
            )])],
        );

        assert_eq!(
            state.chat_messages,
            vec![("assistant".to_string(), "new".to_string())]
        );
        assert_eq!(state.chat_scroll, 0);
    }

    #[test]
    fn append_chat_message_effect_extends_thread_and_updates_loading() {
        let mut state = CoreState {
            chat_messages: vec![("user".to_string(), "hello".to_string())],
            chat_loading: true,
            ..CoreState::default()
        };

        execute_effects_to_status(
            &mut state,
            vec![
                CoreEffect::AppendChatMessage {
                    role: "assistant".to_string(),
                    content: "world".to_string(),
                },
                CoreEffect::SetChatLoading(false),
            ],
        );

        assert_eq!(
            state.chat_messages,
            vec![
                ("user".to_string(), "hello".to_string()),
                ("assistant".to_string(), "world".to_string())
            ]
        );
        assert!(!state.chat_loading);
    }
}
