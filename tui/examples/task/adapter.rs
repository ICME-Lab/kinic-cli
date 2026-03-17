use super::TaskRecord;
use tui_kit_model::{UiItemDetail, UiItemKind, UiItemSummary, UiRow, UiSection, UiVisibility};

pub fn to_summary(record: &TaskRecord) -> UiItemSummary {
    UiItemSummary {
        id: record.id.clone(),
        name: record.title.clone(),
        kind: UiItemKind::Custom("task".to_string()),
        visibility: UiVisibility::Private,
        qualified_name: Some(record.project.clone()),
        subtitle: Some(record.note.clone()),
        tags: vec![record.status.clone()],
    }
}

pub fn to_detail(record: &TaskRecord) -> UiItemDetail {
    UiItemDetail {
        id: record.id.clone(),
        title: record.title.clone(),
        kind: UiItemKind::Custom("task".to_string()),
        definition: format!("[{}] {}", record.status, record.title),
        location: None,
        docs: Some(record.note.clone()),
        badges: vec![record.status.clone()],
        sections: vec![UiSection {
            heading: "Task".to_string(),
            rows: vec![
                UiRow {
                    label: "Project".to_string(),
                    value: record.project.clone(),
                },
                UiRow {
                    label: "Status".to_string(),
                    value: record.status.clone(),
                },
            ],
            body_lines: vec![record.note.clone()],
        }],
    }
}
