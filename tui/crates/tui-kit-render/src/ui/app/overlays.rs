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
use super::screens::settings::{DefaultMemorySelectorLineKind, default_memory_selector_lines};

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
                if line == cfg.content_hint || line == cfg.close_hint {
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
            self.default_memory_selector_labels,
            self.default_memory_selector_index,
            self.default_memory_selector_selected_id,
        );
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

        let text = lines
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
                    .title(" Select Default Memory ")
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

fn settings_overlay_lines(
    snapshot: Option<&SettingsSnapshot>,
    cfg: &super::types::SettingsOverlayText,
) -> Vec<String> {
    let mut lines = vec![format!(" {} ", cfg.title), String::new()];
    if let Some(snapshot) = snapshot {
        if snapshot.quick_entries.is_empty() {
            lines.push(" No settings available yet.".to_string());
        } else {
            for entry in snapshot.quick_entries.iter().take(7) {
                lines.push(format!(" {}: {}", entry.label, entry.value));
            }
        }
    } else {
        lines.push(" No settings available yet.".to_string());
    }
    lines.push(String::new());
    lines.push(cfg.content_hint.clone());
    lines.push(cfg.close_hint.clone());
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::UiConfig;
    use tui_kit_runtime::{SettingsEntry, SettingsSection, SettingsSnapshot};

    #[test]
    fn settings_overlay_lines_include_priority_entries() {
        let cfg = UiConfig::default().settings;
        let snapshot = SettingsSnapshot {
            quick_entries: vec![
                SettingsEntry {
                    id: "identity_name".to_string(),
                    label: "Identity name".to_string(),
                    value: "alice".to_string(),
                    note: None,
                },
                SettingsEntry {
                    id: "principal_id".to_string(),
                    label: "Principal ID".to_string(),
                    value: "aaaaa-aa".to_string(),
                    note: None,
                },
                SettingsEntry {
                    id: "network".to_string(),
                    label: "Network".to_string(),
                    value: "local".to_string(),
                    note: None,
                },
            ],
            sections: vec![SettingsSection::default()],
        };

        let lines = settings_overlay_lines(Some(&snapshot), &cfg);
        let joined = lines.join("\n");

        assert!(joined.contains("Identity name: alice"));
        assert!(joined.contains("Principal ID: aaaaa-aa"));
        assert!(joined.contains("Network: local"));
        assert!(joined.contains("Open the Settings tab for detailed view."));
        assert!(!joined.contains("Quick status while the main UI stays interactive."));
        assert!(!joined.contains("More details are available in the Settings tab."));
    }

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

        assert!(joined.contains("Item 7: Value 7"));
        assert!(!joined.contains("Item 8: Value 8"));
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
}
