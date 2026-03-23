//! Right panel: vertical divider, inspector/context views and chat chat.

use crate::ui::context_view::{self, ContextView};
use crate::ui::inspector::InspectorPanel;
use crate::ui::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        block::BorderType, Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};

use super::types::Focus;
use super::TuiKitUi;

/// Parse a line of markdown into styled spans: **bold**, `code`, ## header.
fn markdown_line_to_spans(line: &str, theme: &Theme, base_style: Style) -> Vec<Span<'static>> {
    let mut spans: Vec<Span> = Vec::new();
    let bold = base_style.add_modifier(Modifier::BOLD);
    let code_style = theme.style_type();
    let header_style = theme.style_accent().add_modifier(Modifier::BOLD);

    let mut s = line;
    if s.starts_with("## ") {
        spans.push(Span::styled("  ", theme.style_dim()));
        spans.push(Span::styled(
            s.trim_start_matches("## ").to_string(),
            header_style,
        ));
        return spans;
    }
    if s.starts_with("# ") {
        spans.push(Span::styled("  ", theme.style_dim()));
        spans.push(Span::styled(
            s.trim_start_matches("# ").to_string(),
            header_style,
        ));
        return spans;
    }

    while !s.is_empty() {
        if let Some(rest) = s.strip_prefix("**") {
            if let Some(end) = rest.find("**") {
                spans.push(Span::styled(rest[..end].to_string(), bold));
                s = &rest[end + 2..];
                continue;
            }
        }
        if s.starts_with('`') {
            if let Some(end) = s[1..].find('`') {
                let code = &s[1..=end];
                spans.push(Span::styled(code.to_string(), code_style));
                s = &s[end + 2..];
                continue;
            }
        }
        let next_bold = s.find("**");
        let next_code = s.find('`');
        let next = match (next_bold, next_code) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        if let Some(i) = next {
            spans.push(Span::styled(s[..i].to_string(), base_style));
            s = &s[i..];
        } else {
            spans.push(Span::styled(s.to_string(), base_style));
            break;
        }
    }
    spans
}

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_vertical_divider(&self, area: Rect, buf: &mut Buffer) {
        let style = self.theme.style_border();
        let symbol = "│";
        for y in area.top()..area.bottom() {
            if area.width > 0 {
                if let Some(cell) = buf.cell_mut((area.x, y)) {
                    cell.set_symbol(symbol).set_style(style);
                }
            }
        }
    }

    pub(super) fn render_inspector(&self, area: Rect, buf: &mut Buffer) {
        if self.show_context_panel {
            let selected_context_name = self
                .list_selected
                .and_then(|i| self.filtered_context_indices.get(i).copied())
                .and_then(|tree_idx| self.context_tree.get(tree_idx))
                .map(|(n, _)| n.as_str());

            if self.context_details_loading && self.ui_context_node.is_none() {
                if let Some(name) = selected_context_name {
                    context_view::render_context_loading(self.theme, area, buf, name);
                    return;
                }
            }
            if self.context_details_failed && self.ui_context_node.is_none() {
                if let Some(name) = selected_context_name {
                    context_view::render_context_load_failed(self.theme, area, buf, name);
                    return;
                }
            }

            let context_view = ContextView::new(self.theme)
                .ui_node(self.ui_context_node)
                .focused(self.focus == Focus::Inspector)
                .scroll(self.inspector_scroll)
                .show_link_hints(true);
            context_view.render(area, buf);
            return;
        }

        let inspector = InspectorPanel::new(self.theme)
            .ui_detail(self.ui_selected_detail)
            .focused(self.focus == Focus::Inspector)
            .scroll(self.inspector_scroll);
        inspector.render(area, buf);
    }

    pub(super) fn render_chat_panel(&self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }
        let border_style = if self.focus == Focus::Chat {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(self.ui_config.chat.title.as_str());
        let inner = block.inner(area);
        block.render(area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(1)])
            .split(inner);
        let messages_area = chunks[0];
        let input_area = chunks[1];

        let mut lines: Vec<Line<'_>> = Vec::new();
        if self.chat_loading && self.chat_messages.last().is_some_and(|(r, _)| r == "user") {
            lines.push(Line::from(Span::styled(
                self.ui_config.chat.loading_hint.as_str(),
                self.theme.style_muted(),
            )));
        }
        for (role, content) in self.chat_messages {
            let label = if role == "user" { "You" } else { "Chat" };
            let base_style = if role == "user" {
                self.theme.style_accent()
            } else {
                self.theme.style_normal()
            };
            lines.push(Line::from(Span::styled(
                format!("  {}: ", label),
                self.theme.style_dim().add_modifier(Modifier::BOLD),
            )));
            for raw_line in content.lines() {
                let trimmed = raw_line.trim_end();
                if role == "assistant" {
                    let mut sp = vec![Span::raw("    ")];
                    sp.extend(markdown_line_to_spans(trimmed, self.theme, base_style));
                    lines.push(Line::from(sp));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(trimmed.to_string(), base_style),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }

        let total_lines = lines.len();
        let visible_height = messages_area.height as usize;
        let max_scroll = total_lines.saturating_sub(visible_height).max(0);
        let scroll = self.chat_scroll.min(max_scroll);

        let content_area = Rect {
            width: messages_area.width.saturating_sub(1),
            ..messages_area
        };
        let visible_lines: Vec<Line<'_>> = lines
            .iter()
            .skip(scroll)
            .take(visible_height)
            .cloned()
            .collect();
        Paragraph::new(visible_lines)
            .wrap(Wrap { trim: false })
            .render(content_area, buf);

        if total_lines > visible_height {
            let scrollbar_area = Rect {
                x: messages_area.x + messages_area.width.saturating_sub(1),
                y: messages_area.y,
                width: 1,
                height: messages_area.height,
            };
            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(scroll)
                .viewport_content_length(visible_height);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            scrollbar.render(scrollbar_area, buf, &mut scrollbar_state);
        }

        let input_display: &str = if self.chat_input.is_empty() {
            self.ui_config.chat.input_placeholder.as_str()
        } else {
            self.chat_input
        };
        let input_style = if self.focus == Focus::Chat {
            self.theme.style_accent()
        } else {
            self.theme.style_muted()
        };
        let input_block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(self.theme.bg_highlight));
        let input_inner = input_block.inner(input_area);
        input_block.render(input_area, buf);
        let input_line = Paragraph::new(Line::from(vec![
            Span::styled(" ▸ ", self.theme.style_dim()),
            Span::styled(input_display, input_style),
        ]));
        input_line.render(input_inner, buf);
    }
}
