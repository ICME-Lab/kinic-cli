//! Overlay blocks: settings popup, help popup.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use tui_kit_runtime::CreateModalFocus;

use super::TuiKitUi;

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
                    if self.create_submitting {
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
        let w = 48.min(area.width.saturating_sub(4));
        let h = 10.min(area.height.saturating_sub(4));
        let settings_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(settings_area, buf);
        let cfg = &self.ui_config.settings;
        let text = vec![
            Line::from(Span::styled(
                format!(" {} ", cfg.title),
                self.theme.style_accent_bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                cfg.close_hint.clone(),
                self.theme.style_muted(),
            )),
        ];
        let block = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.style_border_focused())
                .title(format!(" {} ", cfg.title))
                .style(Style::default().bg(self.theme.bg_panel)),
        );
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
}
