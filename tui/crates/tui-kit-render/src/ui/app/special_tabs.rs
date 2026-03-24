//! Thin router for full-body tab screens.

use ratatui::{buffer::Buffer, layout::Rect};

use super::TuiKitUi;

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_special_tab_body(&self, area: Rect, buf: &mut Buffer) -> bool {
        self.render_tab_screen(area, buf)
    }
}
