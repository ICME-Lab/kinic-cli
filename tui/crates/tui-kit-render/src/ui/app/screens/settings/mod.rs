//! Settings screen rendering and picker copy helpers.

use ratatui::{
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{
    PickerConfirmKind, PickerContext, PickerItem, PickerListMode, PickerState, SettingsSnapshot,
};

use crate::ui::app::{Focus, TuiKitUi};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickerLineKind {
    Selected,
    CurrentDefault,
    Normal,
    Hint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PickerPresentation<'a> {
    List {
        context: PickerContext,
        items: &'a [PickerItem],
        selected_index: usize,
    },
    Confirm {
        kind: &'a PickerConfirmKind,
    },
    Input {
        context: PickerContext,
        value: &'a str,
    },
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

pub(crate) fn picker_presentation(picker: &PickerState) -> Option<PickerPresentation<'_>> {
    match picker {
        PickerState::List {
            context,
            items,
            selected_index,
            mode,
            ..
        } => match mode {
            PickerListMode::Browsing => Some(PickerPresentation::List {
                context: *context,
                items,
                selected_index: *selected_index,
            }),
            PickerListMode::Confirm { kind } => Some(PickerPresentation::Confirm { kind }),
        },
        PickerState::Input { context, value, .. } => Some(PickerPresentation::Input {
            context: *context,
            value,
        }),
        PickerState::Closed => None,
    }
}


pub(crate) fn picker_lines(presentation: &PickerPresentation<'_>) -> Vec<(String, PickerLineKind)> {
    match presentation {
        PickerPresentation::Confirm { kind } => confirm_lines(kind),
        PickerPresentation::Input { context, value } => {
            let input = if value.is_empty() {
                picker_input_placeholder(*context).to_string()
            } else {
                format!("  {value}")
            };
            vec![
                (String::new(), PickerLineKind::Normal),
                (input, PickerLineKind::Selected),
                (String::new(), PickerLineKind::Normal),
                (picker_hint(*context).to_string(), PickerLineKind::Hint),
            ]
        }
        PickerPresentation::List {
            context,
            items,
            selected_index,
        } => {
            let mut lines = Vec::new();

            if items.is_empty() {
                lines.push((
                    picker_empty_message(*context).to_string(),
                    PickerLineKind::Normal,
                ));
            } else {
                for (index, item) in items.iter().enumerate() {
                    let is_selected = index == *selected_index;
                    let suffix = if item.is_current_default && show_current_default_marker(*context)
                    {
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
            lines.push((picker_hint(*context).to_string(), PickerLineKind::Hint));
            lines
        }
    }
}

pub(crate) fn picker_title(presentation: &PickerPresentation<'_>) -> &'static str {
    match presentation {
        PickerPresentation::Confirm { kind } => confirm_title(kind),
        PickerPresentation::List { context, .. } | PickerPresentation::Input { context, .. } => {
            picker_context_title(*context)
        }
    }
}

pub(crate) fn picker_requires_vertical_centering(presentation: &PickerPresentation<'_>) -> bool {
    matches!(presentation, PickerPresentation::Confirm { .. })
}

pub(crate) fn picker_hint(context: PickerContext) -> &'static str {
    match context {
        PickerContext::DefaultMemory => " Enter: save  ↑/↓: move  Esc: close",
        PickerContext::InsertTarget => " Enter: use target  ↑/↓: move  Esc: close",
        PickerContext::InsertTag => " Enter: choose  ↑/↓: move  Esc: close",
        PickerContext::TagManagement => " Enter: use  d: delete  ↑/↓: browse  Esc: close",
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
    matches!(
        context,
        PickerContext::DefaultMemory | PickerContext::InsertTag | PickerContext::TagManagement
    )
}

fn picker_context_title(context: PickerContext) -> &'static str {
    match context {
        PickerContext::DefaultMemory => "Select default memory",
        PickerContext::InsertTarget => "Select target memory",
        PickerContext::InsertTag => "Select insert tag",
        PickerContext::TagManagement => "Saved tags",
        PickerContext::AddTag => "Add tag",
    }
}

fn confirm_title(kind: &PickerConfirmKind) -> &'static str {
    match kind {
        PickerConfirmKind::DeleteTag { .. } => "Delete saved tag",
    }
}

fn confirm_lines(kind: &PickerConfirmKind) -> Vec<(String, PickerLineKind)> {
    match kind {
        PickerConfirmKind::DeleteTag { tag_id } => vec![
            (String::new(), PickerLineKind::Normal),
            (
                format!(" Delete tag \"{tag_id}\"?"),
                PickerLineKind::Selected,
            ),
            (String::new(), PickerLineKind::Normal),
            (
                " Enter: confirm  Esc: cancel".to_string(),
                PickerLineKind::Hint,
            ),
        ],
    }
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
        PickerConfirmKind, SETTINGS_ENTRY_DEFAULT_MEMORY_ID, SettingsEntry, SettingsSection,
        SettingsSnapshot,
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
        let lines = picker_lines(&PickerPresentation::List {
            context: PickerContext::TagManagement,
            items: &[
                PickerItem::option("docs", "docs", false),
                PickerItem::add_action("+ Add new tag"),
            ],
            selected_index: 1,
        });

        assert!(lines.iter().any(|(line, _)| line.contains("+ Add new tag")));
    }

    #[test]
    fn picker_lines_mark_current_default_only_for_default_memory() {
        let default_lines = picker_lines(&PickerPresentation::List {
            context: PickerContext::DefaultMemory,
            items: &[PickerItem::option("aaaaa-aa", "Alpha Memory", true)],
            selected_index: 0,
        });
        let insert_lines = picker_lines(&PickerPresentation::List {
            context: PickerContext::InsertTarget,
            items: &[PickerItem::option("aaaaa-aa", "Alpha Memory", true)],
            selected_index: 0,
        });

        assert!(default_lines.iter().any(|(line, _)| line.contains('★')));
        assert!(!insert_lines.iter().any(|(line, _)| line.contains('★')));
    }

    #[test]
    fn tag_management_hint_describes_add_only_behavior() {
        assert_eq!(
            picker_hint(PickerContext::TagManagement),
            " Enter: use  d: delete  ↑/↓: browse  Esc: close"
        );
    }

    #[test]
    fn tag_picker_titles_differ_by_context() {
        assert_eq!(
            picker_title(&PickerPresentation::List {
                context: PickerContext::InsertTag,
                items: &[],
                selected_index: 0,
            }),
            "Select insert tag"
        );
        assert_eq!(
            picker_title(&PickerPresentation::List {
                context: PickerContext::TagManagement,
                items: &[],
                selected_index: 0,
            }),
            "Saved tags"
        );
    }

    #[test]
    fn tag_management_confirm_lines_show_delete_prompt() {
        let lines = picker_lines(&PickerPresentation::Confirm {
            kind: &PickerConfirmKind::DeleteTag {
                tag_id: "docs".to_string(),
            },
        });

        assert!(
            lines
                .iter()
                .any(|(line, _)| line.contains("Delete tag \"docs\"?"))
        );
        assert!(
            lines
                .iter()
                .any(|(line, _)| line.contains("Enter: confirm  Esc: cancel"))
        );
    }

    #[test]
    fn picker_presentation_maps_confirm_delete_mode() {
        let picker = PickerState::List {
            context: PickerContext::TagManagement,
            items: vec![PickerItem::option("docs", "docs", false)],
            selected_index: 0,
            selected_id: Some("docs".to_string()),
            mode: PickerListMode::Confirm {
                kind: PickerConfirmKind::DeleteTag {
                    tag_id: "docs".to_string(),
                },
            },
        };

        assert_eq!(
            picker_presentation(&picker),
            Some(PickerPresentation::Confirm {
                kind: &PickerConfirmKind::DeleteTag {
                    tag_id: "docs".to_string(),
                },
            })
        );
    }

    #[test]
    fn picker_presentation_maps_input_picker() {
        let picker = PickerState::Input {
            context: PickerContext::AddTag,
            origin_context: Some(PickerContext::InsertTag),
            value: "research".to_string(),
        };

        assert_eq!(
            picker_presentation(&picker),
            Some(PickerPresentation::Input {
                context: PickerContext::AddTag,
                value: "research",
            })
        );
    }
}
