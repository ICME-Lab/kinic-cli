//! Placeholder bodies for non-interactive tabs.

use ratatui::{
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::ui::app::{Focus, TuiKitUi};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_placeholder_screen(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut Buffer,
        title: &str,
        lead: &str,
        detail: &str,
    ) {
        let body = vec![
            Line::from(""),
            Line::from(Span::styled(lead, self.theme.style_normal())),
            Line::from(""),
            Line::from(Span::styled(detail, self.theme.style_muted())),
            Line::from(""),
            Line::from(vec![
                Span::styled(" 1 ", self.theme.style_accent()),
                Span::styled("Memories", self.theme.style_muted()),
                Span::styled("  │  ", self.theme.style_dim()),
                Span::styled(" 2 ", self.theme.style_accent()),
                Span::styled("Create", self.theme.style_muted()),
            ]),
        ];
        Paragraph::new(body)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if self.focus == Focus::Tabs {
                        self.theme.style_border()
                    } else {
                        self.theme.style_border_focused()
                    })
                    .title(format!(" {} ", title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
