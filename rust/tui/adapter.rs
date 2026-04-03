use super::provider::KinicRecord;
use serde_json::Value;
use tui_kit_model::{UiItemContent, UiItemKind, UiItemSummary, UiRow, UiSection, UiVisibility};

pub fn to_summary(record: &KinicRecord) -> UiItemSummary {
    UiItemSummary {
        id: record.id.clone(),
        name: record.title.clone(),
        leading_marker: None,
        kind: summary_kind(record),
        visibility: UiVisibility::Public,
        qualified_name: Some(format!("kinic::{}", record.group)),
        subtitle: Some(record.summary.clone()),
        tags: vec![record.group.clone()],
    }
}

pub fn to_content(record: &KinicRecord) -> UiItemContent {
    match record.group.as_str() {
        "search-result" => search_result_content(record),
        "memories" => memory_content(record),
        _ => generic_content(record),
    }
}

fn summary_kind(record: &KinicRecord) -> UiItemKind {
    match record.group.as_str() {
        "search-result" => UiItemKind::Custom(String::new()),
        "memories" => UiItemKind::Custom("memory".to_string()),
        other => UiItemKind::Custom(other.to_string()),
    }
}

fn generic_content(record: &KinicRecord) -> UiItemContent {
    UiItemContent {
        id: record.id.clone(),
        title: record.title.clone(),
        subtitle: None,
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
                body_lines: record
                    .content_md
                    .lines()
                    .map(|line| line.to_string())
                    .collect(),
            },
            metadata_section(record, &[]),
        ],
    }
}

fn memory_content(record: &KinicRecord) -> UiItemContent {
    let content_text = section_body(&record.content_md, "### Content");
    let user_lines = memory_user_lines(&section_body(&record.content_md, "### Users"));
    let name = metadata_value(record, "- Name:");
    let description = description_value(&record.content_md);

    UiItemContent {
        id: record.id.clone(),
        title: name.clone(),
        subtitle: description,
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
                        label: "Name".to_string(),
                        value: name.clone(),
                    },
                    UiRow {
                        label: "Status".to_string(),
                        value: summary_value(&record.summary),
                    },
                    UiRow {
                        label: "Version".to_string(),
                        value: metadata_value(record, "- Version:"),
                    },
                ],
                body_lines: vec![],
            },
            UiSection {
                heading: "Access".to_string(),
                rows: vec![],
                body_lines: if user_lines.is_empty() {
                    vec![
                        "unavailable".to_string(),
                        String::new(),
                        "  + Add User".to_string(),
                    ]
                } else {
                    user_lines
                },
            },
            UiSection {
                heading: "Content".to_string(),
                rows: vec![],
                body_lines: if content_text.is_empty() {
                    vec!["No additional content available.".to_string()]
                } else {
                    content_text
                },
            },
            metadata_section(
                record,
                &[
                    ("Owners", metadata_value(record, "- Owners:")),
                    ("Dimension", metadata_value(record, "- Dimension:")),
                    (
                        "Stable Memory Size",
                        metadata_value(record, "- Stable Memory Size:"),
                    ),
                    ("Cycle Amount", metadata_value(record, "- Cycle Amount:")),
                ],
            ),
        ],
    }
}

fn search_result_content(record: &KinicRecord) -> UiItemContent {
    let memory_id = row_value(&record.content_md, "- Memory:");
    let score = row_value(&record.content_md, "- Score:");
    let tag = row_value(&record.content_md, "- Tag:");
    let sentence = section_body(&record.content_md, "### Sentence");
    let raw_payload = section_body(&record.content_md, "### Raw Payload").join("\n");
    let pretty_payload = pretty_payload(&raw_payload);

    UiItemContent {
        id: record.id.clone(),
        title: record.title.clone(),
        subtitle: None,
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
                body_lines: pretty_payload
                    .lines()
                    .map(|line| line.to_string())
                    .collect(),
            },
            metadata_section(record, &[]),
        ],
    }
}

fn metadata_section(_record: &KinicRecord, extra_rows: &[(&str, String)]) -> UiSection {
    let rows = extra_rows
        .iter()
        .map(|(label, value)| UiRow {
            label: (*label).to_string(),
            value: value.clone(),
        })
        .collect();
    UiSection {
        heading: "Metadata".to_string(),
        rows,
        body_lines: vec![],
    }
}

fn memory_user_lines(lines: &[String]) -> Vec<String> {
    if lines
        .first()
        .is_some_and(|line| line.trim() == "No users found.")
    {
        return vec![
            "none".to_string(),
            String::new(),
            "> + Add User".to_string(),
        ];
    } else if lines
        .first()
        .is_some_and(|line| line.trim() == "Users unavailable.")
    {
        return vec![
            "unavailable".to_string(),
            String::new(),
            "> + Add User".to_string(),
        ];
    }

    let mut formatted = lines
        .iter()
        .filter_map(|line| {
            let body = line.trim().strip_prefix("- User:")?.trim();
            let (principal, role) = body.split_once('|')?;
            let principal = principal.trim().trim_matches('`');
            let role = role.trim();
            Some(format!("  {}   {role}", short_id(principal)))
        })
        .collect::<Vec<_>>();

    if !formatted.is_empty() {
        formatted[0].replace_range(..1, ">");
        formatted.push(String::new());
    }
    formatted.push("  + Add User".to_string());
    formatted
}

fn metadata_value(record: &KinicRecord, prefix: &str) -> String {
    row_value(&record.content_md, prefix).unwrap_or_else(|| "unknown".to_string())
}

fn description_value(body: &str) -> Option<String> {
    multiline_row_value(body, "- Description:").or_else(|| row_value(body, "- Description:"))
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

fn multiline_row_value(body: &str, prefix: &str) -> Option<String> {
    let mut lines = body.lines().peekable();
    while let Some(line) = lines.next() {
        if !line.trim_start().starts_with(prefix) {
            continue;
        }
        let mut value_lines = Vec::new();
        while let Some(next_line) = lines.peek().copied() {
            if next_line.starts_with("  ") {
                value_lines.push(next_line.trim_start().to_string());
                lines.next();
                continue;
            }
            break;
        }
        if value_lines.is_empty() {
            return None;
        }
        return Some(value_lines.join("\n"));
    }
    None
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

fn pretty_payload(payload: &str) -> String {
    serde_json::from_str::<Value>(payload)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| payload.to_string())
}

fn summary_value(summary: &str) -> String {
    summary
        .strip_prefix("Status:")
        .map(str::trim)
        .unwrap_or(summary)
        .to_string()
}

pub(crate) fn short_id(id: &str) -> String {
    const EDGE_CHARS: usize = 5;
    const MAX_CHARS: usize = EDGE_CHARS * 2 + 5;

    let char_count = id.chars().count();
    if char_count <= MAX_CHARS {
        return id.to_string();
    }

    let prefix_end = nth_char_boundary(id, EDGE_CHARS);
    let suffix_start = nth_char_boundary(id, char_count - EDGE_CHARS);
    format!("{}...{}", &id[..prefix_end], &id[suffix_start..])
}

fn nth_char_boundary(value: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

fn optional_row(label: &str, value: Option<String>) -> Option<UiRow> {
    value.map(|value| UiRow {
        label: label.to_string(),
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::{memory_content, memory_user_lines, short_id};
    use crate::tui::provider::KinicRecord;

    #[test]
    fn short_id_keeps_short_values() {
        assert_eq!(short_id("aaaaa-aa"), "aaaaa-aa");
    }

    #[test]
    fn short_id_truncates_long_ascii_values() {
        assert_eq!(short_id("1234567890abcdefghij"), "12345...fghij");
    }

    #[test]
    fn short_id_truncates_multibyte_values_on_char_boundaries() {
        assert_eq!(
            short_id("あいうえおかきくけこさしすせそた"),
            "あいうえお...しすせそた"
        );
    }

    #[test]
    fn memory_user_lines_formats_principal_and_role() {
        let lines = vec!["- User: `2chl6-4hpzw-vqaaa-aaaaa-c` | writer".to_string()];

        let formatted = memory_user_lines(&lines);

        assert_eq!(
            formatted,
            vec![
                "> 2chl6...aaa-c   writer".to_string(),
                String::new(),
                "  + Add User".to_string(),
            ]
        );
    }

    #[test]
    fn memory_user_lines_maps_empty_state_to_none() {
        let formatted = memory_user_lines(&["No users found.".to_string()]);

        assert_eq!(
            formatted,
            vec![
                "none".to_string(),
                String::new(),
                "> + Add User".to_string(),
            ]
        );
    }

    #[test]
    fn memory_user_lines_maps_unavailable_state() {
        let formatted = memory_user_lines(&["Users unavailable.".to_string()]);

        assert_eq!(
            formatted,
            vec![
                "unavailable".to_string(),
                String::new(),
                "> + Add User".to_string(),
            ]
        );
    }

    #[test]
    fn memory_content_overview_includes_memory_details() {
        let record = KinicRecord::new(
            "2chl6-4hpzw-vqaaa-aaaaa-c",
            "2chl6-4hpzw-vqaaa-aaaaa-c",
            "memories",
            "Status: running",
            "## Memory\n\n- Id: `2chl6-4hpzw-vqaaa-aaaaa-c`\n- Status: `running`\n- Name: `Alpha`\n- Description:\n  Project notes\n  second line\n- Version: `1.0.0`\n- Owners: `aaaaa-aa, bbbbb-bb`\n- Dimension: `768`\n- Stable Memory Size: `2,048`\n- Cycle Amount: `1.235T`\n\n### Content\nready\n\n### Search\nsearch help\n\n### Users\n- User: `2chl6-4hpzw-vqaaa-aaaaa-c` | writer\n".to_string(),
        );

        let content = memory_content(&record);
        let overview = content
            .sections
            .iter()
            .find(|section| section.heading == "Overview")
            .expect("overview section");
        let metadata = content
            .sections
            .iter()
            .find(|section| section.heading == "Metadata")
            .expect("metadata section");
        let access = content
            .sections
            .iter()
            .find(|section| section.heading == "Access")
            .expect("access section");
        assert!(
            content
                .sections
                .iter()
                .all(|section| section.heading != "Search")
        );

        assert_eq!(content.title, "Alpha");
        assert_eq!(
            content.subtitle.as_deref(),
            Some("Project notes\nsecond line")
        );
        assert!(
            overview
                .rows
                .iter()
                .any(|row| row.label == "Name" && row.value == "Alpha")
        );
        assert!(
            overview
                .rows
                .iter()
                .any(|row| row.label == "Version" && row.value == "1.0.0")
        );
        assert!(overview.rows.iter().all(|row| row.label != "Owners"));
        assert!(overview.rows.iter().all(|row| row.label != "Dimension"));
        assert!(
            metadata
                .rows
                .iter()
                .any(|row| row.label == "Owners" && row.value == "aaaaa-aa, bbbbb-bb")
        );
        assert!(
            metadata
                .rows
                .iter()
                .any(|row| row.label == "Dimension" && row.value == "768")
        );
        assert!(
            metadata
                .rows
                .iter()
                .any(|row| row.label == "Stable Memory Size" && row.value == "2,048")
        );
        assert!(
            metadata
                .rows
                .iter()
                .any(|row| row.label == "Cycle Amount" && row.value == "1.235T")
        );
        assert!(metadata.rows.iter().all(|row| row.label != "Group"));
        assert!(metadata.rows.iter().all(|row| row.label != "Kind"));
        assert!(overview.body_lines.is_empty());
        assert!(
            access
                .body_lines
                .iter()
                .any(|line| line == "> 2chl6...aaa-c   writer")
        );
    }
}
