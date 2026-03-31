use super::*;

fn raw_insert_state(text: &str, embedding: &str) -> CoreState {
    CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_text: text.to_string(),
        insert_embedding: embedding.to_string(),
        ..CoreState::default()
    }
}

fn pdf_insert_state(file_path: &str) -> CoreState {
    CoreState {
        insert_mode: InsertMode::Pdf,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_file_path: file_path.to_string(),
        ..CoreState::default()
    }
}

fn normal_insert_file_state(file_path: &str) -> CoreState {
    CoreState {
        insert_mode: InsertMode::Normal,
        insert_memory_id: "aaaaa-aa".to_string(),
        insert_tag: "docs".to_string(),
        insert_file_path: file_path.to_string(),
        ..CoreState::default()
    }
}

#[test]
fn insert_submit_rejects_invalid_embedding_json_before_background_submit() {
    let mut provider = KinicProvider::new(live_config());
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
fn build_insert_request_prefers_explicit_memory_over_saved_default() {
    let mut provider = KinicProvider::new(live_config());
    provider.user_preferences.default_memory_id = Some("aaaaa-aa".to_string());

    let request = provider.build_insert_request(&CoreState {
        insert_mode: InsertMode::Raw,
        insert_memory_id: "bbbbb-bb".to_string(),
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
    let mut provider = KinicProvider::new(live_config());
    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &raw_insert_state("   ", "[0.1]"),
        )
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "Text is required for raw insert."
    )));
}

#[test]
fn insert_submit_rejects_missing_pdf_path() {
    let mut provider = KinicProvider::new(live_config());
    let output = provider
        .handle_action(&CoreAction::InsertSubmit, &pdf_insert_state(""))
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message == "File path is required for PDF insert."
    )));
}

#[test]
fn insert_submit_rejects_pdf_submit_after_validation() {
    let mut provider = KinicProvider::new(live_config());
    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &pdf_insert_state("/path/that/does/not/need/to/exist.pdf"),
        )
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message.contains("File path does not exist")
    )));
}

#[test]
fn insert_submit_rejects_nonexistent_normal_file_path_before_background_submit() {
    let mut provider = KinicProvider::new(live_config());
    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &normal_insert_file_state("/path/that/does/not/need/to/exist.md"),
        )
        .expect("insert submit should succeed");

    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::InsertFormError(Some(message))
            if message.contains("File path does not exist")
    )));
}

#[test]
fn insert_submit_starts_background_submit_for_valid_request() {
    let mut provider = KinicProvider::new(live_config());
    let output = provider
        .handle_action(
            &CoreAction::InsertSubmit,
            &CoreState {
                insert_mode: InsertMode::Normal,
                insert_memory_id: "aaaaa-aa".to_string(),
                insert_tag: "docs".to_string(),
                insert_text: "  keep spacing  ".to_string(),
                ..CoreState::default()
            },
        )
        .expect("insert submit should return output");

    assert!(provider.insert_submit_in_flight);
    assert!(provider.pending_insert_submit.is_some());
    assert!(output.effects.iter().any(|effect| matches!(
        effect,
        CoreEffect::Notify(message)
            if message == "Submitting insert request..."
    )));
    assert!(
        !output
            .effects
            .iter()
            .any(|effect| matches!(effect, CoreEffect::ResetInsertFormForRepeat))
    );
}
