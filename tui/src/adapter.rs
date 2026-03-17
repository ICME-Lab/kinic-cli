use super::provider::KinicRecord;
use tui_kit_model::{
    UiItemDetail, UiItemKind, UiItemSummary, UiRow, UiSection, UiVisibility,
};

pub fn to_summary(record: &KinicRecord) -> UiItemSummary {
    UiItemSummary {
        id: record.id.clone(),
        name: record.title.clone(),
        kind: UiItemKind::Custom("entry".to_string()),
        visibility: UiVisibility::Public,
        qualified_name: Some(format!("kinic::{}", record.group)),
        subtitle: Some(record.summary.clone()),
        tags: vec![record.group.clone()],
    }
}

pub fn to_detail(record: &KinicRecord) -> UiItemDetail {
    UiItemDetail {
        id: record.id.clone(),
        title: record.title.clone(),
        kind: UiItemKind::Custom("entry".to_string()),
        definition: format!("kinic::{}::{}", record.group, record.id),
        location: None,
        docs: Some(record.content_md.clone()),
        badges: vec![record.group.clone()],
        sections: vec![
            UiSection {
                heading: "Summary".to_string(),
                rows: vec![UiRow {
                    label: "Text".to_string(),
                    value: record.summary.clone(),
                }],
                body_lines: record
                    .content_md
                    .lines()
                    .map(|line| line.to_string())
                    .collect(),
            },
            UiSection {
                heading: "Metadata".to_string(),
                rows: vec![
                    UiRow {
                        label: "Group".to_string(),
                        value: record.group.clone(),
                    },
                    UiRow {
                        label: "Id".to_string(),
                        value: record.id.clone(),
                    },
                ],
                body_lines: vec![],
            },
        ],
    }
}
