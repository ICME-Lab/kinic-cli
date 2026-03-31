use super::*;
use tui_kit_runtime::CoreState;
use tui_kit_runtime::kinic_tabs::{
    KINIC_CREATE_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID, KINIC_SETTINGS_TAB_ID,
};

mod effect_application {
    use super::*;

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
                HostGlobalCommand::ClearQuery,
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
                KeyCode::F(5),
                KeyModifiers::NONE,
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
                HostGlobalCommand::BackToTabs,
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
    fn global_command_resolves_escape_for_special_tabs() {
        let cases = [
            (
                PaneFocus::Items,
                KINIC_MEMORIES_TAB_ID,
                HostGlobalCommand::BackToTabs,
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
}
