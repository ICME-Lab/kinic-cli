//! Settings screen rendering.
//! Where: full-body Settings tab content inside the shared app shell.
//! What: renders current session status and saved preferences from one snapshot.
//! Why: keeps the Settings tab aligned with the Shift+S quick status overlay.

use ratatui::{
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{MemorySelectorContext, SettingsSnapshot};

use crate::ui::app::{Focus, TuiKitUi};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DefaultMemorySelectorLineKind {
    Title,
    Selected,
    CurrentDefault,
    Normal,
    Hint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DefaultMemorySelectorCopy<'a> {
    pub title: &'a str,
    pub hint: &'a str,
    pub show_current_default_marker: bool,
}

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_settings_screen(&self, area: ratatui::layout::Rect, buf: &mut Buffer) {
        let body_area = crate::ui::app::shared::layout::body_rect_for_area_with_tabs(
            area,
            !self.tab_specs.is_empty(),
        );
        let lines =
            settings_screen_lines_with_selection(self.settings_snapshot, self.list_selected);
        Paragraph::new(
            lines
                .into_iter()
                .map(|line| {
                    if line.starts_with("## ") {
                        Line::from(Span::styled(
                            line.trim_start_matches("## ").to_string(),
                            self.theme.style_accent_bold(),
                        ))
                    } else if line.starts_with("note: ") {
                        Line::from(Span::styled(
                            line.trim_start_matches("note: ").to_string(),
                            self.theme.style_muted(),
                        ))
                    } else if line.starts_with("› ") {
                        Line::from(Span::styled(line, self.theme.style_accent_bold()))
                    } else if line.is_empty() {
                        Line::from("")
                    } else {
                        Line::from(Span::styled(line, self.theme.style_normal()))
                    }
                })
                .collect::<Vec<_>>(),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if self.focus == Focus::Tabs {
                    self.theme.style_border()
                } else {
                    self.theme.style_border_focused()
                })
                .title(" Settings ")
                .style(Style::default().bg(self.theme.bg_panel)),
        )
        .wrap(Wrap { trim: false })
        .render(body_area, buf);
    }
}

pub(crate) fn default_memory_selector_lines(
    items: &[String],
    selected_index: usize,
    current_default_id: Option<&str>,
    copy: DefaultMemorySelectorCopy<'_>,
) -> Vec<(String, DefaultMemorySelectorLineKind)> {
    let mut lines = vec![
        (
            format!(" {} ", copy.title),
            DefaultMemorySelectorLineKind::Title,
        ),
        (String::new(), DefaultMemorySelectorLineKind::Normal),
    ];

    if items.is_empty() {
        lines.push((
            " No memories available yet.".to_string(),
            DefaultMemorySelectorLineKind::Normal,
        ));
    } else {
        for (index, item) in items.iter().enumerate() {
            let is_selected = index == selected_index;
            let is_default =
                copy.show_current_default_marker && current_default_id == Some(item.as_str());
            let prefix = if is_selected { "›" } else { " " };
            let suffix = if is_default { "  ★" } else { "" };
            let kind = if is_selected {
                DefaultMemorySelectorLineKind::Selected
            } else if is_default {
                DefaultMemorySelectorLineKind::CurrentDefault
            } else {
                DefaultMemorySelectorLineKind::Normal
            };
            lines.push((format!(" {prefix} {item}{suffix}"), kind));
        }
    }

    lines.push((String::new(), DefaultMemorySelectorLineKind::Normal));
    lines.push((copy.hint.to_string(), DefaultMemorySelectorLineKind::Hint));
    lines
}

pub(crate) fn default_memory_selector_copy(
    context: MemorySelectorContext,
) -> DefaultMemorySelectorCopy<'static> {
    match context {
        MemorySelectorContext::DefaultPreference => DefaultMemorySelectorCopy {
            title: "Select Default Memory",
            hint: " Enter: save  ↑/↓: move  Esc: close",
            show_current_default_marker: true,
        },
        MemorySelectorContext::InsertTarget => DefaultMemorySelectorCopy {
            title: "Select Target Memory",
            hint: " Enter: use target  ↑/↓: move  Esc: close",
            show_current_default_marker: false,
        },
    }
}

fn settings_screen_lines_with_selection(
    snapshot: Option<&SettingsSnapshot>,
    selected_entry_index: Option<usize>,
) -> Vec<String> {
    let mut lines = Vec::new();

    let Some(snapshot) = snapshot else {
        lines.push("No settings data available yet.".to_string());
        return lines;
    };

    let label_width = snapshot
        .sections
        .iter()
        .flat_map(|section| section.entries.iter())
        .map(|entry| entry.label.chars().count())
        .max()
        .unwrap_or(0);

    let mut flattened_index = 0usize;
    for section in &snapshot.sections {
        lines.push(format!("## {}", section.title));
        for entry in &section.entries {
            let prefix = if selected_entry_index == Some(flattened_index) {
                "›"
            } else {
                " "
            };
            lines.push(format!(
                "{prefix} {label:<width$}: {value}",
                label = entry.label,
                width = label_width,
                value = entry.value,
            ));
            if let Some(note) = &entry.note {
                lines.push(format!("note:   {note}"));
            }
            flattened_index += 1;
        }
        if let Some(footer) = &section.footer {
            lines.push(format!("note: {footer}"));
        }
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::{
        SETTINGS_ENTRY_DEFAULT_MEMORY_ID, SettingsEntry, SettingsSection, SettingsSnapshot,
    };

    #[test]
    fn settings_screen_lines_align_value_columns_across_sections() {
        let snapshot = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![
                SettingsSection {
                    title: "Current session".to_string(),
                    entries: vec![
                        SettingsEntry {
                            id: "auth".to_string(),
                            label: "Auth".to_string(),
                            value: "mock".to_string(),
                            note: None,
                        },
                        SettingsEntry {
                            id: "embedding_api_endpoint".to_string(),
                            label: "Embedding".to_string(),
                            value: "https://api.kinic.io".to_string(),
                            note: None,
                        },
                    ],
                    footer: None,
                },
                SettingsSection {
                    title: "Account".to_string(),
                    entries: vec![
                        SettingsEntry {
                            id: "principal_id".to_string(),
                            label: "Principal ID".to_string(),
                            value: "aaaaa-aa".to_string(),
                            note: None,
                        },
                        SettingsEntry {
                            id: "kinic_balance".to_string(),
                            label: "KINIC balance".to_string(),
                            value: "12.34000000 KINIC".to_string(),
                            note: None,
                        },
                    ],
                    footer: None,
                },
            ],
        };

        let lines = settings_screen_lines_with_selection(Some(&snapshot), None);
        let auth_line = lines
            .iter()
            .find(|line| line.contains("Auth"))
            .expect("auth line");
        let endpoint_line = lines
            .iter()
            .find(|line| line.contains("Embedding"))
            .expect("endpoint line");
        let principal_line = lines
            .iter()
            .find(|line| line.contains("Principal ID"))
            .expect("principal line");

        assert_eq!(
            auth_line.find("mock"),
            endpoint_line.find("https://api.kinic.io")
        );
        assert_eq!(auth_line.find(':'), principal_line.find(':'));
    }

    #[test]
    fn settings_screen_lines_marks_selected_entry() {
        let snapshot = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![SettingsSection {
                title: "Saved preferences".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                        label: "Default memory".to_string(),
                        value: "Alpha Memory".to_string(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "preferences_status".to_string(),
                        label: "Preferences status".to_string(),
                        value: "ok".to_string(),
                        note: None,
                    },
                ],
                footer: None,
            }],
        };

        let lines = settings_screen_lines_with_selection(Some(&snapshot), Some(1)).join("\n");

        assert!(lines.contains("› Preferences status: ok"));
        assert!(lines.contains("  Default memory"));
    }
    #[test]
    fn default_memory_selector_lines_mark_selected_and_current_entries() {
        let lines = default_memory_selector_lines(
            &["Alpha Memory".to_string(), "Beta Memory".to_string()],
            1,
            Some("Alpha Memory"),
            default_memory_selector_copy(MemorySelectorContext::DefaultPreference),
        );
        let joined = lines
            .into_iter()
            .map(|(line, _)| line)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("Alpha Memory  ★"));
        assert!(joined.contains("› Beta Memory"));
        assert!(joined.contains("Enter: save"));
        assert!(joined.contains("↑/↓: move"));
        assert!(!joined.contains("ID aaaaa-aa"));
        assert!(!joined.contains("j/k: move"));
    }

    #[test]
    fn insert_target_selector_lines_use_insert_specific_copy() {
        let lines = default_memory_selector_lines(
            &["Alpha Memory".to_string()],
            0,
            Some("Alpha Memory"),
            default_memory_selector_copy(MemorySelectorContext::InsertTarget),
        );
        let joined = lines
            .into_iter()
            .map(|(line, _)| line)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("Select Target Memory"));
        assert!(joined.contains("Enter: use target"));
        assert!(!joined.contains('★'));
    }
}
