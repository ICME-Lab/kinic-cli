use super::*;

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
