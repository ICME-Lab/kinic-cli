//! Search input and completion dropdown for the memories screen.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

use crate::ui::app::TuiKitUi;
use crate::ui::search::{SearchBar, SearchCompletion};
use tui_kit_runtime::{SearchScope, kinic_tabs::KINIC_MEMORIES_TAB_ID};

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_search_panel(&self, area: Rect, buf: &mut Buffer) {
        let placeholder = if self.show_context_panel && self.in_context_items_view {
            "Filter items... (e.g., de::Deserialize)"
        } else {
            self.tab_specs
                .iter()
                .find(|t| t.id == self.current_tab_id)
                .map(|t| t.search_placeholder.as_str())
                .unwrap_or("Search memories...")
        };
        let title = if self.current_tab_id.0.as_str() == KINIC_MEMORIES_TAB_ID {
            match self.search_scope {
                SearchScope::All => " Search [all] ",
                SearchScope::Selected => " Search [selected] ",
            }
        } else {
            " Search "
        };
        let search = SearchBar::new(self.search_input, self.theme)
            .focused(self.focus == crate::ui::app::Focus::Search)
            .placeholder(placeholder)
            .title(title);
        search.render(area, buf);
    }

    pub(super) fn render_search_completion(&self, search_area: Rect, buf: &mut Buffer) {
        if !self.show_completion || self.candidates.is_empty() {
            return;
        }
        let max_height = 12.min(self.candidates.len() as u16 + 2);
        let dropdown_area = Rect {
            x: search_area.x + 2,
            y: search_area.y + search_area.height,
            width: search_area.width.saturating_sub(4).min(60),
            height: max_height,
        };
        let completion = SearchCompletion::new(self.candidates, self.theme)
            .selected(self.completion_selected)
            .filter(self.search_input)
            .max_visible(10);
        completion.render(dropdown_area, buf);
    }
}
