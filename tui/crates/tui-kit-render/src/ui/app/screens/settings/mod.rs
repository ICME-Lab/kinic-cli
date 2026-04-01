//! Settings screen rendering and picker copy helpers.

use ratatui::{
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{PickerContext, PickerItem, SettingsSnapshot};

use crate::ui::app::{Focus, TuiKitUi};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickerLineKind {
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

pub(crate) fn picker_lines(
    context: PickerContext,
    items: &[PickerItem],
    selected_index: usize,
) -> Vec<(String, PickerLineKind)> {
    let mut lines = Vec::new();

    if items.is_empty() {
        lines.push((
            picker_empty_message(context).to_string(),
            PickerLineKind::Normal,
        ));
    } else {
        for (index, item) in items.iter().enumerate() {
            let is_selected = index == selected_index;
            let suffix = if item.is_current_default && show_current_default_marker(context) {
                "  ★"
            } else {
                ""
            };
            let prefix = if is_selected { "›" } else { " " };
            let kind = if is_selected {
                PickerLineKind::Selected
            } else if item.is_current_default {
                PickerLineKind::CurrentDefault
            } else {
                PickerLineKind::Normal
            };
            lines.push((format!(" {prefix} {}{suffix}", item.label), kind));
        }
    }

    lines.push((String::new(), PickerLineKind::Normal));
    lines.push((picker_hint(context).to_string(), PickerLineKind::Hint));
    lines
}

pub(crate) fn picker_title(context: PickerContext) -> &'static str {
    match context {
        PickerContext::DefaultMemory => "Select default memory",
        PickerContext::InsertTarget => "Select target memory",
        PickerContext::InsertTag => "Select insert tag",
        PickerContext::TagManagement => "Saved tags",
        PickerContext::AddTag => "Add tag",
    }
}

pub(crate) fn picker_hint(context: PickerContext) -> &'static str {
    match context {
        PickerContext::DefaultMemory => " Enter: save  ↑/↓: move  Esc: close",
        PickerContext::InsertTarget => " Enter: use target  ↑/↓: move  Esc: close",
        PickerContext::InsertTag => " Enter: choose  ↑/↓: move  Esc: close",
        PickerContext::TagManagement => " Enter: add/select  ↑/↓: move  Esc: close",
        PickerContext::AddTag => " Enter: save tag  Esc: close",
    }
}

pub(crate) fn picker_input_placeholder(context: PickerContext) -> &'static str {
    match context {
        PickerContext::AddTag => "<enter a new tag>",
        PickerContext::DefaultMemory
        | PickerContext::InsertTarget
        | PickerContext::InsertTag
        | PickerContext::TagManagement => "",
    }
}

fn show_current_default_marker(context: PickerContext) -> bool {
    matches!(context, PickerContext::DefaultMemory)
}

fn picker_empty_message(context: PickerContext) -> &'static str {
    match context {
        PickerContext::DefaultMemory | PickerContext::InsertTarget => " No memories available yet.",
        PickerContext::InsertTag | PickerContext::TagManagement | PickerContext::AddTag => {
            " No saved tags yet."
        }
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
    fn picker_lines_include_add_action_rows() {
        let lines = picker_lines(
            PickerContext::TagManagement,
            &[
                PickerItem::option("docs", "docs", false),
                PickerItem::add_action("+ Add new tag"),
            ],
            1,
        );

        assert!(lines.iter().any(|(line, _)| line.contains("+ Add new tag")));
    }

    #[test]
    fn picker_lines_mark_current_default_only_for_default_memory() {
        let default_lines = picker_lines(
            PickerContext::DefaultMemory,
            &[PickerItem::option("aaaaa-aa", "Alpha Memory", true)],
            0,
        );
        let insert_lines = picker_lines(
            PickerContext::InsertTarget,
            &[PickerItem::option("aaaaa-aa", "Alpha Memory", true)],
            0,
        );

        assert!(default_lines.iter().any(|(line, _)| line.contains('★')));
        assert!(!insert_lines.iter().any(|(line, _)| line.contains('★')));
    }
}
