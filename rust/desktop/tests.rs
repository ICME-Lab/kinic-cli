use super::*;
use crate::preferences::UserPreferences;

#[test]
fn desktop_session_requires_identity() {
    let error = DesktopSession::from_request(DesktopSessionRequest {
        identity: " ".to_string(),
        network: DesktopNetwork::Local,
    })
    .unwrap_err();

    assert_eq!(error.to_string(), "Identity name is required.");
}

#[test]
fn file_insert_derives_tag_from_file_name() {
    let request = build_insert_request(DesktopInsertRequest {
        mode: DesktopInsertMode::File,
        memory_id: "aaaaa-aa".to_string(),
        tag: None,
        text: None,
        file_path: Some("/tmp/Notes.md".to_string()),
    })
    .unwrap();

    assert!(request.tag().starts_with("Notes-"));
    assert_eq!(request.tag().len(), 14);
}

#[test]
fn pdf_file_uses_pdf_insert_request() {
    let request = build_insert_request(DesktopInsertRequest {
        mode: DesktopInsertMode::File,
        memory_id: "aaaaa-aa".to_string(),
        tag: Some("docs".to_string()),
        text: None,
        file_path: Some("/tmp/guide.pdf".to_string()),
    })
    .unwrap();

    assert!(matches!(request, InsertRequest::Pdf { .. }));
}

#[test]
fn update_preferences_normalizes_blank_default_memory() {
    let view = preferences_view(UserPreferences {
        default_memory_id: Some(" ".to_string()),
        ..UserPreferences::default()
    });

    assert_eq!(view.default_memory_id, None);
}
