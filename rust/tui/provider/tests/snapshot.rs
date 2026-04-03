use super::*;
use tui_kit_runtime::RemoveMemoryModalState;

#[test]
fn build_snapshot_prefers_saved_default_for_insert_selector_and_placeholder() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let snapshot = provider.build_snapshot(&CoreState {
        picker: PickerState::List {
            context: PickerContext::InsertTarget,
            items: Vec::new(),
            selected_index: 0,
            selected_id: None,
            mode: PickerListMode::Browsing,
        },
        ..CoreState::default()
    });

    assert_eq!(
        snapshot.picker,
        PickerState::List {
            context: PickerContext::InsertTarget,
            items: vec![
                PickerItem::option("aaaaa-aa", "Alpha Memory", true),
                PickerItem::option("bbbbb-bb", "Beta Memory", false),
            ],
            selected_index: 0,
            selected_id: Some("aaaaa-aa".to_string()),
            mode: PickerListMode::Browsing,
        }
    );
    assert_eq!(
        snapshot.insert_memory_placeholder.as_deref(),
        Some("Alpha Memory")
    );
}

#[test]
fn build_snapshot_prefers_explicit_insert_memory_id_for_insert_selector() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];
    provider.all = provider.memory_records.clone();
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let snapshot = provider.build_snapshot(&CoreState {
        picker: PickerState::List {
            context: PickerContext::InsertTarget,
            items: Vec::new(),
            selected_index: 0,
            selected_id: None,
            mode: PickerListMode::Browsing,
        },
        insert_memory_id: "bbbbb-bb".to_string(),
        ..CoreState::default()
    });

    assert_eq!(
        snapshot.picker,
        PickerState::List {
            context: PickerContext::InsertTarget,
            items: vec![
                PickerItem::option("aaaaa-aa", "Alpha Memory", true),
                PickerItem::option("bbbbb-bb", "Beta Memory", false),
            ],
            selected_index: 1,
            selected_id: Some("bbbbb-bb".to_string()),
            mode: PickerListMode::Browsing,
        }
    );
    assert_eq!(snapshot.insert_memory_placeholder, None);
}

#[test]
fn submit_insert_target_selector_updates_insert_memory_only() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    let state = CoreState {
        picker: PickerState::List {
            context: PickerContext::InsertTarget,
            items: vec![
                PickerItem::option("aaaaa-aa", "Alpha Memory", true),
                PickerItem::option("bbbbb-bb", "Beta Memory", false),
            ],
            selected_index: 1,
            selected_id: Some("bbbbb-bb".to_string()),
            mode: PickerListMode::Browsing,
        },
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::SubmitPicker, &state)
        .expect("picker submit output");

    assert_eq!(
        provider.user_preferences.default_memory_id.as_deref(),
        Some("aaaaa-aa")
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::SetInsertMemoryId(memory_id) if memory_id == "bbbbb-bb"
    )));
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Selected target memory bbbbb-bb"
    )));
}

#[test]
fn submit_tag_management_existing_tag_sets_insert_tag_without_switching_tabs() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.saved_tags = vec!["docs".to_string(), "research".to_string()];
    let state = CoreState {
        current_tab_id: KINIC_SETTINGS_TAB_ID.to_string(),
        picker: PickerState::List {
            context: PickerContext::TagManagement,
            items: vec![
                PickerItem::option("docs", "docs", false),
                PickerItem::option("research", "research", false),
                PickerItem::add_action("+ Add new tag"),
            ],
            selected_index: 0,
            selected_id: Some("docs".to_string()),
            mode: PickerListMode::Browsing,
        },
        insert_tag: "existing-insert-tag".to_string(),
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::SubmitPicker, &state)
        .expect("tag management submit output");

    assert_eq!(
        provider.user_preferences.saved_tags,
        vec!["docs", "research"]
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::SetInsertTag(tag) if tag == "docs"
    )));
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Selected tag docs for insert"
    )));
    assert_eq!(
        output.snapshot.expect("snapshot").picker,
        PickerState::Closed
    );
}

#[test]
fn save_tags_to_preferences_updates_saved_tags_in_tests() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.saved_tags = vec!["docs".to_string()];

    let outcome = provider.save_tags_to_preferences("  research  ".to_string());

    assert_eq!(outcome.result, SaveTagResult::Saved);
    assert_eq!(
        outcome.effect,
        CoreEffect::Notify("Saved tag research".to_string())
    );
    assert_eq!(provider.preferences_health, PreferencesHealth::default());
    assert_eq!(
        provider.user_preferences.saved_tags,
        vec!["docs", "research"]
    );
}

#[test]
fn add_tag_submit_success_closes_picker_input() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.saved_tags = vec!["docs".to_string()];

    let state = CoreState {
        picker: PickerState::Input {
            context: PickerContext::AddTag,
            origin_context: Some(PickerContext::InsertTag),
            value: "  research  ".to_string(),
        },
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::SubmitPicker, &state)
        .expect("add tag submit output");

    assert_eq!(
        provider.user_preferences.saved_tags,
        vec!["docs", "research"]
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Saved tag research"
    )));
    assert_eq!(
        output.snapshot.expect("snapshot").picker,
        PickerState::Closed
    );
}

#[test]
fn add_tag_submit_failure_keeps_picker_input_open() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.saved_tags = vec!["docs".to_string()];
    set_next_test_settings_save_to_fail();

    let state = CoreState {
        picker: PickerState::Input {
            context: PickerContext::AddTag,
            origin_context: Some(PickerContext::InsertTag),
            value: "research".to_string(),
        },
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::SubmitPicker, &state)
        .expect("add tag submit output");

    assert_eq!(provider.user_preferences.saved_tags, vec!["docs"]);
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Tag save failed: No config directory found"
    )));
    assert_eq!(
        output.snapshot.expect("snapshot").picker,
        PickerState::Input {
            context: PickerContext::AddTag,
            origin_context: Some(PickerContext::InsertTag),
            value: "research".to_string(),
        }
    );
}

#[test]
fn add_tag_submit_empty_value_keeps_picker_input_open() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.saved_tags = vec!["docs".to_string()];

    let state = CoreState {
        picker: PickerState::Input {
            context: PickerContext::AddTag,
            origin_context: Some(PickerContext::InsertTag),
            value: "   ".to_string(),
        },
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::SubmitPicker, &state)
        .expect("add tag submit output");

    assert_eq!(provider.user_preferences.saved_tags, vec!["docs"]);
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Tag cannot be empty."
    )));
    assert_eq!(
        output.snapshot.expect("snapshot").picker,
        PickerState::Input {
            context: PickerContext::AddTag,
            origin_context: Some(PickerContext::InsertTag),
            value: "   ".to_string(),
        }
    );
}

#[test]
fn build_snapshot_appends_add_memory_action_in_memories_tab() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.all = provider.memory_records.clone();

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        ..CoreState::default()
    });

    assert_eq!(snapshot.items.len(), 2);
    assert_eq!(snapshot.items[1].id, "kinic-action-add-memory");
    assert_eq!(snapshot.items[1].name, "+ Add Existing Memory Canister");
}

#[test]
fn build_snapshot_shows_remove_action_only_for_manual_memory() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.manual_memory_ids = vec!["aaaaa-aa".to_string()];
    provider.memory_summaries = vec![MemorySummary {
        users: Some(vec![bridge::MemoryUser {
            principal_id: "user-1".to_string(),
            role: "reader".to_string(),
        }]),
        ..running_memory_summary("aaaaa-aa", "first")
    }];
    provider.refresh_memory_records_from_summaries();
    provider.active_memory_id = Some("aaaaa-aa".to_string());

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        memory_content_action_index: 2,
        ..CoreState::default()
    });

    let content = snapshot.selected_content.expect("selected content");
    let actions = content
        .sections
        .iter()
        .find(|section| section.heading == "Actions")
        .expect("actions section");
    assert_eq!(actions.body_lines, vec!["> Remove from list".to_string()]);

    provider.user_preferences.manual_memory_ids.clear();
    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        ..CoreState::default()
    });
    let content = snapshot.selected_content.expect("selected content");
    assert!(
        content
            .sections
            .iter()
            .all(|section| section.heading != "Actions")
    );
}

#[test]
fn memory_content_open_selected_opens_remove_modal_for_manual_memory_action() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.manual_memory_ids = vec!["aaaaa-aa".to_string()];
    provider.memory_summaries = vec![MemorySummary {
        users: Some(vec![bridge::MemoryUser {
            principal_id: "user-1".to_string(),
            role: "reader".to_string(),
        }]),
        ..running_memory_summary("aaaaa-aa", "first")
    }];
    provider.refresh_memory_records_from_summaries();
    provider.active_memory_id = Some("aaaaa-aa".to_string());

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentOpenSelected,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 2,
                ..CoreState::default()
            },
        )
        .expect("open selected output");

    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::OpenRemoveMemory))
    );
}

#[test]
fn memory_content_jump_moves_between_add_and_remove_actions() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.manual_memory_ids = vec!["aaaaa-aa".to_string()];
    provider.memory_summaries = vec![MemorySummary {
        users: Some(vec![
            bridge::MemoryUser {
                principal_id: "user-1".to_string(),
                role: "reader".to_string(),
            },
            bridge::MemoryUser {
                principal_id: "user-2".to_string(),
                role: "writer".to_string(),
            },
        ]),
        ..running_memory_summary("aaaaa-aa", "first")
    }];
    provider.refresh_memory_records_from_summaries();
    provider.active_memory_id = Some("aaaaa-aa".to_string());

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentJumpNext,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 0,
                ..CoreState::default()
            },
        )
        .expect("jump next output");
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SetMemoryContentActionIndex(3)))
    );

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentJumpPrev,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 3,
                ..CoreState::default()
            },
        )
        .expect("jump prev output");
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SetMemoryContentActionIndex(2)))
    );
}

#[test]
fn remove_manual_memory_submit_updates_preferences_and_active_selection() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.manual_memory_ids = vec!["aaaaa-aa".to_string()];
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    provider.memory_summaries = vec![
        running_memory_summary("aaaaa-aa", "first"),
        running_memory_summary("bbbbb-bb", "second"),
    ];
    provider.refresh_memory_records_from_summaries();
    provider.active_memory_id = Some("aaaaa-aa".to_string());

    let output = provider
        .handle_action(
            &CoreAction::RemoveMemorySubmit,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                remove_memory: RemoveMemoryModalState {
                    open: true,
                    confirm_yes: true,
                    ..RemoveMemoryModalState::default()
                },
                ..CoreState::default()
            },
        )
        .expect("remove output");

    assert_eq!(
        provider.user_preferences.manual_memory_ids,
        Vec::<String>::new()
    );
    assert_eq!(provider.user_preferences.default_memory_id, None);
    assert_eq!(provider.active_memory_id.as_deref(), Some("bbbbb-bb"));
    assert!(
        provider
            .memory_records
            .iter()
            .all(|record| record.id != "aaaaa-aa")
    );
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::CloseRemoveMemory))
    );
}

#[test]
fn remove_manual_memory_submit_keeps_modal_open_on_save_failure() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.manual_memory_ids = vec!["aaaaa-aa".to_string()];
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "first")];
    provider.refresh_memory_records_from_summaries();
    provider.active_memory_id = Some("aaaaa-aa".to_string());
    set_next_test_settings_save_to_fail();

    let output = provider
        .handle_action(
            &CoreAction::RemoveMemorySubmit,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                remove_memory: RemoveMemoryModalState {
                    open: true,
                    confirm_yes: true,
                    ..RemoveMemoryModalState::default()
                },
                ..CoreState::default()
            },
        )
        .expect("remove output");

    assert_eq!(
        provider.user_preferences.manual_memory_ids,
        vec!["aaaaa-aa".to_string()]
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::RemoveMemoryFormError(Some(message))
            if message.contains("Manual memory removal failed")
    )));
}

#[test]
fn open_selected_on_add_memory_action_opens_modal() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![live_memory("aaaaa-aa", "Alpha Memory")];
    provider.all = provider.memory_records.clone();

    let output = provider
        .handle_action(
            &CoreAction::OpenSelected,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                selected_index: Some(1),
                ..CoreState::default()
            },
        )
        .expect("open selected output");

    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::OpenAddMemory))
    );
}

#[test]
fn poll_add_memory_validation_background_saves_preference_and_reloads() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel();
    provider.add_memory_validation_task.receiver = Some(rx);
    provider.add_memory_validation_task.in_flight = true;
    tx.send(AddMemoryValidationTaskOutput {
        memory_id: "bbbbb-bb".to_string(),
        result: Ok(()),
    })
    .expect("send validation result");

    let output = provider
        .poll_add_memory_validation_background(&CoreState {
            current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            ..CoreState::default()
        })
        .expect("background output");

    assert_eq!(
        provider.user_preferences.manual_memory_ids,
        vec!["bbbbb-bb".to_string()]
    );
    assert!(provider.initial_memories_in_flight);
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::CloseAddMemory))
    );
}

#[test]
fn validate_add_memory_submit_rejects_invalid_principal() {
    let provider = KinicProvider::new(live_config());

    let error = provider
        .validate_add_memory_submit(&CoreState {
            add_memory: TextInputModalState {
                value: "not-a-principal".to_string(),
                ..TextInputModalState::default()
            },
            ..CoreState::default()
        })
        .expect_err("invalid principal should fail");

    assert_eq!(error, "Invalid principal text: not-a-principal");
}

#[test]
fn submit_tag_management_delete_confirm_removes_saved_tag_and_keeps_picker_open() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.saved_tags = vec!["docs".to_string(), "research".to_string()];

    let output = provider
        .handle_action(
            &CoreAction::SubmitPicker,
            &CoreState {
                picker: PickerState::List {
                    context: PickerContext::TagManagement,
                    items: vec![
                        PickerItem::option("docs", "docs", false),
                        PickerItem::option("research", "research", false),
                        PickerItem::add_action("+ Add new tag"),
                    ],
                    selected_index: 1,
                    selected_id: Some("research".to_string()),
                    mode: PickerListMode::Confirm {
                        kind: PickerConfirmKind::DeleteTag {
                            tag_id: "research".to_string(),
                        },
                    },
                },
                insert_tag: "research".to_string(),
                ..CoreState::default()
            },
        )
        .expect("delete confirm output");

    assert_eq!(provider.user_preferences.saved_tags, vec!["docs"]);
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Deleted tag research"
    )));
    assert!(matches!(
        output.snapshot.expect("snapshot").picker,
        PickerState::List {
            context: PickerContext::TagManagement,
            mode: PickerListMode::Browsing,
            ..
        }
    ));
}

#[test]
fn build_snapshot_exposes_saved_default_memory_id() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let snapshot = provider.build_snapshot(&CoreState::default());

    assert_eq!(
        snapshot.saved_default_memory_id.as_deref(),
        Some("aaaaa-aa")
    );
}

#[test]
fn build_snapshot_exposes_selector_titles() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_records = vec![
        live_memory("aaaaa-aa", "Alpha Memory"),
        live_memory("bbbbb-bb", "Beta Memory"),
    ];

    let snapshot = provider.build_snapshot(&CoreState {
        picker: PickerState::List {
            context: PickerContext::DefaultMemory,
            items: Vec::new(),
            selected_index: 0,
            selected_id: None,
            mode: PickerListMode::Browsing,
        },
        ..CoreState::default()
    });

    assert_eq!(
        snapshot.picker,
        PickerState::List {
            context: PickerContext::DefaultMemory,
            items: vec![
                PickerItem::option("aaaaa-aa", "Alpha Memory", false),
                PickerItem::option("bbbbb-bb", "Beta Memory", false),
            ],
            selected_index: 0,
            selected_id: Some("aaaaa-aa".to_string()),
            mode: PickerListMode::Browsing,
        }
    );
}
