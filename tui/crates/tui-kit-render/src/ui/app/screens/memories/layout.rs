//! Layout helpers for the memories screen.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
};

use crate::ui::app::TuiKitUi;

#[derive(Clone, Copy)]
pub(super) struct MemoriesScreenLayout {
    pub search_rect: Rect,
    pub list_rect: Rect,
    pub divider_rect: Rect,
    pub detail_rect: Rect,
    pub chat_rect: Option<Rect>,
}

impl<'a> TuiKitUi<'a> {
    pub(super) fn memories_screen_layout(&self, body: Rect) -> MemoriesScreenLayout {
        let left_div_right = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Length(1),
                Constraint::Ratio(2, 3),
            ])
            .split(body);
        let left_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(6)])
            .split(left_div_right[0]);
        let (detail_rect, chat_rect) = if self.show_chat_panel {
            (left_div_right[2], Some(left_div_right[2]))
        } else {
            (left_div_right[2], None)
        };

        MemoriesScreenLayout {
            search_rect: left_split[0],
            list_rect: left_split[1],
            divider_rect: left_div_right[1],
            detail_rect,
            chat_rect,
        }
    }

    pub(super) fn render_memories_divider(&self, area: Rect, buf: &mut Buffer) {
        let style = self.theme.style_border();
        for y in area.top()..area.bottom() {
            if area.width > 0 {
                if let Some(cell) = buf.cell_mut((area.x, y)) {
                    cell.set_symbol("│").set_style(style);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::*;
    use crate::theme::Theme;

    #[test]
    fn memories_layout_uses_full_detail_pane_for_chat_when_open() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme).show_chat(true);

        let layout = ui.memories_screen_layout(Rect::new(0, 0, 120, 40));

        assert_eq!(layout.chat_rect, Some(layout.detail_rect));
    }

    #[test]
    fn memories_layout_keeps_chat_closed_when_panel_is_hidden() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme).show_chat(false);

        let layout = ui.memories_screen_layout(Rect::new(0, 0, 120, 40));

        assert_eq!(layout.chat_rect, None);
    }
}
