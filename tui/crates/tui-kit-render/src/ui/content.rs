//! Content panel for displaying generic UI item content.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget, Wrap, block::BorderType,
    },
};
use unicode_width::UnicodeWidthStr;

use crate::ui::model::UiItemContent;
use crate::ui::theme::Theme;

/// Panel for rendering item content with scrolling support.
pub struct ContentPanel<'a> {
    content: Option<&'a UiItemContent>,
    theme: &'a Theme,
    focused: bool,
    scroll_offset: usize,
}

impl<'a> ContentPanel<'a> {
    const SECTION_HEADER_WIDTH: usize = 34;

    pub fn new(theme: &'a Theme) -> Self {
        Self {
            content: None,
            theme,
            focused: false,
            scroll_offset: 0,
        }
    }

    pub fn ui_content(mut self, content: Option<&'a UiItemContent>) -> Self {
        self.content = content;
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
        let title_width = UnicodeWidthStr::width(title);
        let dash_count = Self::SECTION_HEADER_WIDTH.saturating_sub(title_width).max(1);
        Line::from(vec![
            Span::styled("▸ ", self.theme.style_accent()),
            Span::styled(
                title.to_string(),
                self.theme.style_accent().add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {}", "─".repeat(dash_count)), self.theme.style_muted()),
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
            Line::from("  Select an item from Items to view content."),
            Line::from(""),
            self.section_header("Navigation"),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("↑/↓", self.theme.style_accent()),
                Span::raw("  Navigate items"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Enter/→", self.theme.style_accent()),
                Span::raw("  Focus content"),
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

    fn render_content(&self, content: &UiItemContent, area: Rect, buf: &mut Buffer) {
        let mut lines = Vec::new();
        if !is_memory_item(content) {
            let mut title_spans = Vec::new();
            if !content.kind.label().is_empty() {
                title_spans.push(Span::styled(
                    format!("{} ", content.kind.label()),
                    self.theme.style_keyword().add_modifier(Modifier::BOLD),
                ));
            }
            title_spans.push(Span::styled(
                content.title.clone(),
                self.theme
                    .style_accent_bold()
                    .add_modifier(Modifier::UNDERLINED),
            ));
            lines.push(Line::from(title_spans));
        } else if let Some(subtitle) = &content.subtitle {
            lines.push(self.section_header("Description"));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(subtitle.clone(), self.theme.style_muted()),
            ]));
        }

        if !content.definition.trim().is_empty() {
            lines.push(Line::from(""));
            lines.push(self.section_header("Definition"));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(content.definition.clone(), self.theme.style_type()),
            ]));
        }

        if let Some(loc) = &content.location {
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

        if !content.badges.is_empty() {
            lines.push(Line::from(""));
            lines.push(self.section_header("Badges"));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(content.badges.join(", "), self.theme.style_keyword()),
            ]));
        }

        for section in &content.sections {
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

        self.render_panel(content_panel_title(self.theme, content), lines, area, buf);
    }

    fn render_panel(
        &self,
        title: Line<'static>,
        lines: Vec<Line<'static>>,
        area: Rect,
        buf: &mut Buffer,
    ) {
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

fn content_panel_title(theme: &Theme, content: &UiItemContent) -> Line<'static> {
    match &content.kind {
        crate::ui::model::UiItemKind::Custom(kind) if kind == "memory" => {
            Line::from(vec![
                Span::styled(" ◇ ", theme.style_accent()),
                Span::styled(content.title.clone(), theme.style_accent_bold()),
                Span::styled(" : ", theme.style_muted()),
                Span::styled(content.id.clone(), theme.style_accent()),
                Span::styled(" ", theme.style_accent()),
            ])
        }
        _ => Line::from(vec![
            Span::styled(" ◇ ", theme.style_accent()),
            Span::styled("Content", theme.style_accent_bold()),
            Span::styled(" ", theme.style_accent()),
        ]),
    }
}

fn is_memory_item(content: &UiItemContent) -> bool {
    matches!(&content.kind, crate::ui::model::UiItemKind::Custom(kind) if kind == "memory")
}

impl Widget for ContentPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.content {
            Some(content) => self.render_content(content, area, buf),
            None => self.render_empty(area, buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::Widget;

    use super::{ContentPanel, content_panel_title};
    use crate::ui::model::{UiItemContent, UiItemKind};
    use crate::ui::theme::Theme;

    fn item_content(id: &str, kind: UiItemKind) -> UiItemContent {
        UiItemContent {
            id: id.to_string(),
            title: id.to_string(),
            subtitle: None,
            kind,
            definition: String::new(),
            location: None,
            docs: None,
            badges: Vec::new(),
            sections: Vec::new(),
        }
    }

    #[test]
    fn content_panel_title_uses_memory_name_and_canister_id_for_memory_items() {
        let theme = Theme::default();
        let mut content = item_content(
            "uxrrr-q7777-77774-qaaaq-cai",
            UiItemKind::Custom("memory".to_string()),
        );
        content.title = "Alpha Memory".to_string();

        assert_eq!(
            content_panel_title(&theme, &content).to_string(),
            " ◇ Alpha Memory : uxrrr-q7777-77774-qaaaq-cai "
        );
    }

    #[test]
    fn content_panel_title_keeps_default_for_non_memory_items() {
        let theme = Theme::default();
        let content = item_content("search-1", UiItemKind::Custom("search-result".to_string()));

        assert_eq!(content_panel_title(&theme, &content).to_string(), " ◇ Content ");
    }

    #[test]
    fn section_headers_end_at_same_column_for_different_titles() {
        let theme = Theme::default();
        let panel = ContentPanel::new(&theme);

        let short = panel.section_header("Name").to_string();
        let long = panel.section_header("Description").to_string();

        assert_eq!(short.chars().count(), long.chars().count());
    }

    #[test]
    fn memory_content_keeps_description_section_in_body() {
        let theme = Theme::default();
        let mut content = item_content("memory-1", UiItemKind::Custom("memory".to_string()));
        content.subtitle = Some("Notes".to_string());

        let mut buf = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 60, 20));
        ContentPanel::new(&theme)
            .ui_content(Some(&content))
            .render(ratatui::layout::Rect::new(0, 0, 60, 20), &mut buf);

        let rendered = (0..20)
            .map(|y| {
                (0..60)
                    .filter_map(|x| buf.cell((x, y)).map(|cell| cell.symbol()))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Description"));
        assert!(rendered.contains("Notes"));
    }
}
