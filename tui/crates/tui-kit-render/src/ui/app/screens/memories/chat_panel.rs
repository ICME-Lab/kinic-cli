//! Chat panel rendering and markdown decoration for the memories screen.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget, Wrap, block::BorderType,
    },
};
use tui_kit_runtime::ChatScope;
use unicode_width::UnicodeWidthStr;

use crate::ui::app::{Focus, TuiKitUi};
use crate::ui::theme::Theme;

pub(super) const CHAT_INPUT_MAX_HEIGHT: u16 = 4;

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

fn wrap_chat_text_line(
    text: &str,
    width: usize,
    initial_prefix: &str,
    continuation_prefix: &str,
    style: Style,
) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }

    let prefix_width = UnicodeWidthStr::width(initial_prefix);
    let continuation_width = UnicodeWidthStr::width(continuation_prefix);
    let first_width = width.saturating_sub(prefix_width).max(1);
    let rest_width = width.saturating_sub(continuation_width).max(1);
    let wrapped = wrap_plain_text_segments(text, first_width, rest_width);

    if wrapped.is_empty() {
        return vec![Line::from(vec![
            Span::raw(initial_prefix.to_string()),
            Span::styled(String::new(), style),
        ])];
    }

    wrapped
        .into_iter()
        .enumerate()
        .map(|(index, segment)| {
            let prefix = if index == 0 {
                initial_prefix
            } else {
                continuation_prefix
            };
            Line::from(vec![
                Span::raw(prefix.to_string()),
                Span::styled(segment, style),
            ])
        })
        .collect()
}

fn wrap_chat_span_line(
    spans: &[Span<'static>],
    width: usize,
    initial_prefix: &str,
    continuation_prefix: &str,
) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }

    let prefix_width = UnicodeWidthStr::width(initial_prefix);
    let continuation_width = UnicodeWidthStr::width(continuation_prefix);
    let first_width = width.saturating_sub(prefix_width).max(1);
    let rest_width = width.saturating_sub(continuation_width).max(1);
    let wrapped = wrap_styled_segments(spans, first_width, rest_width);

    if wrapped.is_empty() {
        return vec![Line::from(Span::raw(initial_prefix.to_string()))];
    }

    wrapped
        .into_iter()
        .enumerate()
        .map(|(index, mut segment_spans)| {
            let prefix = if index == 0 {
                initial_prefix
            } else {
                continuation_prefix
            };
            let mut line_spans = vec![Span::raw(prefix.to_string())];
            line_spans.append(&mut segment_spans);
            Line::from(line_spans)
        })
        .collect()
}

fn build_chat_lines(ui: &TuiKitUi<'_>, width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    if ui.chat_loading && ui.chat_messages.last().is_some_and(|(r, _)| r == "user") {
        lines.push(Line::from(Span::styled(
            ui.ui_config.chat.loading_hint.clone(),
            ui.theme.style_muted(),
        )));
    }
    for (role, content) in ui.chat_messages {
        let label = if role == "user" { "You" } else { "Chat" };
        let base_style = if role == "user" {
            ui.theme.style_accent()
        } else {
            ui.theme.style_normal()
        };
        lines.push(Line::from(Span::styled(
            format!("  {}: ", label),
            ui.theme.style_dim().add_modifier(Modifier::BOLD),
        )));
        for raw_line in content.lines() {
            let trimmed = raw_line.trim_end();
            if role == "assistant" {
                let spans = markdown_line_to_spans(trimmed, ui.theme, base_style);
                lines.extend(wrap_chat_span_line(&spans, width, "    ", "    "));
            } else {
                lines.extend(wrap_chat_text_line(
                    trimmed, width, "    ", "    ", base_style,
                ));
            }
        }
        lines.push(Line::from(""));
    }
    lines
}

fn wrap_plain_text_segments(text: &str, first_width: usize, rest_width: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut max_width = first_width;

    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width > 0 && current_width + ch_width > max_width {
            lines.push(current.trim_end().to_string());
            current = String::new();
            current_width = 0;
            max_width = rest_width;
            if ch.is_whitespace() {
                continue;
            }
        }
        current.push(ch);
        current_width += ch_width;
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn wrap_styled_segments(
    spans: &[Span<'static>],
    first_width: usize,
    rest_width: usize,
) -> Vec<Vec<Span<'static>>> {
    let mut lines: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    let mut current_width = 0usize;
    let mut max_width = first_width;

    for span in spans {
        let style = span.style;
        let mut remaining = span.content.to_string();

        while !remaining.is_empty() {
            let available = max_width.saturating_sub(current_width).max(1);
            let split_at = split_index_for_width(remaining.as_str(), available);

            if split_at == 0 && current_width > 0 {
                lines.push(Vec::new());
                current_width = 0;
                max_width = rest_width;
                continue;
            }

            let take = if split_at == 0 {
                remaining.clone()
            } else {
                remaining[..split_at].to_string()
            };
            let rest = if split_at == 0 {
                String::new()
            } else {
                remaining[split_at..].to_string()
            };

            if !take.is_empty() {
                lines
                    .last_mut()
                    .expect("chat wrapped line should exist")
                    .push(Span::styled(take.clone(), style));
                current_width += UnicodeWidthStr::width(take.as_str());
            }

            remaining = rest;

            if !remaining.is_empty() {
                lines.push(Vec::new());
                current_width = 0;
                max_width = rest_width;
            }
        }
    }

    if lines.len() == 1 && lines[0].is_empty() {
        Vec::new()
    } else {
        lines
    }
}

fn split_index_for_width(text: &str, width: usize) -> usize {
    let mut current_width = 0usize;
    let mut split_at = 0usize;

    for (index, ch) in text.char_indices() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width > 0 && current_width + ch_width > width {
            break;
        }
        current_width += ch_width;
        split_at = index + ch.len_utf8();
    }

    split_at
}

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_chat_panel(&self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }
        let border_style = if self.focus == Focus::Chat {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let title = match self.chat_scope {
            ChatScope::All => " ◇ Chat [all] ",
            ChatScope::Selected => " ◇ Chat [selected] ",
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        let input_visible = visible_chat_input_rows(
            self.chat_input,
            self.ui_config.chat.input_placeholder.as_str(),
            CHAT_INPUT_MAX_HEIGHT,
            self.chat_input_cursor.unwrap_or_default().0,
            self.chat_input_cursor.unwrap_or_default().1,
            inner.width.saturating_sub(3),
        );
        let input_height = input_visible.rows.len().max(1) as u16;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(input_height)])
            .split(inner);
        let messages_area = chunks[0];
        let input_area = chunks[1];
        let full_width = messages_area.width as usize;
        let mut lines = build_chat_lines(self, full_width);
        let visible_height = messages_area.height as usize;
        let needs_scrollbar = lines.len() > visible_height;
        let content_width = if needs_scrollbar {
            messages_area.width.saturating_sub(1) as usize
        } else {
            full_width
        };
        if needs_scrollbar && content_width > 0 {
            lines = build_chat_lines(self, content_width);
        }

        let total_lines = lines.len();
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll = self.chat_scroll.min(max_scroll);

        let content_area = Rect {
            width: if total_lines > visible_height {
                messages_area.width.saturating_sub(1)
            } else {
                messages_area.width
            },
            ..messages_area
        };
        let visible_lines: Vec<Line<'static>> = lines
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
        let input_lines = input_visible
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let prefix = if index == 0 { " ▸ " } else { "   " };
                Line::from(vec![
                    Span::styled(prefix, self.theme.style_dim()),
                    Span::styled(row.clone(), input_style),
                ])
            })
            .collect::<Vec<_>>();
        Paragraph::new(input_lines)
            .wrap(Wrap { trim: false })
            .render(input_inner, buf);
    }
}

pub(super) struct VisibleChatInputRows {
    pub(super) rows: Vec<String>,
    pub(super) row_start_cols: Vec<usize>,
    pub(super) scroll_row: usize,
}

pub(super) fn visible_chat_input_rows(
    value: &str,
    placeholder: &str,
    max_height: u16,
    cursor_row: usize,
    cursor_col: usize,
    max_width: u16,
) -> VisibleChatInputRows {
    let mut source_rows = if value.is_empty() {
        vec![placeholder.to_string()]
    } else {
        value
            .split('\n')
            .map(|row| row.to_string())
            .collect::<Vec<_>>()
    };
    if source_rows.is_empty() {
        source_rows.push(String::new());
    }
    let visible_height = source_rows.len().clamp(1, max_height as usize);
    let scroll_row = if cursor_row >= visible_height {
        cursor_row + 1 - visible_height
    } else {
        0
    };
    let mut row_start_cols = Vec::new();
    let mut rows = source_rows
        .into_iter()
        .skip(scroll_row)
        .take(visible_height)
        .enumerate()
        .map(|(index, row)| {
            let row_cursor_col = if value.is_empty() {
                0
            } else if scroll_row + index == cursor_row {
                cursor_col
            } else {
                0
            };
            let (trimmed, start_col) = trim_chat_input_row(row.as_str(), max_width, row_cursor_col);
            row_start_cols.push(start_col);
            trimmed
        })
        .collect::<Vec<_>>();
    while rows.len() < visible_height {
        rows.push(String::new());
        row_start_cols.push(0);
    }
    VisibleChatInputRows {
        rows,
        row_start_cols,
        scroll_row,
    }
}

fn trim_chat_input_row(value: &str, max_width: u16, cursor_col: usize) -> (String, usize) {
    if max_width == 0 || UnicodeWidthStr::width(value) as u16 <= max_width {
        return (value.to_string(), 0);
    }

    let chars = value.chars().collect::<Vec<_>>();
    let char_widths = chars
        .iter()
        .map(|ch| unicode_width::UnicodeWidthChar::width(*ch).unwrap_or(0))
        .collect::<Vec<_>>();
    let total_width = char_widths.iter().sum::<usize>();
    if total_width as u16 <= max_width {
        return (value.to_string(), 0);
    }

    let cursor_index = cursor_col.min(chars.len());
    let prefix_width = char_widths.iter().take(cursor_index).sum::<usize>();
    let window_width = max_width.saturating_sub(2).max(1) as usize;
    let mut start_width = prefix_width.saturating_sub(window_width.saturating_sub(1));
    let max_start_width = total_width.saturating_sub(window_width);
    start_width = start_width.min(max_start_width);

    let mut start_index = 0usize;
    let mut consumed_width = 0usize;
    while start_index < chars.len() && consumed_width < start_width {
        consumed_width += char_widths[start_index];
        start_index += 1;
    }

    let hidden_left = start_index > 0;
    let available_width = max_width as usize - usize::from(hidden_left);
    let mut end_index = start_index;
    let mut visible_width = 0usize;
    while end_index < chars.len() && visible_width + char_widths[end_index] <= available_width {
        visible_width += char_widths[end_index];
        end_index += 1;
    }

    let hidden_right = end_index < chars.len();
    let mut visible = chars[start_index..end_index].iter().collect::<String>();

    if hidden_right {
        let right_budget = available_width.saturating_sub(1);
        while !visible.is_empty() && UnicodeWidthStr::width(visible.as_str()) > right_budget {
            visible.pop();
        }
    }

    let mut result = String::new();
    if hidden_left {
        result.push('…');
    }
    result.push_str(visible.as_str());
    if hidden_right {
        result.push('…');
    }
    (result, start_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn wrapped_user_chat_lines_keep_indentation() {
        let theme = Theme::default();
        let messages = vec![(
            "user".to_string(),
            "1234567890123456789012345678901234567890".to_string(),
        )];
        let ui = TuiKitUi::new(&theme)
            .chat_messages(&messages)
            .focus(Focus::Chat);
        let lines = build_chat_lines(&ui, 20);

        assert!(lines.len() > 3);
        assert!(lines[1].to_string().starts_with("    "));
        assert!(lines[2].to_string().starts_with("    "));
    }

    #[test]
    fn wrapped_assistant_chat_lines_keep_indentation() {
        let theme = Theme::default();
        let messages = vec![(
            "assistant".to_string(),
            "alpha beta gamma delta epsilon zeta eta theta iota".to_string(),
        )];
        let ui = TuiKitUi::new(&theme)
            .chat_messages(&messages)
            .focus(Focus::Chat);
        let lines = build_chat_lines(&ui, 20);

        assert!(lines.len() > 3);
        assert!(lines[1].to_string().starts_with("    "));
        assert!(lines[2].to_string().starts_with("    "));
    }

    #[test]
    fn visible_chat_input_rows_keeps_cursor_side_visible_near_end() {
        let visible = visible_chat_input_rows(
            "abcdefghijklmnopqrstuvwxyz",
            "placeholder",
            CHAT_INPUT_MAX_HEIGHT,
            0,
            26,
            10,
        );

        assert_eq!(visible.rows, vec!["…stuvwxyz".to_string()]);
    }

    #[test]
    fn visible_chat_input_rows_shows_both_ellipses_when_cursor_is_mid_line() {
        let visible = visible_chat_input_rows(
            "abcdefghijklmnopqrstuvwxyz",
            "placeholder",
            CHAT_INPUT_MAX_HEIGHT,
            0,
            13,
            10,
        );

        assert_eq!(visible.rows, vec!["…ghijklmn…".to_string()]);
    }

    #[test]
    fn visible_chat_input_rows_preserves_placeholder_and_short_rows() {
        let placeholder = visible_chat_input_rows("", "type here", CHAT_INPUT_MAX_HEIGHT, 0, 0, 20);
        let short = visible_chat_input_rows("short", "type here", CHAT_INPUT_MAX_HEIGHT, 0, 5, 20);

        assert_eq!(placeholder.rows, vec!["type here".to_string()]);
        assert_eq!(short.rows, vec!["short".to_string()]);
    }
}
