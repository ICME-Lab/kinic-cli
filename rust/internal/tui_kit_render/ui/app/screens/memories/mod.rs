//! Memories screen composition.

use ratatui::{buffer::Buffer, layout::Rect};

use crate::ui::app::TuiKitUi;

mod chat_panel;
mod content_panel;
mod context_list_panel;
mod cursor;
mod items_panel;
mod layout;
mod search_panel;

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_memories_body(&self, body: Rect, buf: &mut Buffer) {
        let layout = self.memories_screen_layout(body);

        self.render_search_panel(layout.search_rect, buf);
        if self.show_context_panel && !self.in_context_items_view {
            self.render_context_list_panel(layout.list_rect, buf);
        } else {
            self.render_items_panel(layout.list_rect, buf);
        }
        self.render_memories_divider(layout.divider_rect, buf);
        if let Some(chat_rect) = layout.chat_rect {
            self.render_chat_panel(chat_rect, buf);
        } else {
            self.render_content_panel(layout.detail_rect, buf);
        }
        self.render_search_completion(layout.search_rect, buf);
    }
}
