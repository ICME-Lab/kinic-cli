use super::*;
use tui_kit_runtime::RemoveMemoryModalState;

fn set_memory_selection(provider: &mut KinicProvider, memory_id: &str) {
    provider.cursor_memory_id = Some(memory_id.to_string());
}

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
    set_memory_selection(&mut provider, "aaaaa-aa");

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        memory_content_action_index: 3,
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
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentOpenSelected,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 3,
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
fn memory_content_open_selected_opens_access_add_for_add_user_row() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        users: Some(vec![bridge::MemoryUser {
            principal_id: "user-1".to_string(),
            role: "reader".to_string(),
        }]),
        ..running_memory_summary("aaaaa-aa", "first")
    }];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

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

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::OpenAccessAdd { memory_id } if memory_id == "aaaaa-aa"
    )));
}

#[test]
fn memory_content_open_selected_opens_access_action_for_user_row() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        users: Some(vec![bridge::MemoryUser {
            principal_id: "user-1".to_string(),
            role: "writer".to_string(),
        }]),
        ..running_memory_summary("aaaaa-aa", "first")
    }];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentOpenSelected,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 1,
                ..CoreState::default()
            },
        )
        .expect("open selected output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::OpenAccessAction { memory_id, principal_id, role }
            if memory_id == "aaaaa-aa"
                && principal_id == "user-1"
                && *role == AccessControlRole::Writer
    )));
}

#[test]
fn build_snapshot_marks_overview_name_row_when_selected_for_rename() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "first")];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        memory_content_action_index: 0,
        ..CoreState::default()
    });

    let content = snapshot.selected_content.expect("selected content");
    let overview = content
        .sections
        .iter()
        .find(|section| section.heading == "Overview")
        .expect("overview section");
    assert!(
        overview
            .rows
            .iter()
            .any(|row| row.label == "> Name" && !row.value.is_empty())
    );
    assert!(
        overview
            .rows
            .iter()
            .any(|row| row.label == "  Status" && !row.value.is_empty())
    );
    assert!(
        overview
            .rows
            .iter()
            .any(|row| row.label == "  Version" && !row.value.is_empty())
    );
    let access = content
        .sections
        .iter()
        .find(|section| section.heading == "Access")
        .expect("access section");
    assert!(
        access
            .body_lines
            .iter()
            .all(|line| !line.trim_start().starts_with('>'))
    );
}

#[test]
fn build_snapshot_reserves_marker_column_for_memory_metadata_rows() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "first")];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        ..CoreState::default()
    });

    let content = snapshot.selected_content.expect("selected content");
    let metadata = content
        .sections
        .iter()
        .find(|section| section.heading == "Metadata")
        .expect("metadata section");
    assert!(
        metadata
            .rows
            .iter()
            .all(|row| row.label.starts_with("  ") && !row.label.starts_with(">"))
    );
}

#[test]
fn build_snapshot_reserves_marker_column_for_unselected_name_row() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        users: Some(vec![bridge::MemoryUser {
            principal_id: "user-1".to_string(),
            role: "reader".to_string(),
        }]),
        ..running_memory_summary("aaaaa-aa", "first")
    }];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        memory_content_action_index: 2,
        ..CoreState::default()
    });

    let content = snapshot.selected_content.expect("selected content");
    let overview = content
        .sections
        .iter()
        .find(|section| section.heading == "Overview")
        .expect("overview section");
    assert!(
        overview
            .rows
            .iter()
            .any(|row| row.label == "  Name" && !row.value.is_empty())
    );
}

#[test]
fn build_snapshot_marks_only_add_user_when_add_user_selected() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        users: Some(vec![bridge::MemoryUser {
            principal_id: "user-1".to_string(),
            role: "reader".to_string(),
        }]),
        ..running_memory_summary("aaaaa-aa", "first")
    }];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        memory_content_action_index: 2,
        ..CoreState::default()
    });

    let content = snapshot.selected_content.expect("selected content");
    let overview = content
        .sections
        .iter()
        .find(|section| section.heading == "Overview")
        .expect("overview section");
    assert!(
        overview
            .rows
            .iter()
            .all(|row| !row.label.trim_start().starts_with('>'))
    );
    let access = content
        .sections
        .iter()
        .find(|section| section.heading == "Access")
        .expect("access section");
    assert_eq!(
        access
            .body_lines
            .iter()
            .filter(|line| line.trim_start().starts_with('>'))
            .count(),
        1
    );
    assert!(access.body_lines.iter().any(|line| line == "> + Add User"));
}

#[test]
fn build_snapshot_marks_only_remove_action_when_remove_selected() {
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
    set_memory_selection(&mut provider, "aaaaa-aa");

    let snapshot = provider.build_snapshot(&CoreState {
        current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
        memory_content_action_index: 3,
        ..CoreState::default()
    });

    let content = snapshot.selected_content.expect("selected content");
    let access = content
        .sections
        .iter()
        .find(|section| section.heading == "Access")
        .expect("access section");
    assert!(
        access
            .body_lines
            .iter()
            .all(|line| !line.trim_start().starts_with('>'))
    );
    let actions = content
        .sections
        .iter()
        .find(|section| section.heading == "Actions")
        .expect("actions section");
    assert_eq!(actions.body_lines, vec!["> Remove from list".to_string()]);
}

#[test]
fn memory_content_jump_moves_sequentially_within_detail() {
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
    set_memory_selection(&mut provider, "aaaaa-aa");

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
            .any(|effect| matches!(effect, CoreEffect::SetMemoryContentActionIndex(1)))
    );

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentJumpPrev,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 4,
                ..CoreState::default()
            },
        )
        .expect("jump prev output");
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SetMemoryContentActionIndex(3)))
    );
}

#[test]
fn memory_content_move_wraps_within_detail() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "first")];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentMoveNext,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 1,
                ..CoreState::default()
            },
        )
        .expect("move next output");
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SetMemoryContentActionIndex(0)))
    );

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentMovePrev,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 0,
                ..CoreState::default()
            },
        )
        .expect("move prev output");
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::SetMemoryContentActionIndex(1)))
    );
}

#[test]
fn memory_content_jump_next_focuses_search_after_last_action() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "first")];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentJumpNext,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 1,
                ..CoreState::default()
            },
        )
        .expect("jump next output");

    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::Search)))
    );
}

#[test]
fn memory_content_jump_prev_focuses_items_before_first_action() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "first")];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentJumpPrev,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 0,
                ..CoreState::default()
            },
        )
        .expect("jump prev output");

    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::FocusPane(PaneFocus::Items)))
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
    set_memory_selection(&mut provider, "aaaaa-aa");

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
    assert_eq!(provider.cursor_memory_id.as_deref(), Some("bbbbb-bb"));
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
    set_memory_selection(&mut provider, "aaaaa-aa");
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
