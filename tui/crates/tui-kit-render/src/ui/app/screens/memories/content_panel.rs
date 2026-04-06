//! Content and context-content panel for the memories screen.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::ui::app::{Focus, TuiKitUi};
use crate::ui::content::ContentPanel;
use crate::ui::context_view::{self, ContextView};

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_content_panel(&self, area: Rect, buf: &mut Buffer) {
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
                .focused(self.focus == Focus::Content)
                .scroll(self.inspector_scroll);
            context_view.render(area, buf);
            return;
        }

        let content = ContentPanel::new(self.theme)
            .ui_content(self.ui_selected_content)
            .focused(self.focus == Focus::Content)
            .scroll(self.inspector_scroll);
        content.render(area, buf);
    }
}
