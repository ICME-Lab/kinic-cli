//! Root frame composition and screen dispatch.

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Widget, block::BorderType},
};
use tui_kit_runtime::PickerState;
use tui_kit_runtime::kinic_tabs::{TabKind, tab_kind};

use crate::ui::components::TabBar;

use super::{Focus, TuiKitUi, shared};

impl<'a> TuiKitUi<'a> {
    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        if self.tab_specs.is_empty() {
            return;
        }
        let titles: Vec<&str> = self.tab_specs.iter().map(|t| t.title.as_str()).collect();
        let selected = self
            .tab_specs
            .iter()
            .position(|t| t.id == self.current_tab_id)
            .unwrap_or(0);
        let tab_bar = TabBar::new(titles, self.theme)
            .select(selected)
            .focused(self.focus == Focus::Tabs);
        tab_bar.render(area, buf);
    }

    pub fn cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if self.show_help
            || self.show_settings
            || self.show_create_modal
            || !matches!(self.picker, PickerState::Closed)
        {
            return None;
        }
        if self.access_control_open {
            return self.access_control_cursor_position_for_area(area);
        }
        match tab_kind(self.current_tab_id.0.as_str()) {
            TabKind::CreateForm => return self.create_cursor_position_for_area(area),
            TabKind::InsertForm => return self.insert_cursor_position_for_area(area),
            _ => {}
        }
        self.memories_cursor_position_for_area(area)
    }

    pub fn render_in_frame(self, frame: &mut Frame<'_>, area: Rect) {
        let cursor = self.cursor_position_for_area(area);
        frame.render_widget(self, area);
        if let Some((x, y)) = cursor {
            frame.set_cursor_position((x, y));
        }
    }
}

impl Widget for TuiKitUi<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use shared::layout::{HEADER_HEIGHT, STATUS_HEIGHT, TABS_HEIGHT};

        let tabs_height = if self.tab_specs.is_empty() {
            0
        } else {
            TABS_HEIGHT
        };
        let outer = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.style_border_glow())
            .style(Style::default().bg(self.theme.bg));
        outer.render(area, buf);

        let padded = shared::layout::content_area(area, true);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(HEADER_HEIGHT),
                Constraint::Length(tabs_height),
                Constraint::Min(12),
                Constraint::Length(STATUS_HEIGHT),
            ])
            .split(padded);

        self.render_header(chunks[0], buf);
        if tabs_height > 0 {
            self.render_tabs(chunks[1], buf);
        }

        if !self.render_special_tab_body(area, buf) {
            self.render_memories_body(chunks[2], buf);
        }
        self.render_status(chunks[3], buf);
        self.render_create_overlay(area, buf);
        self.render_settings_overlay(area, buf);
        self.render_picker_overlay(area, buf);
        self.render_access_control_overlay(area, buf);
        self.render_picker_overlay(area, buf);
        self.render_help_overlay(area, buf);
    }
}
