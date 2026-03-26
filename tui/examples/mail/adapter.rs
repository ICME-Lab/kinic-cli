use super::MailRecord;
use tui_kit_model::{UiItemDetail, UiItemKind, UiItemSummary, UiRow, UiSection, UiVisibility};

pub fn to_summary(record: &MailRecord) -> UiItemSummary {
    UiItemSummary {
        id: record.id.clone(),
        name: record.subject.clone(),
        leading_marker: None,
        kind: UiItemKind::Custom("mail".to_string()),
        visibility: UiVisibility::Private,
        qualified_name: Some(record.from.clone()),
        subtitle: Some(record.preview.clone()),
        tags: vec!["inbox".to_string()],
    }
}

pub fn to_detail(record: &MailRecord) -> UiItemDetail {
    UiItemDetail {
        id: record.id.clone(),
        title: record.subject.clone(),
        kind: UiItemKind::Custom("mail".to_string()),
        definition: format!("From: {}", record.from),
        location: None,
        docs: Some(record.preview.clone()),
        badges: vec!["mail".to_string()],
        sections: vec![UiSection {
            heading: "Message".to_string(),
            rows: vec![UiRow {
                label: "From".to_string(),
                value: record.from.clone(),
            }],
            body_lines: vec![record.preview.clone()],
        }],
    }
}
