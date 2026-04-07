//! Cursor geometry for the memories screen.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use unicode_width::UnicodeWidthStr;

use crate::ui::app::{Focus, TuiKitUi, shared};

use super::chat_panel::{CHAT_INPUT_MAX_HEIGHT, visible_chat_input_rows};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn memories_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        let layout = self.memories_screen_layout(self.memories_body_rect(area)?);

        match self.focus {
            Focus::Search => search_cursor_position(layout.search_rect, self.search_input),
            Focus::Chat => layout.chat_rect.and_then(|chat_rect| {
                chat_cursor_position(chat_rect, self.chat_input, self.chat_input_cursor)
            }),
            _ => None,
        }
    }

    fn memories_body_rect(&self, area: Rect) -> Option<Rect> {
        let padded = shared::layout::content_area(area, true);
        let tabs_height = if self.tab_specs.is_empty() {
            0
        } else {
            shared::layout::TABS_HEIGHT
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(shared::layout::HEADER_HEIGHT),
                Constraint::Length(tabs_height),
                Constraint::Min(12),
                Constraint::Length(shared::layout::STATUS_HEIGHT),
            ])
            .split(padded);
        Some(chunks[2])
    }
}

fn search_cursor_position(search_rect: Rect, search_input: &str) -> Option<(u16, u16)> {
    if search_rect.width < 3 || search_rect.height < 3 {
        return None;
    }
    let inner = Rect {
        x: search_rect.x + 1,
        y: search_rect.y + 1,
        width: search_rect.width.saturating_sub(2),
        height: search_rect.height.saturating_sub(2),
    };
    if inner.width == 0 || inner.height == 0 {
        return None;
    }
    let prompt_width = 2u16;
    let input_width = search_input.chars().count() as u16;
    let max_x = inner.x + inner.width.saturating_sub(1);
    let x = (inner.x + prompt_width + input_width).min(max_x);
    Some((x, inner.y))
}

fn chat_cursor_position(
    chat_rect: Rect,
    chat_input: &str,
    chat_input_cursor: Option<(usize, usize)>,
) -> Option<(u16, u16)> {
    if chat_rect.width < 4 || chat_rect.height < 4 {
        return None;
    }
    let chat_inner = Rect {
        x: chat_rect.x + 1,
        y: chat_rect.y + 1,
        width: chat_rect.width.saturating_sub(2),
        height: chat_rect.height.saturating_sub(2),
    };
    if chat_inner.width == 0 || chat_inner.height == 0 {
        return None;
    }
    let (cursor_row, cursor_col) = chat_input_cursor.unwrap_or_default();
    let input_rows = visible_chat_input_rows(
        chat_input,
        "<type your question>",
        CHAT_INPUT_MAX_HEIGHT,
        cursor_row,
        cursor_col,
        chat_inner.width.saturating_sub(3),
    );
    let input_height = input_rows.rows.len().max(1) as u16;
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(input_height)])
        .split(chat_inner);
    let input_area = v_chunks[1];
    if input_area.width == 0 || input_area.height == 0 {
        return None;
    }
    let visible_row = cursor_row
        .saturating_sub(input_rows.scroll_row)
        .min(input_height.saturating_sub(1) as usize) as u16;
    let prefix_width = 3u16;
    let visible_line = input_rows.rows.get(visible_row as usize)?;
    let row_start_col = *input_rows.row_start_cols.get(visible_row as usize).unwrap_or(&0);
    let visible_cursor_col = cursor_col.saturating_sub(row_start_col);
    let prefix = visible_line
        .chars()
        .take(visible_cursor_col + usize::from(row_start_col > 0))
        .collect::<String>();
    let input_width = UnicodeWidthStr::width(prefix.as_str()) as u16;
    let max_x = input_area.x + input_area.width.saturating_sub(1);
    let x = (input_area.x + prefix_width + input_width).min(max_x);
    Some((x, input_area.y + visible_row))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn memories_search_cursor_is_available() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .focus(Focus::Search)
            .search_input("abc");
        assert!(
            ui.memories_cursor_position_for_area(Rect::new(0, 0, 120, 40))
                .is_some()
        );
    }

    #[test]
    fn memories_chat_cursor_is_available_when_chat_is_open() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .focus(Focus::Chat)
            .show_chat(true)
            .chat_input("hello")
            .chat_input_cursor(Some((0, 5)));
        assert!(
            ui.memories_cursor_position_for_area(Rect::new(0, 0, 120, 40))
                .is_some()
        );
    }
}
