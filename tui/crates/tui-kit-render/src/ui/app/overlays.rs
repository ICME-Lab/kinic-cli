//! Overlay blocks: settings popup, help popup.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{CreateModalFocus, CreateSubmitState, SettingsSnapshot};

use super::TuiKitUi;
use super::screens::settings::{
    DefaultMemorySelectorLineKind, default_memory_selector_copy, default_memory_selector_lines,
};

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_create_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.show_create_modal {
            return;
        }
        let w = 64.min(area.width.saturating_sub(4));
        let h = 14.min(area.height.saturating_sub(4));
        let create_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(create_area, buf);
        let cfg = &self.ui_config.create;

        let name_style = if self.create_focus == CreateModalFocus::Name {
            self.theme.style_accent()
        } else {
            self.theme.style_normal()
        };
        let desc_style = if self.create_focus == CreateModalFocus::Description {
            self.theme.style_accent()
        } else {
            self.theme.style_normal()
        };
        let submit_style = if self.create_focus == CreateModalFocus::Submit {
            self.theme.style_accent_bold()
        } else {
            self.theme.style_normal()
        };

        let mut text = vec![
            Line::from(Span::styled(
                format!(" {} ", cfg.title),
                self.theme.style_accent_bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(cfg.name_label.clone(), self.theme.style_dim())),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.create_name.is_empty() {
                        "<enter a memory name>".to_string()
                    } else {
                        self.create_name.to_string()
                    },
                    name_style,
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                cfg.description_label.clone(),
                self.theme.style_dim(),
            )),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.create_description.is_empty() {
                        "<enter a short description>".to_string()
                    } else {
                        self.create_description.to_string()
                    },
                    desc_style,
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    if self.create_submit_state == CreateSubmitState::Submitting {
                        cfg.submit_pending_label.as_str()
                    } else {
                        cfg.submit_label.as_str()
                    },
                    submit_style,
                ),
            ]),
        ];
        if let Some(error) = self.create_error {
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                format!("  {error}"),
                self.theme.style_error(),
            )));
        }
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            cfg.close_hint.clone(),
            self.theme.style_muted(),
        )));

        let block = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.style_border_focused())
                .title(format!(" {} ", cfg.title))
                .style(Style::default().bg(self.theme.bg_panel)),
        );
        block.render(create_area, buf);
    }

    pub(super) fn render_settings_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.show_settings {
            return;
        }
        let cfg = &self.ui_config.settings;
        let lines = settings_overlay_lines(self.settings_snapshot, cfg);
        let Some((w, h)) = fit_overlay_rect(area, 68, lines.len().saturating_add(2) as u16, 10)
        else {
            return;
        };
        let settings_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(settings_area, buf);
        let text = lines
            .into_iter()
            .map(|line| {
                if line == cfg.close_hint {
                    Line::from(Span::styled(line, self.theme.style_muted()))
                } else if line.starts_with(' ') {
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(line.trim_start().to_string(), self.theme.style_normal()),
                    ])
                } else {
                    Line::from(Span::styled(line, self.theme.style_accent_bold()))
                }
            })
            .collect::<Vec<_>>();
        let block = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", cfg.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false });
        block.render(settings_area, buf);
    }

    pub(super) fn render_help_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.show_help {
            return;
        }
        let help_width = 62.min(area.width.saturating_sub(4));
        let help_height = 30.min(area.height.saturating_sub(4));
        let help_area = Rect {
            x: area.x + (area.width - help_width) / 2,
            y: area.y + (area.height - help_height) / 2,
            width: help_width,
            height: help_height,
        };
        Clear.render(help_area, buf);
        let cfg = &self.ui_config.help;
        let mut help_text = vec![
            Line::from(Span::styled(
                format!("⌨️  {}", cfg.title),
                self.theme.style_accent_bold(),
            )),
            Line::from(""),
        ];
        for line in &cfg.lines {
            help_text.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(line.clone(), self.theme.style_normal()),
            ]));
        }
        help_text.push(Line::from(""));
        help_text.push(Line::from(Span::styled(
            cfg.close_hint.clone(),
            self.theme.style_muted(),
        )));
        let help = Paragraph::new(help_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.style_border_focused())
                .title(format!(" {} ", cfg.title))
                .style(Style::default().bg(self.theme.bg_panel)),
        );
        help.render(help_area, buf);
    }

    pub(super) fn render_default_memory_selector_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.default_memory_selector_open {
            return;
        }

        let lines = default_memory_selector_lines(
            self.default_memory_selector_items,
            self.default_memory_selector_index,
            self.default_memory_selector_selected_id,
            default_memory_selector_copy(self.default_memory_selector_context),
        );
        let copy = default_memory_selector_copy(self.default_memory_selector_context);
        let Some((w, h)) = fit_overlay_rect(area, 56, lines.len().saturating_add(2) as u16, 8)
        else {
            return;
        };
        let picker_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(picker_area, buf);

        let text = visible_default_memory_selector_lines(lines, h)
            .into_iter()
            .map(|(line, kind)| match kind {
                DefaultMemorySelectorLineKind::Selected => {
                    Line::from(Span::styled(line, self.theme.style_accent_bold()))
                }
                DefaultMemorySelectorLineKind::CurrentDefault => {
                    Line::from(Span::styled(line, self.theme.style_accent()))
                }
                DefaultMemorySelectorLineKind::Hint => {
                    Line::from(Span::styled(line, self.theme.style_muted()))
                }
                DefaultMemorySelectorLineKind::Normal => {
                    Line::from(Span::styled(line, self.theme.style_normal()))
                }
                DefaultMemorySelectorLineKind::Title => {
                    Line::from(Span::styled(line, self.theme.style_accent_bold()))
                }
            })
            .collect::<Vec<_>>();

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", copy.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(picker_area, buf);
    }
}

fn fit_overlay_rect(
    area: Rect,
    desired_width: u16,
    desired_height: u16,
    min_height: u16,
) -> Option<(u16, u16)> {
    let width = desired_width.min(area.width.saturating_sub(4));
    let max_height = area.height.saturating_sub(4);
    if width == 0 || max_height == 0 {
        return None;
    }

    let height = if max_height < min_height {
        max_height
    } else {
        desired_height.clamp(min_height, max_height)
    };

    Some((width, height))
}

fn visible_default_memory_selector_lines(
    lines: Vec<(String, DefaultMemorySelectorLineKind)>,
    overlay_height: u16,
) -> Vec<(String, DefaultMemorySelectorLineKind)> {
    let fixed_prefix_len = 2usize;
    let fixed_suffix_len = 2usize;
    if lines.len() <= fixed_prefix_len + fixed_suffix_len {
        return lines;
    }

    let candidate_rows = overlay_height.saturating_sub(2) as usize;
    let visible_body_len = candidate_rows.saturating_sub(fixed_prefix_len + fixed_suffix_len);
    if visible_body_len == 0 {
        return lines;
    }

    let body_start = fixed_prefix_len;
    let body_end = lines.len().saturating_sub(fixed_suffix_len);
    let body = &lines[body_start..body_end];
    if body.len() <= visible_body_len {
        return lines;
    }

    let selected_in_body = body
        .iter()
        .position(|(_, kind)| *kind == DefaultMemorySelectorLineKind::Selected)
        .unwrap_or(0);
    let max_start = body.len().saturating_sub(visible_body_len);
    let start = selected_in_body
        .saturating_sub(visible_body_len / 2)
        .min(max_start);
    let end = start + visible_body_len;

    let mut visible = Vec::with_capacity(fixed_prefix_len + visible_body_len + fixed_suffix_len);
    visible.extend_from_slice(&lines[..fixed_prefix_len]);
    visible.extend_from_slice(&body[start..end]);
    visible.extend_from_slice(&lines[body_end..]);
    visible
}

fn settings_overlay_lines(
    snapshot: Option<&SettingsSnapshot>,
    cfg: &super::types::SettingsOverlayText,
) -> Vec<String> {
    let mut lines = vec![format!(" {} ", cfg.title), String::new()];
    if let Some(snapshot) = snapshot {
        if snapshot.quick_entries.is_empty() {
            lines.push(" No settings available yet.".to_string());
        } else {
            let visible_entries = snapshot.quick_entries.iter().take(7).collect::<Vec<_>>();
            let label_width = visible_entries
                .iter()
                .map(|entry| entry.label.chars().count())
                .max()
                .unwrap_or(0);
            for entry in visible_entries {
                lines.push(format!(
                    " {label:<width$}: {value}",
                    label = entry.label,
                    width = label_width,
                    value = entry.value,
                ));
            }
        }
    } else {
        lines.push(" No settings available yet.".to_string());
    }
    lines.push(String::new());
    lines.push(cfg.close_hint.clone());
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::UiConfig;
    use tui_kit_runtime::{
        MemorySelectorContext, SettingsEntry, SettingsSection, SettingsSnapshot,
    };

    #[test]
    fn settings_overlay_lines_show_up_to_seven_entries() {
        let cfg = UiConfig::default().settings;
        let snapshot = SettingsSnapshot {
            quick_entries: (1..=8)
                .map(|index| SettingsEntry {
                    id: format!("item_{index}"),
                    label: format!("Item {index}"),
                    value: format!("Value {index}"),
                    note: None,
                })
                .collect(),
            sections: vec![SettingsSection::default()],
        };

        let joined = settings_overlay_lines(Some(&snapshot), &cfg).join("\n");

        assert!(!joined.contains("Open the Settings tab for detailed view."));
        assert!(joined.contains("Item 7: Value 7"));
        assert!(!joined.contains("Item 8: Value 8"));
    }

    #[test]
    fn settings_overlay_lines_align_quick_entry_columns() {
        let cfg = UiConfig::default().settings;
        let snapshot = SettingsSnapshot {
            quick_entries: vec![
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
            sections: vec![SettingsSection::default()],
        };

        let lines = settings_overlay_lines(Some(&snapshot), &cfg);
        let auth_line = lines
            .iter()
            .find(|line| line.contains("Auth"))
            .expect("auth line");
        let embedding_line = lines
            .iter()
            .find(|line| line.contains("Embedding"))
            .expect("embedding line");

        assert_eq!(auth_line.find(':'), embedding_line.find(':'));
    }

    #[test]
    fn fit_overlay_rect_uses_available_height_when_terminal_is_short() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 11,
        };

        let rect = fit_overlay_rect(area, 56, 12, 8);

        assert_eq!(rect, Some((56, 7)));
    }

    #[test]
    fn fit_overlay_rect_returns_none_when_inner_area_is_unavailable() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 3,
            height: 3,
        };

        let rect = fit_overlay_rect(area, 56, 12, 8);

        assert_eq!(rect, None);
    }

    #[test]
    fn default_memory_selector_window_keeps_selected_row_visible_near_end() {
        let lines = default_memory_selector_lines(
            &(0..12)
                .map(|index| format!("id-{index}"))
                .collect::<Vec<_>>(),
            &(0..12)
                .map(|index| format!("Memory {index}"))
                .collect::<Vec<_>>(),
            10,
            Some("id-2"),
            default_memory_selector_copy(MemorySelectorContext::DefaultPreference),
        );

        let visible = visible_default_memory_selector_lines(lines, 8);
        let visible_lines = visible
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<_>>();
        let joined = visible_lines.join("\n");

        assert!(joined.contains("Memory 10"));
        assert!(!visible_lines.iter().any(|line| *line == "  Memory 1"));
        assert!(joined.contains("Enter: save"));
    }

    #[test]
    fn default_memory_selector_window_keeps_selected_row_visible_near_start() {
        let lines = default_memory_selector_lines(
            &(0..12)
                .map(|index| format!("id-{index}"))
                .collect::<Vec<_>>(),
            &(0..12)
                .map(|index| format!("Memory {index}"))
                .collect::<Vec<_>>(),
            1,
            Some("id-9"),
            default_memory_selector_copy(MemorySelectorContext::DefaultPreference),
        );
        let near_start_joined = near_start
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(near_start_joined.contains("Memory 1"));
        assert!(!near_start_joined.contains("Memory 10"));
        assert!(near_start_joined.contains("Select Default Memory"));
    }

    #[test]
    fn insert_target_selector_window_uses_insert_specific_copy() {
        let lines = default_memory_selector_lines(
            &["id-0".to_string()],
            &["Memory 0".to_string()],
            0,
            Some("id-0"),
            default_memory_selector_copy(MemorySelectorContext::InsertTarget),
        );

        let visible = visible_default_memory_selector_lines(lines, 8);
        let joined = visible
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("Select Target Memory"));
        assert!(joined.contains("Enter: use target"));
        assert!(!joined.contains('★'));
    }
}
