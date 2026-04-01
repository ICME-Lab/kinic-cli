//! Settings screen rendering and selector helper copy.

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
        (
            format!(" {} ", selector_title(context, mode)),
            SelectorLineKind::Title,
        ),
        (String::new(), SelectorLineKind::Normal),
    ];

    if items.is_empty() {
        lines.push((
            selector_empty_message(context).to_string(),
            SelectorLineKind::Normal,
        ));
    } else {
        for (index, item) in items.iter().enumerate() {
            let label = labels.get(index).unwrap_or(item);
            let is_selected = index == selected_index;
            let is_default = current_default_id == Some(item.as_str());
            let prefix = if is_selected { "›" } else { " " };
            let suffix = if is_default { "  ★" } else { "" };
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
            SelectorContext::InsertTarget => " Enter: choose  ↑/↓: move  Esc: close",
            SelectorContext::InsertTag => " Enter: choose  ↑/↓: move  Esc: close",
            SelectorContext::TagManagement => " Enter: add/select  ↑/↓: move  Esc: close",
        },
    }
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
            sections: vec![SettingsSection {
                title: "Saved preferences".to_string(),
                entries: vec![
                    SettingsEntry {
                        id: SETTINGS_ENTRY_DEFAULT_MEMORY_ID.to_string(),
                        label: "Default memory".to_string(),
                        value: "aaaaa-aa".to_string(),
                        note: None,
                    },
                    SettingsEntry {
                        id: "saved_tags".to_string(),
                        label: "Saved tags".to_string(),
                        value: "docs".to_string(),
                        note: None,
                    },
                ],
                footer: None,
            }],
        };

        let lines = settings_screen_lines_with_selection(Some(&snapshot), Some(1));
        assert!(lines.iter().any(|line| line.contains("Default memory")));
        assert!(lines.iter().any(|line| line.contains("Saved tags")));
        assert!(lines.iter().any(|line| line.starts_with("› ")));
    }

    #[test]
    fn selector_lines_add_add_row_for_tag_management() {
        let lines = selector_lines(
            SelectorContext::TagManagement,
            SelectorMode::List,
            &["docs".to_string()],
            &["docs".to_string()],
            1,
            None,
        );

        assert!(lines.iter().any(|(line, _)| line.contains("+ Add new tag")));
    }

    #[test]
    fn selector_lines_mark_current_default() {
        let items = vec!["aaaaa-aa".to_string()];
        let labels = vec!["Alpha Memory".to_string()];
        let lines = selector_lines(
            SelectorContext::DefaultMemory,
            SelectorMode::List,
            &items,
            &labels,
            0,
            Some("aaaaa-aa"),
        );

        assert!(lines.iter().any(|(line, _)| line.contains('★')));
    }
}
