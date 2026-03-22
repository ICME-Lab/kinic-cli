//! Inspector panel for displaying generic UI item details.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        block::BorderType, Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};

use crate::ui::model::UiItemDetail;
use crate::ui::theme::Theme;

/// Panel for inspecting item details with scrolling support.
pub struct InspectorPanel<'a> {
    detail: Option<&'a UiItemDetail>,
    theme: &'a Theme,
    focused: bool,
    scroll_offset: usize,
}

impl<'a> InspectorPanel<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            detail: None,
            theme,
            focused: false,
            scroll_offset: 0,
        }
    }

    pub fn ui_detail(mut self, detail: Option<&'a UiItemDetail>) -> Self {
        self.detail = detail;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    fn section_header(&self, title: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled("▸ ", self.theme.style_accent()),
            Span::styled(
                title.to_string(),
                self.theme.style_accent().add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ─────────────────", self.theme.style_muted()),
        ])
    }

    fn key_value(&self, key: &str, value: String) -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {} ", key), self.theme.style_dim()),
            Span::styled(value, self.theme.style_normal()),
        ])
    }

    fn render_empty(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.style_border())
            .style(Style::default().bg(self.theme.bg_panel))
            .title(" ◇ Content ");

        let inner = block.inner(area);
        block.render(area, buf);

        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled("  No item selected", self.theme.style_muted())),
            Line::from(""),
            Line::from("  Select an item from the list to view details."),
            Line::from(""),
            self.section_header("Navigation"),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("↑/↓ j/k", self.theme.style_accent()),
                Span::raw("  Navigate list"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Enter/→", self.theme.style_accent()),
                Span::raw("  Focus inspector"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("/", self.theme.style_accent()),
                Span::raw("        Search"),
            ]),
        ];

        Paragraph::new(help_text)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }

    fn render_detail(&self, detail: &UiItemDetail, area: Rect, buf: &mut Buffer) {
        let mut title_spans = Vec::new();
        if !detail.kind.label().is_empty() {
            title_spans.push(Span::styled(
                format!("{} ", detail.kind.label()),
                self.theme.style_keyword().add_modifier(Modifier::BOLD),
            ));
        }
        title_spans.push(Span::styled(
            detail.title.clone(),
            self.theme
                .style_accent_bold()
                .add_modifier(Modifier::UNDERLINED),
        ));

        let mut lines = vec![Line::from(title_spans)];

        if !detail.definition.trim().is_empty() {
            lines.push(Line::from(""));
            lines.push(self.section_header("Definition"));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(detail.definition.clone(), self.theme.style_type()),
            ]));
        }

        if let Some(loc) = &detail.location {
            lines.push(Line::from(""));
            lines.push(self.section_header("Source"));
            lines.push(Line::from(""));
            let mut location = String::new();
            if let Some(file) = &loc.file {
                location.push_str(file);
            }
            if let Some(line) = loc.line {
                location.push_str(&format!(":{}", line));
            }
            if !location.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  📍 "),
                    Span::styled(location, self.theme.style_muted()),
                ]));
            }
        }

        if !detail.badges.is_empty() {
            lines.push(Line::from(""));
            lines.push(self.section_header("Badges"));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(detail.badges.join(", "), self.theme.style_keyword()),
            ]));
        }

        for section in &detail.sections {
            lines.push(Line::from(""));
            lines.push(self.section_header(&section.heading));
            lines.push(Line::from(""));
            for row in &section.rows {
                lines.push(self.key_value(&format!("{}:", row.label), row.value.clone()));
            }
            for line in &section.body_lines {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(line.clone(), self.theme.style_normal()),
                ]));
            }
        }

        self.render_panel(" ◇ Content ", lines, area, buf);
    }

    fn render_panel(&self, title: &str, lines: Vec<Line<'static>>, area: Rect, buf: &mut Buffer) {
        let total_lines = lines.len();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.focused {
                self.theme.style_border_focused()
            } else {
                self.theme.style_border()
            })
            .style(Style::default().bg(self.theme.bg_panel))
            .title(title);

        let inner = block.inner(area);
        block.render(area, buf);

        let visible_lines: Vec<Line> = lines.into_iter().skip(self.scroll_offset).collect();

        Paragraph::new(visible_lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);

        if total_lines > inner.height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ScrollbarState::new(total_lines).position(self.scroll_offset);
            scrollbar.render(inner, buf, &mut scrollbar_state);
        }
    }
}

impl Widget for InspectorPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.detail {
            Some(detail) => self.render_detail(detail, area, buf),
            None => self.render_empty(area, buf),
        }
    }
}
