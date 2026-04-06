use super::*;

fn set_memory_selection(provider: &mut KinicProvider, memory_id: &str) {
    provider.cursor_memory_id = Some(memory_id.to_string());
}

#[test]
fn open_rename_memory_uses_active_memory_name() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        id: "aaaaa-aa".to_string(),
        status: "running".to_string(),
        detail: "detail".to_string(),
        searchable_memory_id: Some("aaaaa-aa".to_string()),
        name: "Alpha Memory".to_string(),
        version: "1.0.0".to_string(),
        dim: None,
        owners: None,
        stable_memory_size: None,
        cycle_amount: None,
        users: None,
    }];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(&CoreAction::OpenRenameMemory, &CoreState::default())
        .expect("rename open output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::OpenRenameMemory { memory_id, current_name }
            if memory_id == "aaaaa-aa" && current_name == "Alpha Memory"
    )));
}

#[test]
fn memory_content_open_selected_opens_rename_modal_for_name_row() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        id: "aaaaa-aa".to_string(),
        status: "running".to_string(),
        detail: "detail".to_string(),
        searchable_memory_id: Some("aaaaa-aa".to_string()),
        name: "Alpha Memory".to_string(),
        version: "1.0.0".to_string(),
        dim: None,
        owners: None,
        stable_memory_size: None,
        cycle_amount: None,
        users: Some(vec![bridge::MemoryUser {
            principal_id: "user-1".to_string(),
            role: "reader".to_string(),
        }]),
    }];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(
            &CoreAction::MemoryContentOpenSelected,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                memory_content_action_index: 0,
                ..CoreState::default()
            },
        )
        .expect("content open output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::OpenRenameMemory { memory_id, current_name }
            if memory_id == "aaaaa-aa" && current_name == "Alpha Memory"
    )));
}

#[test]
fn open_rename_memory_uses_resolved_name_from_metadata_object() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        id: "aaaaa-aa".to_string(),
        status: "running".to_string(),
        detail: "detail".to_string(),
        searchable_memory_id: Some("aaaaa-aa".to_string()),
        name: "{\"description\":\"ddddd\",\"name\":\"tetete\"}".to_string(),
        version: "1.0.0".to_string(),
        dim: None,
        owners: None,
        stable_memory_size: None,
        cycle_amount: None,
        users: None,
    }];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(&CoreAction::OpenRenameMemory, &CoreState::default())
        .expect("rename open output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::OpenRenameMemory { memory_id, current_name }
            if memory_id == "aaaaa-aa" && current_name == "tetete"
    )));
}

#[test]
fn open_rename_memory_ignores_add_memory_action_selection() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        id: "aaaaa-aa".to_string(),
        status: "running".to_string(),
        detail: "detail".to_string(),
        searchable_memory_id: Some("aaaaa-aa".to_string()),
        name: "Alpha Memory".to_string(),
        version: "1.0.0".to_string(),
        dim: None,
        owners: None,
        stable_memory_size: None,
        cycle_amount: None,
        users: None,
    }];
    provider.refresh_memory_records_from_summaries();
    set_memory_selection(&mut provider, "aaaaa-aa");

    let output = provider
        .handle_action(
            &CoreAction::OpenRenameMemory,
            &CoreState {
                current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
                selected_index: Some(provider.current_records().len()),
                ..CoreState::default()
            },
        )
        .expect("rename open output");

    assert!(
        output
            .effects
            .iter()
            .all(|effect| !matches!(effect, CoreEffect::OpenRenameMemory { .. }))
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Select a memory before renaming."
    )));
}

#[test]
fn rename_memory_submit_rejects_blank_name() {
    let mut provider = KinicProvider::new(live_config());
    let state = CoreState {
        rename_memory: RenameMemoryModalState {
            form: TextInputModalState {
                open: true,
                value: "   ".to_string(),
                ..TextInputModalState::default()
            },
            memory_id: "aaaaa-aa".to_string(),
            ..RenameMemoryModalState::default()
        },
        ..CoreState::default()
    };

    let output = provider
        .handle_action(&CoreAction::RenameMemorySubmit, &state)
        .expect("rename submit output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::RenameFormError(Some(message)) if message == "Memory name is required."
    )));
}

#[test]
fn poll_rename_submit_background_updates_memory_name_and_closes_overlay() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![running_memory_summary("aaaaa-aa", "detail")];
    provider.memory_summaries[0].name = "Old Name".to_string();
    provider.refresh_memory_records_from_summaries();
    let (tx, rx) = mpsc::channel();
    provider.rename_submit_task.receiver = Some(rx);
    provider.rename_submit_task.in_flight = true;
    tx.send(RenameSubmitTaskOutput {
        memory_id: "aaaaa-aa".to_string(),
        next_name: "New Name".to_string(),
        result: Ok(()),
    })
    .expect("rename result should send");

    let output = provider
        .poll_rename_submit_background(&CoreState::default())
        .expect("rename background output");

    assert!(!provider.rename_submit_task.in_flight);
    assert_eq!(provider.memory_summaries[0].name, "New Name");
    assert!(
        output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::CloseRenameMemory))
    );
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Renamed memory to New Name."
    )));
    assert_eq!(provider.memory_records[0].title, "New Name");
    assert!(provider.memory_records[0].summary.contains("aaaaa-aa"));
}

#[test]
fn poll_rename_submit_background_resets_task_when_worker_disconnects() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = mpsc::channel::<RenameSubmitTaskOutput>();
    provider.rename_submit_task.receiver = Some(rx);
    provider.rename_submit_task.in_flight = true;
    drop(tx);

    let output = provider
        .poll_rename_submit_background(&CoreState::default())
        .expect("rename disconnect output");

    assert!(matches!(
        output.effects.as_slice(),
        [CoreEffect::RenameFormError(Some(message))]
            if message == "Rename request failed."
    ));
    assert!(provider.rename_submit_task.receiver.is_none());
    assert!(!provider.rename_submit_task.in_flight);
}

#[test]
fn renamed_memory_is_searchable_by_name_in_browser_filter() {
    let mut provider = KinicProvider::new(live_config());
    provider.memory_summaries = vec![MemorySummary {
        id: "aaaaa-aa".to_string(),
        status: "running".to_string(),
        detail: "{\"description\":\"Quarterly goals\",\"name\":\"Alpha Memory\"}".to_string(),
        searchable_memory_id: Some("aaaaa-aa".to_string()),
        name: "Alpha Memory".to_string(),
        version: "1.0.0".to_string(),
        dim: None,
        owners: None,
        stable_memory_size: None,
        cycle_amount: None,
        users: None,
    }];
    provider.refresh_memory_records_from_summaries();
    provider.query = "alpha".to_string();

    let visible = provider.current_records();

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].title, "Alpha Memory");
}
