//! Context list/details view using generic UI nodes.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{
        block::BorderType, Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};

use crate::ui::model::UiContextNode;
use crate::ui::theme::Theme;

/// View for displaying context information (scrollable).
pub struct ContextView<'a> {
    ui_node: Option<&'a UiContextNode>,
    theme: &'a Theme,
    focused: bool,
    scroll_offset: usize,
    show_link_hints: bool,
}

impl<'a> ContextView<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            ui_node: None,
            theme,
            focused: false,
            scroll_offset: 0,
            show_link_hints: false,
        }
    }

    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn ui_node(mut self, node: Option<&'a UiContextNode>) -> Self {
        self.ui_node = node;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn show_link_hints(mut self, show: bool) -> Self {
        self.show_link_hints = show;
        self
    }

    /// Number of lines this view would render (for scroll clamping).
    pub fn content_height(&self) -> usize {
        self.ui_node
            .map_or(10, |node| self.build_ui_node_lines(node).len())
    }

    fn render_empty(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.style_border())
            .title(" ◇ Context ");

        let inner = block.inner(area);
        block.render(area, buf);

        let mut help = vec![
            Line::from(""),
            Line::from(Span::styled(
                "This panel shows context metadata. Select one to view details.",
                self.theme.style_dim(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Provide context data from your host app/provider to populate this view.",
                self.theme.style_muted(),
            )),
        ];
        if self.show_link_hints {
            help.push(Line::from(""));
            help.push(Line::from(vec![
                Span::styled(" [o] ", self.theme.style_accent()),
                Span::styled("primary link  ", self.theme.style_dim()),
                Span::styled(" [c] ", self.theme.style_accent()),
                Span::styled("secondary link", self.theme.style_dim()),
            ]));
        }
        Paragraph::new(help)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }

    fn build_ui_node_lines(&self, node: &UiContextNode) -> Vec<Line<'static>> {
        let mut lines = vec![Line::from(vec![
            Span::styled(
                node.name.clone(),
                self.theme
                    .style_accent_bold()
                    .add_modifier(Modifier::UNDERLINED),
            ),
            Span::raw(" "),
            Span::styled(
                node.version
                    .as_ref()
                    .map(|v| format!("v{v}"))
                    .unwrap_or_default(),
                self.theme.style_dim(),
            ),
        ])];
        lines.push(Line::from(""));

        if let Some(relation) = &node.relation {
            lines.push(Line::from(vec![
                Span::styled("Relation: ", self.theme.style_dim()),
                Span::styled(relation.clone(), self.theme.style_normal()),
            ]));
        }

        if let Some(desc) = &node.description {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                desc.clone(),
                self.theme.style_normal(),
            )));
        }

        if let Some(v) = &node.docs_url {
            lines.push(Line::from(vec![
                Span::styled("Primary: ", self.theme.style_dim()),
                Span::styled(v.clone(), self.theme.style_accent()),
            ]));
        }
        if let Some(v) = &node.homepage_url {
            lines.push(Line::from(vec![
                Span::styled("Secondary: ", self.theme.style_dim()),
                Span::styled(v.clone(), self.theme.style_accent()),
            ]));
        }
        if let Some(v) = &node.repository_url {
            lines.push(Line::from(vec![
                Span::styled("Reference: ", self.theme.style_dim()),
                Span::styled(v.clone(), self.theme.style_accent()),
            ]));
        }

        if !node.metadata.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Metadata:",
                self.theme.style_dim(),
            )));
            for row in &node.metadata {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{}: ", row.label), self.theme.style_dim()),
                    Span::styled(row.value.clone(), self.theme.style_normal()),
                ]));
            }
        }

        lines
    }

    fn render_ui_node(&self, node: &UiContextNode, area: Rect, buf: &mut Buffer) {
        let lines = self.build_ui_node_lines(node);
        let total_lines = lines.len();
        let inner = Block::default().inner(area);
        let viewport_height = inner.height as usize;
        let max_scroll = total_lines.saturating_sub(viewport_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.focused {
                self.theme.style_border_focused()
            } else {
                self.theme.style_border()
            })
            .title(" ◇ Context ");
        let inner = block.inner(area);
        block.render(area, buf);

        let visible_lines: Vec<Line> = lines.into_iter().skip(scroll_offset).collect();
        Paragraph::new(visible_lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);

        if total_lines > inner.height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll_offset);
            StatefulWidget::render(scrollbar, inner, buf, &mut scrollbar_state);
        }

        if self.show_link_hints && inner.height > 0 {
            let hint_y = inner.y + inner.height - 1;
            let hint_line = Line::from(vec![
                Span::styled(" [o] ", self.theme.style_accent()),
                Span::styled("primary link  ", self.theme.style_dim()),
                Span::styled(" [c] ", self.theme.style_accent()),
                Span::styled("secondary link", self.theme.style_dim()),
            ]);
            Paragraph::new(hint_line).render(
                Rect {
                    y: hint_y,
                    height: 1,
                    ..inner
                },
                buf,
            );
        }
    }
}

impl Widget for ContextView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.ui_node {
            Some(node) => self.render_ui_node(node, area, buf),
            None => self.render_empty(area, buf),
        }
    }
}

/// Render loading state for context details.
pub fn render_context_loading(theme: &Theme, area: Rect, buf: &mut Buffer, context_name: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.style_border())
        .title(" ◇ Context ");
    let inner = block.inner(area);
    block.render(area, buf);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Loading details for ", theme.style_dim()),
            Span::styled(context_name.to_string(), theme.style_accent()),
            Span::styled("...", theme.style_dim()),
        ]),
    ];
    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .render(inner, buf);
}

/// Render failure state for context details.
pub fn render_context_load_failed(theme: &Theme, area: Rect, buf: &mut Buffer, context_name: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.style_border())
        .title(" ◇ Context ");
    let inner = block.inner(area);
    block.render(area, buf);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Failed to load details for ", theme.style_error()),
            Span::styled(context_name.to_string(), theme.style_accent()),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("Use [o]/[c] to open linked resources.", theme.style_dim()),
        ]),
    ];
    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .render(inner, buf);
}
