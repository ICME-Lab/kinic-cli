use super::provider::KinicRecord;
use serde_json::Value;
use tui_kit_model::{
    UiItemDetail, UiItemKind, UiItemSummary, UiRow, UiSection, UiVisibility,
};

pub fn to_summary(record: &KinicRecord) -> UiItemSummary {
    UiItemSummary {
        id: record.id.clone(),
        name: record.title.clone(),
        kind: summary_kind(record),
        visibility: UiVisibility::Public,
        qualified_name: Some(format!("kinic::{}", record.group)),
        subtitle: Some(record.summary.clone()),
        tags: vec![record.group.clone()],
    }
}

pub fn to_detail(record: &KinicRecord) -> UiItemDetail {
    match record.group.as_str() {
        "search-result" => search_result_detail(record),
        "memories" => memory_detail(record),
        _ => generic_detail(record),
    }
}

fn summary_kind(record: &KinicRecord) -> UiItemKind {
    match record.group.as_str() {
        "search-result" => UiItemKind::Custom(String::new()),
        "memories" => UiItemKind::Custom("memory".to_string()),
        other => UiItemKind::Custom(other.to_string()),
    }
}

fn generic_detail(record: &KinicRecord) -> UiItemDetail {
    UiItemDetail {
        id: record.id.clone(),
        title: record.title.clone(),
        kind: summary_kind(record),
        definition: format!("kinic::{}::{}", record.group, record.id),
        location: None,
        docs: Some(record.content_md.clone()),
        badges: vec![record.group.clone()],
        sections: vec![
            UiSection {
                heading: "Overview".to_string(),
                rows: vec![UiRow {
                    label: "Summary".to_string(),
                    value: record.summary.clone(),
                }],
                body_lines: record.content_md.lines().map(|line| line.to_string()).collect(),
            },
            metadata_section(record, &[]),
        ],
    }
}

fn memory_detail(record: &KinicRecord) -> UiItemDetail {
    let detail_text = section_body(&record.content_md, "### Detail");
    let guidance = section_body(&record.content_md, "### Search");

    UiItemDetail {
        id: record.id.clone(),
        title: format!("Memory {}", short_id(&record.id)),
        kind: summary_kind(record),
        definition: String::new(),
        location: None,
        docs: Some(record.content_md.clone()),
        badges: vec![],
        sections: vec![
            UiSection {
                heading: "Overview".to_string(),
                rows: vec![
                    UiRow {
                        label: "Canister".to_string(),
                        value: record.id.clone(),
                    },
                    UiRow {
                        label: "Status".to_string(),
                        value: summary_value(&record.summary),
                    },
                ],
                body_lines: vec![
                    "Selected memory canister for remote semantic search.".to_string(),
                ],
            },
            UiSection {
                heading: "Detail".to_string(),
                rows: vec![],
                body_lines: if detail_text.is_empty() {
                    vec!["No additional detail available.".to_string()]
                } else {
                    detail_text
                },
            },
            UiSection {
                heading: "Search".to_string(),
                rows: vec![],
                body_lines: if guidance.is_empty() {
                    vec![
                        "Type a query in Search and press Enter to fetch matching chunks."
                            .to_string(),
                    ]
                } else {
                    guidance
                },
            },
            metadata_section(record, &[("Kind", "memory".to_string())]),
        ],
    }
}

fn search_result_detail(record: &KinicRecord) -> UiItemDetail {
    let memory_id = row_value(&record.content_md, "- Memory:");
    let score = row_value(&record.content_md, "- Score:");
    let tag = row_value(&record.content_md, "- Tag:");
    let sentence = section_body(&record.content_md, "### Sentence");
    let raw_payload = section_body(&record.content_md, "### Raw Payload").join("\n");
    let pretty_payload = pretty_payload(&raw_payload);

    UiItemDetail {
        id: record.id.clone(),
        title: record.title.clone(),
        kind: summary_kind(record),
        definition: String::new(),
        location: None,
        docs: Some(record.content_md.clone()),
        badges: vec![],
        sections: vec![
            UiSection {
                heading: "Overview".to_string(),
                rows: vec![
                    optional_row("Memory", memory_id),
                    optional_row("Score", score),
                    optional_row("Tag", tag),
                ]
                .into_iter()
                .flatten()
                .collect(),
                body_lines: vec![record.summary.clone()],
            },
            UiSection {
                heading: "Sentence".to_string(),
                rows: vec![],
                body_lines: if sentence.is_empty() {
                    vec!["No sentence extracted from payload.".to_string()]
                } else {
                    sentence
                },
            },
            UiSection {
                heading: "Payload".to_string(),
                rows: vec![],
                body_lines: pretty_payload.lines().map(|line| line.to_string()).collect(),
            },
            metadata_section(record, &[("Kind", "search hit".to_string())]),
        ],
    }
}

fn metadata_section(record: &KinicRecord, extra_rows: &[(&str, String)]) -> UiSection {
    let mut rows = vec![
        UiRow {
            label: "Group".to_string(),
            value: record.group.clone(),
        },
        UiRow {
            label: "Id".to_string(),
            value: record.id.clone(),
        },
    ];
    rows.extend(extra_rows.iter().map(|(label, value)| UiRow {
        label: (*label).to_string(),
        value: value.clone(),
    }));
    UiSection {
        heading: "Metadata".to_string(),
        rows,
        body_lines: vec![],
    }
}

fn row_value(body: &str, prefix: &str) -> Option<String> {
    body.lines()
        .find(|line| line.trim_start().starts_with(prefix))
        .map(|line| {
            line.trim_start()
                .trim_start_matches(prefix)
                .trim()
                .trim_matches('`')
                .to_string()
        })
}

fn section_body(body: &str, marker: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut in_section = false;
    for line in body.lines() {
        if line.starts_with("### ") {
            if line == marker {
                in_section = true;
                continue;
            }
            if in_section {
                break;
            }
        }
        if in_section {
            if line.trim().is_empty() {
                lines.push(String::new());
            } else {
                lines.push(line.trim_end().to_string());
            }
        }
    }
    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

fn pretty_payload(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| raw.to_string())
}

fn summary_value(summary: &str) -> String {
    summary
        .split(':')
        .nth(1)
        .map(str::trim)
        .unwrap_or(summary)
        .to_string()
}

fn short_id(id: &str) -> String {
    id.chars().take(5).collect()
}

fn optional_row(label: &str, value: Option<String>) -> Option<UiRow> {
    value.map(|value| UiRow {
        label: label.to_string(),
        value,
    })
}
