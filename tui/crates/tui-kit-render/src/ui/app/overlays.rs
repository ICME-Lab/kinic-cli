//! Overlay blocks: settings, help, selectors, and create.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{CreateModalFocus, CreateSubmitState, SettingsSnapshot};

use super::TuiKitUi;
use super::screens::settings::{SelectorLineKind, selector_lines};

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

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", cfg.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .render(create_area, buf);
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
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", cfg.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(settings_area, buf);
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
        Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(format!(" {} ", cfg.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .render(help_area, buf);
    }

    pub(super) fn render_selector_overlay(&self, area: Rect, buf: &mut Buffer) {
        if self.selector_open {
            if self.selector_mode == tui_kit_runtime::SelectorMode::AddTag {
                self.render_selector_add_tag_overlay(area, buf);
                return;
            }

            let lines = selector_lines(
                self.selector_context,
                self.selector_mode,
                self.selector_items,
                self.selector_labels,
                self.selector_index,
                self.selector_selected_id,
            );
            let Some((w, h)) =
                fit_overlay_rect(area, 58, lines.len().saturating_add(2) as u16, 8)
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
            let text = visible_selector_lines(lines, h)
                .into_iter()
                .map(render_selector_line(self.theme))
                .collect::<Vec<_>>();
            Paragraph::new(text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(self.theme.style_border_focused())
                        .title(format!(" {} ", selector_title(self)))
                        .style(Style::default().bg(self.theme.bg_panel)),
                )
                .wrap(Wrap { trim: false })
                .render(picker_area, buf);
            return;
        }

    }

    fn render_selector_add_tag_overlay(&self, area: Rect, buf: &mut Buffer) {
        let w = 52.min(area.width.saturating_sub(4));
        let h = 9.min(area.height.saturating_sub(4));
        let add_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(add_area, buf);

        let input = if self.selector_add_tag_input.is_empty() {
            "<enter a new tag>".to_string()
        } else {
            self.selector_add_tag_input.to_string()
        };
        let lines = vec![
            Line::from(Span::styled(" Add tag ", self.theme.style_accent_bold())),
            Line::from(""),
            Line::from(Span::styled(format!("  {input}"), self.theme.style_accent_bold())),
            Line::from(""),
            Line::from(Span::styled(
                " Enter: save tag  Esc: return",
                self.theme.style_muted(),
            )),
        ];

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.style_border_focused())
                    .title(" Add tag ")
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(add_area, buf);
    }
}

fn render_selector_line<'a>(
    theme: &'a crate::ui::theme::Theme,
) -> impl Fn((String, SelectorLineKind)) -> Line<'static> + 'a {
    move |(line, kind)| match kind {
        SelectorLineKind::Selected => Line::from(Span::styled(line, theme.style_accent_bold())),
        SelectorLineKind::CurrentDefault => Line::from(Span::styled(line, theme.style_accent())),
        SelectorLineKind::Hint => Line::from(Span::styled(line, theme.style_muted())),
        SelectorLineKind::Normal => Line::from(Span::styled(line, theme.style_normal())),
        SelectorLineKind::Title => Line::from(Span::styled(line, theme.style_accent_bold())),
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

fn visible_selector_lines(
    lines: Vec<(String, SelectorLineKind)>,
    overlay_height: u16,
) -> Vec<(String, SelectorLineKind)> {
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
        .position(|(_, kind)| *kind == SelectorLineKind::Selected)
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

fn selector_title(ui: &TuiKitUi<'_>) -> &'static str {
    match (ui.selector_context, ui.selector_mode) {
        (_, tui_kit_runtime::SelectorMode::AddTag) => "Add tag",
        (tui_kit_runtime::SelectorContext::DefaultMemory, _) => "Select default memory",
        (tui_kit_runtime::SelectorContext::InsertTarget, _) => "Select insert target",
        (tui_kit_runtime::SelectorContext::InsertTag, _) => "Select insert tag",
        (tui_kit_runtime::SelectorContext::TagManagement, _) => "Saved tags",
    }
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
    use tui_kit_runtime::{SettingsEntry, SettingsSection, SettingsSnapshot};

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
        assert!(joined.contains("Item 7"));
        assert!(!joined.contains("Item 8"));
    }

    #[test]
    fn visible_selector_lines_keeps_selected_row() {
        let lines = vec![
            (" title ".to_string(), SelectorLineKind::Title),
            ("".to_string(), SelectorLineKind::Normal),
            (" a".to_string(), SelectorLineKind::Normal),
            (" b".to_string(), SelectorLineKind::Selected),
            ("".to_string(), SelectorLineKind::Normal),
            (" hint".to_string(), SelectorLineKind::Hint),
        ];
        let visible = visible_selector_lines(lines, 6);
        assert!(visible.iter().any(|(_, kind)| *kind == SelectorLineKind::Selected));
    }

    #[test]
    fn selector_lines_renders_titles() {
        let items = vec!["aaaaa-aa".to_string()];
        let labels = vec!["Alpha Memory".to_string()];
        let lines = selector_lines(
            tui_kit_runtime::SelectorContext::DefaultMemory,
            tui_kit_runtime::SelectorMode::List,
            &items,
            &labels,
            0,
            Some("aaaaa-aa"),
        );
        assert!(lines.iter().any(|(line, _)| line.contains("Alpha Memory")));
    }
}
