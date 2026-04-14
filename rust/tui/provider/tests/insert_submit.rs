use std::{fs, path::PathBuf};

use super::*;

fn raw_insert_state(text: &str, embedding: &str) -> CoreState {
    CoreState {
        insert_mode: InsertMode::ManualEmbedding,
        insert_tag: "docs".to_string(),
        insert_text: text.to_string(),
        insert_embedding: embedding.to_string(),
        ..CoreState::default()
    }
}

fn file_insert_state(file_path: &str) -> CoreState {
    CoreState {
        insert_mode: InsertMode::File,
        insert_tag: "docs".to_string(),
        insert_file_path_input: file_path.to_string(),
        ..CoreState::default()
    }
}

fn provider_with_active_memory(memory_id: &str) -> KinicProvider {
    let mut provider = KinicProvider::new(live_config());
    set_memory_selection(&mut provider, memory_id);
    provider
}

#[test]
fn insert_submit_rejects_invalid_embedding_json_before_background_submit() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &raw_insert_state("payload", "not-json"),
        )
        .expect("insert submit should return output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message.contains("Embedding must be a JSON array")
    )));
}

#[test]
fn build_insert_request_uses_active_memory_over_saved_default() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());
    set_memory_selection(&mut provider, "bbbbb-bb");

    let request = provider.build_insert_request(&CoreState {
        insert_mode: InsertMode::ManualEmbedding,
        insert_tag: "docs".to_string(),
        insert_text: "payload".to_string(),
        insert_embedding: "[0.1]".to_string(),
        ..CoreState::default()
    });

    assert!(matches!(
        request,
        InsertRequest::Raw { memory_id, .. } if memory_id == "bbbbb-bb"
    ));
}

#[test]
fn build_insert_request_uses_inline_text_mode_without_file_path() {
    let provider = provider_with_active_memory("aaaaa-aa");
    let request = provider.build_insert_request(&CoreState {
        insert_mode: InsertMode::InlineText,
        insert_tag: "docs".to_string(),
        insert_text: "hello".to_string(),
        insert_file_path_input: "/tmp/ignored.md".to_string(),
        ..CoreState::default()
    });

    assert_eq!(
        request,
        InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: Some("hello".to_string()),
            file_path: None,
        }
    );
}

#[test]
fn build_insert_request_uses_file_mode_for_non_pdf_paths() {
    let provider = provider_with_active_memory("aaaaa-aa");
    let request = provider.build_insert_request(&CoreState {
        insert_mode: InsertMode::File,
        insert_tag: "docs".to_string(),
        insert_text: "ignored".to_string(),
        insert_file_path_input: "/tmp/doc.md".to_string(),
        ..CoreState::default()
    });

    assert_eq!(
        request,
        InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: None,
            file_path: Some(PathBuf::from("/tmp/doc.md")),
        }
    );
}

#[test]
fn build_insert_request_prefers_selected_file_path_over_manual_input() {
    let provider = provider_with_active_memory("aaaaa-aa");
    let request = provider.build_insert_request(&CoreState {
        insert_mode: InsertMode::File,
        insert_tag: "docs".to_string(),
        insert_file_path_input: "/tmp/manual.md".to_string(),
        insert_selected_file_path: Some(PathBuf::from("/tmp/dialog.pdf")),
        ..CoreState::default()
    });

    assert_eq!(
        request,
        InsertRequest::Pdf {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            file_path: PathBuf::from("/tmp/dialog.pdf"),
        }
    );
}

#[test]
fn build_insert_request_uses_file_mode_for_pdf_paths() {
    let provider = provider_with_active_memory("aaaaa-aa");
    let request = provider.build_insert_request(&CoreState {
        insert_mode: InsertMode::File,
        insert_tag: "docs".to_string(),
        insert_file_path_input: "/tmp/doc.PDF".to_string(),
        ..CoreState::default()
    });

    assert_eq!(
        request,
        InsertRequest::Pdf {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            file_path: PathBuf::from("/tmp/doc.PDF"),
        }
    );
}

#[cfg(unix)]
#[test]
fn build_insert_request_preserves_non_utf8_selected_file_path() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let provider = provider_with_active_memory("aaaaa-aa");
    let selected_path = PathBuf::from(OsString::from_vec(vec![
        b'/', b't', b'm', b'p', b'/', 0xf0, 0x80, b'.', b'm', b'd',
    ]));
    let request = provider.build_insert_request(&CoreState {
        insert_mode: InsertMode::File,
        insert_tag: "docs".to_string(),
        insert_selected_file_path: Some(selected_path.clone()),
        ..CoreState::default()
    });

    assert_eq!(
        request,
        InsertRequest::Normal {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            text: None,
            file_path: Some(selected_path),
        }
    );
}

#[test]
fn format_insert_submit_error_reports_error_stage() {
    assert_eq!(
        format_insert_submit_error(&bridge::InsertMemoryError::ResolveAgentFactory(
            "auth missing".to_string(),
        )),
        "Could not resolve agent configuration. Cause: auth missing"
    );
    assert_eq!(
        format_insert_submit_error(&bridge::InsertMemoryError::BuildAgent(
            "transport down".to_string(),
        )),
        "Could not build agent. Cause: transport down"
    );
    assert_eq!(
        format_insert_submit_error(&bridge::InsertMemoryError::ParseMemoryId(
            "invalid principal".to_string(),
        )),
        "Could not resolve memory canister. Cause: invalid principal"
    );
}

#[test]
fn insert_submit_rejects_blank_raw_text() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &raw_insert_state("   ", "[0.1]"))
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "Text is required for raw insert."
    )));
}

#[test]
fn insert_submit_rejects_missing_file_path_for_file_mode() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &file_insert_state(""))
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "File path is required for file insert."
    )));
}

#[test]
fn insert_submit_rejects_existing_pdf_submit_after_validation() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    let file_path = write_temp_file_with_extension("pdf", "not really a pdf");
    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &file_insert_state(&file_path))
        .expect("insert submit should succeed");

    assert!(provider.insert_submit_task.in_flight);
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Submitting insert request..."
    )));
    fs::remove_file(file_path).expect("temporary file should be removable");
}

#[test]
fn insert_submit_rejects_nonexistent_normal_file_path_before_background_submit() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &file_insert_state("/path/that/does/not/need/to/exist.md"),
        )
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "File path does not exist: /path/that/does/not/need/to/exist.md"
    )));
}

#[test]
fn insert_submit_rejects_unsupported_file_extension_before_background_submit() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &file_insert_state("/tmp/unsupported.exe"),
        )
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message
                == "File path must use a supported .md, .markdown, .mdx, .txt, .json, .yaml, .yml, .csv, .log, .pdf extension."
    )));
}

#[test]
fn insert_submit_rejects_missing_text_for_inline_text_mode() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &CoreState {
                insert_mode: InsertMode::InlineText,
                insert_tag: "docs".to_string(),
                insert_text: "   ".to_string(),
                ..CoreState::default()
            },
        )
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "Text is required for inline text insert."
    )));
}

#[test]
fn insert_success_status_includes_count_tag_and_memory_id() {
    let success = bridge::InsertMemorySuccess {
        memory_id: "aaaaa-aa".to_string(),
        tag: "docs".to_string(),
        inserted_count: 12,
        source_name: None,
    };

    assert_eq!(
        insert_success_status(&success),
        "Inserted 12 chunks (tag: docs) into aaaaa-aa"
    );
}

#[test]
fn insert_success_status_includes_source_name_for_file_insert() {
    let success = bridge::InsertMemorySuccess {
        memory_id: "aaaaa-aa".to_string(),
        tag: "docs".to_string(),
        inserted_count: 12,
        source_name: Some("doc.md".to_string()),
    };

    assert_eq!(
        insert_success_status(&success),
        "Inserted 12 chunks from doc.md (tag: docs) into aaaaa-aa"
    );
}

#[test]
fn poll_insert_submit_background_resets_form_and_notifies_on_success() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = std::sync::mpsc::channel();
    let request_id = 7;
    provider.insert_submit_task.receiver = Some(rx);
    provider.insert_submit_task.request_id = Some(request_id);
    provider.insert_submit_task.in_flight = true;
    tx.send(InsertSubmitTaskOutput {
        request_id,
        result: Ok(bridge::InsertMemorySuccess {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            inserted_count: 12,
            source_name: None,
        }),
    })
    .expect("background insert result should send");

    let output = provider
        .poll_insert_submit_background(&CoreState::default())
        .expect("provider output");

    assert!(matches!(
        output.effects.as_slice(),
        [
            CoreEffect::InsertFormError(None),
            CoreEffect::ResetInsertFormForRepeat,
            CoreEffect::NotifyPersistent(message),
        ] if message == "Inserted 12 chunks (tag: docs) into aaaaa-aa"
    ));
}

#[test]
fn insert_success_status_emits_persistent_notify() {
    let mut provider = KinicProvider::new(live_config());
    let (tx, rx) = std::sync::mpsc::channel();
    provider.insert_submit_task.receiver = Some(rx);
    provider.insert_submit_task.request_id = Some(1);
    provider.insert_submit_task.in_flight = true;
    tx.send(InsertSubmitTaskOutput {
        request_id: 1,
        result: Ok(bridge::InsertMemorySuccess {
            memory_id: "aaaaa-aa".to_string(),
            tag: "docs".to_string(),
            inserted_count: 12,
            source_name: None,
        }),
    })
    .expect("background insert result should send");

    let output = provider
        .poll_insert_submit_background(&CoreState::default())
        .expect("provider output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::NotifyPersistent(message)
            if message == "Inserted 12 chunks (tag: docs) into aaaaa-aa"
    )));
}

#[test]
fn insert_submit_reports_in_flight_insert_requests() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    provider.insert_submit_task.in_flight = true;

    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &CoreState {
                insert_mode: InsertMode::InlineText,
                insert_tag: "docs".to_string(),
                insert_text: "payload".to_string(),
                ..CoreState::default()
            },
        )
        .expect("insert submit should return output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message) if message == "Insert request already running."
    )));
}

#[test]
fn insert_submit_blocks_after_insert_dim_task_returns_error() {
    let mut provider = provider_with_active_memory("aaaaa-aa");
    let (tx, rx) = std::sync::mpsc::channel();
    provider.insert_expected_dim_memory_id = Some("aaaaa-aa".to_string());
    provider.insert_expected_dim = None;
    provider.insert_expected_dim_loading = true;
    provider.insert_expected_dim_load_error = None;
    provider.pending_insert_dim = Some(rx);
    tx.send(super::super::InsertDimTaskOutput {
        memory_id: "aaaaa-aa".to_string(),
        result: Err(bridge::InsertMemoryError::ResolveAgentFactory(
            "network down".to_string(),
        )),
    })
    .expect("dim task output");

    let state = raw_insert_state("payload", "[0.1, 0.2]");
    provider
        .poll_background(&state)
        .expect("insert dim poll should return output");

    assert!(provider.insert_expected_dim_load_error.is_some());
    assert!(!provider.insert_expected_dim_loading);
    assert!(!provider.insert_submit_task.in_flight);

    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &state)
        .expect("insert submit output");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message.contains("Could not load expected embedding dimension")
    )));
}
