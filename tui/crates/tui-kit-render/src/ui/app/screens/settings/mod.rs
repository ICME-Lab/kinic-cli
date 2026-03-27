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
use tui_kit_runtime::{SelectorContext, SelectorMode, SettingsSnapshot};

use crate::ui::app::{Focus, TuiKitUi};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorLineKind {
    Title,
    Selected,
    CurrentDefault,
    Normal,
    Hint,
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

pub(crate) fn selector_lines(
    context: SelectorContext,
    mode: SelectorMode,
    items: &[String],
    labels: &[String],
    selected_index: usize,
    current_default_id: Option<&str>,
) -> Vec<(String, SelectorLineKind)> {
    let mut lines = vec![
        (format!(" {} ", selector_title(context, mode)), SelectorLineKind::Title),
        (String::new(), SelectorLineKind::Normal),
    ];

    if items.is_empty() {
        lines.push((
            selector_empty_message(context).to_string(),
            SelectorLineKind::Normal,
        ));
    } else {
        for (index, item) in items.iter().enumerate() {
            let is_selected = index == selected_index;
            let is_default = current_default_id == Some(item.as_str());
            let prefix = if is_selected { "›" } else { " " };
            let suffix = if is_default { "  ★" } else { "" };
            let label = labels.get(index).map_or(item.as_str(), String::as_str);
            let kind = if is_selected {
                SelectorLineKind::Selected
            } else if is_default {
                SelectorLineKind::CurrentDefault
            } else {
                SelectorLineKind::Normal
            };
            lines.push((format!(" {prefix} {label}{suffix}"), kind));
        }
    }

    if selector_has_add_row(context, mode) {
        lines.push((String::new(), SelectorLineKind::Normal));
        lines.push((
            " + Add new tag".to_string(),
            if selected_index == items.len() {
                SelectorLineKind::Selected
            } else {
                SelectorLineKind::Normal
            },
        ));
    }

    lines.push((String::new(), SelectorLineKind::Normal));
    lines.push((
        selector_hint(context, mode).to_string(),
        SelectorLineKind::Hint,
    ));
    lines
}

#[cfg(test)]
pub(crate) fn default_memory_selector_lines(
    items: &[String],
    labels: &[String],
    selected_index: usize,
    current_default_id: Option<&str>,
) -> Vec<(String, SelectorLineKind)> {
    selector_lines(
        SelectorContext::DefaultMemory,
        SelectorMode::List,
        items,
        labels,
        selected_index,
        current_default_id,
    )
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

    let mut flattened_index = 0usize;
    for section in &snapshot.sections {
        lines.push(format!("## {}", section.title));
        let label_width = section
            .entries
            .iter()
            .map(|entry| entry.label.chars().count())
            .max()
            .unwrap_or(0);
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

fn selector_title(context: SelectorContext, mode: SelectorMode) -> &'static str {
    match (context, mode) {
        (_, SelectorMode::AddTag) => "Add tag",
        (SelectorContext::DefaultMemory, SelectorMode::List) => "Select default memory",
        (SelectorContext::InsertTarget, SelectorMode::List) => "Select insert target",
        (SelectorContext::InsertTag, SelectorMode::List) => "Select insert tag",
        (SelectorContext::TagManagement, SelectorMode::List) => "Saved tags",
    }
}

fn selector_empty_message(context: SelectorContext) -> &'static str {
    match context {
        SelectorContext::DefaultMemory | SelectorContext::InsertTarget => {
            " No memories available yet."
        }
        SelectorContext::InsertTag | SelectorContext::TagManagement => " No saved tags yet.",
    }
}

fn selector_has_add_row(context: SelectorContext, mode: SelectorMode) -> bool {
    mode == SelectorMode::List
        && matches!(
            context,
            SelectorContext::InsertTag | SelectorContext::TagManagement
        )
}

fn selector_hint(context: SelectorContext, mode: SelectorMode) -> &'static str {
    match mode {
        SelectorMode::AddTag => " Enter: save tag  Esc: return",
        SelectorMode::List => match context {
            SelectorContext::DefaultMemory => " Enter: save  ↑/↓: move  Esc: close",
            SelectorContext::InsertTarget => " Enter: save  ↑/↓: move  Esc: close",
            SelectorContext::InsertTag => " Enter: choose  ↑/↓: move  Esc: close",
            SelectorContext::TagManagement => " Enter: add/select  ↑/↓: move  Esc: close",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::{SettingsEntry, SettingsSection, SettingsSnapshot};

    #[test]
    fn settings_screen_lines_show_sections_and_status_notes() {
        let snapshot = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![
                SettingsSection {
                    title: "Current session".to_string(),
                    entries: vec![SettingsEntry {
                        id: "principal_id".to_string(),
                        label: "Principal ID".to_string(),
                        value: "unavailable".to_string(),
                        note: None,
                    }],
                    footer: None,
                },
                SettingsSection {
                    title: "Saved preferences".to_string(),
                    entries: vec![SettingsEntry {
                        id: "preferred_network".to_string(),
                        label: "Preferred network".to_string(),
                        value: "coming soon".to_string(),
                        note: Some("No persisted network preference is stored in v1.".to_string()),
                    }],
                    footer: None,
                },
            ],
        };

        let lines = settings_screen_lines_with_selection(Some(&snapshot), None).join("\n");

        assert!(lines.contains("## Current session"));
        assert!(lines.contains("Principal ID: unavailable"));
        assert!(!lines.contains("Shift+S shows quick status"));
    }

    #[test]
    fn settings_screen_lines_align_value_columns_within_section() {
        let snapshot = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![SettingsSection {
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
                        label: "Embedding API endpoint".to_string(),
                        value: "https://api.kinic.io".to_string(),
                        note: None,
                    },
                ],
                footer: None,
            }],
        };

        let lines = settings_screen_lines_with_selection(Some(&snapshot), None);
        let auth_line = lines
            .iter()
            .find(|line| line.contains("Auth"))
            .expect("auth line");
        let endpoint_line = lines
            .iter()
            .find(|line| line.contains("Embedding API endpoint"))
            .expect("endpoint line");

        assert_eq!(
            auth_line.find("mock"),
            endpoint_line.find("https://api.kinic.io")
        );
    }

    #[test]
    fn settings_screen_lines_marks_selected_entry() {
        let snapshot = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![SettingsSection {
                title: "Saved preferences".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: "default_memory".to_string(),
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
            &["aaaaa-aa".to_string(), "bbbbb-bb".to_string()],
            &["Alpha Memory".to_string(), "Beta Memory".to_string()],
            1,
            Some("aaaaa-aa"),
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
}
