//! Main tui-kit TUI application, composed from blocks (header, list, status, overlays, right_panel).

mod header;
mod layout;
mod list;
mod overlays;
mod right_panel;
mod status;
mod types;

pub use layout::{list_viewport_height_for_area, tabs_rect_for_area};
pub use types::{
    default_tab_specs, BrandingText, ChatPanelText, Focus, HeaderText, TabId, TabSpec, UiConfig,
};

use crate::ui::animation::AnimationState;
use crate::ui::components::TabBar;
use crate::ui::model::{UiContextNode, UiItemDetail, UiItemSummary};
use crate::ui::search::{CompletionCandidate, SearchBar, SearchCompletion};
use crate::ui::theme::Theme;

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{block::BorderType, Block, Borders, Widget},
};

/// Main tui-kit UI widget, data and builder; rendering is delegated to block modules.
pub struct TuiKitUi<'a> {
    // Data
    pub(super) candidates: &'a [CompletionCandidate],
    pub(super) context_tree: &'a [(String, usize)],
    pub(super) filtered_context_indices: &'a [usize],
    pub(super) context_details_loading: bool,
    pub(super) context_details_failed: bool,
    pub(super) ui_summaries: &'a [UiItemSummary],
    pub(super) ui_selected_detail: Option<&'a UiItemDetail>,
    pub(super) ui_context_node: Option<&'a UiContextNode>,
    pub(super) ui_total_count: usize,
    pub(super) in_context_items_view: bool,
    pub(super) show_context_panel: bool,
    pub(super) target_size_bytes: Option<u64>,
    // UI state
    pub(super) search_input: &'a str,
    pub(super) current_tab_id: TabId,
    pub(super) tab_specs: Vec<TabSpec>,
    pub(super) ui_config: UiConfig,
    pub(super) focus: Focus,
    pub(super) list_selected: Option<usize>,
    pub(super) list_scroll_offset: Option<usize>,
    pub(super) completion_selected: usize,
    pub(super) show_completion: bool,
    pub(super) show_help: bool,
    pub(super) show_settings: bool,
    pub(super) status_message: &'a str,
    pub(super) inspector_scroll: usize,
    pub(super) animation: Option<&'a AnimationState>,
    pub(super) theme: &'a Theme,
    // Chat in-TUI chat
    pub(super) show_chat_panel: bool,
    pub(super) chat_messages: &'a [(String, String)],
    pub(super) chat_input: &'a str,
    pub(super) chat_loading: bool,
    pub(super) chat_scroll: usize,
}

impl<'a> TuiKitUi<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            candidates: &[],
            context_tree: &[],
            filtered_context_indices: &[],
            context_details_loading: false,
            context_details_failed: false,
            ui_summaries: &[],
            ui_selected_detail: None,
            ui_context_node: None,
            ui_total_count: 0,
            in_context_items_view: false,
            show_context_panel: false,
            target_size_bytes: None,
            search_input: "",
            current_tab_id: TabId::new("tab-1"),
            tab_specs: default_tab_specs(),
            ui_config: UiConfig::default(),
            focus: Focus::default(),
            list_selected: None,
            list_scroll_offset: None,
            completion_selected: 0,
            show_completion: false,
            show_help: false,
            show_settings: false,
            status_message: "",
            inspector_scroll: 0,
            animation: None,
            theme,
            show_chat_panel: false,
            chat_messages: &[],
            chat_input: "",
            chat_loading: false,
            chat_scroll: 0,
        }
    }

    #[must_use]
    pub fn ui_summaries(mut self, items: &'a [UiItemSummary]) -> Self {
        self.ui_summaries = items;
        self
    }

    #[must_use]
    pub fn ui_selected_detail(mut self, detail: Option<&'a UiItemDetail>) -> Self {
        self.ui_selected_detail = detail;
        self
    }

    #[must_use]
    pub fn ui_context_node(mut self, node: Option<&'a UiContextNode>) -> Self {
        self.ui_context_node = node;
        self
    }

    #[must_use]
    pub fn ui_total_count(mut self, count: usize) -> Self {
        self.ui_total_count = count;
        self
    }

    #[must_use]
    pub fn in_context_items_view(mut self, v: bool) -> Self {
        self.in_context_items_view = v;
        self
    }

    #[must_use]
    pub fn show_context_panel(mut self, show: bool) -> Self {
        self.show_context_panel = show;
        self
    }

    #[must_use]
    pub fn target_size_bytes(mut self, bytes: Option<u64>) -> Self {
        self.target_size_bytes = bytes;
        self
    }

    #[must_use]
    pub fn list_selected(mut self, selected: Option<usize>) -> Self {
        self.list_selected = selected;
        self
    }

    #[must_use]
    pub fn list_scroll(mut self, offset: usize) -> Self {
        self.list_scroll_offset = Some(offset);
        self
    }

    #[must_use]
    pub fn candidates(mut self, candidates: &'a [CompletionCandidate]) -> Self {
        self.candidates = candidates;
        self
    }

    #[must_use]
    pub fn context_tree(mut self, tree: &'a [(String, usize)]) -> Self {
        self.context_tree = tree;
        self
    }

    #[must_use]
    pub fn filtered_context_indices(mut self, indices: &'a [usize]) -> Self {
        self.filtered_context_indices = indices;
        self
    }

    #[must_use]
    pub fn context_details_loading(mut self, loading: bool) -> Self {
        self.context_details_loading = loading;
        self
    }

    #[must_use]
    pub fn context_details_failed(mut self, failed: bool) -> Self {
        self.context_details_failed = failed;
        self
    }

    #[must_use]
    pub fn search_input(mut self, input: &'a str) -> Self {
        self.search_input = input;
        self
    }

    #[must_use]
    pub fn current_tab_id(mut self, tab_id: TabId) -> Self {
        self.current_tab_id = tab_id;
        self
    }

    #[must_use]
    pub fn tab_specs(mut self, tabs: Vec<TabSpec>) -> Self {
        self.tab_specs = tabs;
        self
    }

    #[must_use]
    pub fn ui_config(mut self, config: UiConfig) -> Self {
        self.tab_specs = config.tabs.clone();
        self.ui_config = config;
        self
    }

    #[must_use]
    pub fn focus(mut self, focus: Focus) -> Self {
        self.focus = focus;
        self
    }

    #[must_use]
    pub fn completion_selected(mut self, index: usize) -> Self {
        self.completion_selected = index;
        self
    }

    #[must_use]
    pub fn show_completion(mut self, show: bool) -> Self {
        self.show_completion = show;
        self
    }

    #[must_use]
    pub fn show_help(mut self, show: bool) -> Self {
        self.show_help = show;
        self
    }

    #[must_use]
    pub fn show_settings(mut self, show: bool) -> Self {
        self.show_settings = show;
        self
    }

    #[must_use]
    pub fn status_message(mut self, msg: &'a str) -> Self {
        self.status_message = msg;
        self
    }

    #[must_use]
    pub fn inspector_scroll(mut self, scroll: usize) -> Self {
        self.inspector_scroll = scroll;
        self
    }

    #[must_use]
    pub fn animation_state(mut self, animation: &'a AnimationState) -> Self {
        self.animation = Some(animation);
        self
    }

    #[must_use]
    pub fn show_chat(mut self, show: bool) -> Self {
        self.show_chat_panel = show;
        self
    }

    #[must_use]
    pub fn chat_messages(mut self, messages: &'a [(String, String)]) -> Self {
        self.chat_messages = messages;
        self
    }

    #[must_use]
    pub fn chat_input(mut self, input: &'a str) -> Self {
        self.chat_input = input;
        self
    }

    #[must_use]
    pub fn chat_loading(mut self, loading: bool) -> Self {
        self.chat_loading = loading;
        self
    }

    #[must_use]
    pub fn chat_scroll(mut self, scroll: usize) -> Self {
        self.chat_scroll = scroll;
        self
    }

    fn render_search(&self, area: Rect, buf: &mut Buffer) {
        let placeholder = if self.show_context_panel && self.in_context_items_view {
            "Filter items... (e.g., de::Deserialize)"
        } else {
            self.tab_specs
                .iter()
                .find(|t| t.id == self.current_tab_id)
                .map(|t| t.search_placeholder.as_str())
                .unwrap_or("Search...")
        };
        let search = SearchBar::new(self.search_input, self.theme)
            .focused(self.focus == Focus::Search)
            .placeholder(placeholder);
        search.render(area, buf);
    }

    fn render_completion(&self, search_area: Rect, buf: &mut Buffer) {
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

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
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

    /// Compute terminal cursor position for active text inputs.
    ///
    /// Returns `None` when no text input is focused or overlays are shown.
    pub fn cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if self.show_help || self.show_settings {
            return None;
        }

        // Rebuild the same layout geometry used by `render` so cursor placement stays in sync.
        let padded = layout::content_area(area, true);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(layout::HEADER_HEIGHT),
                Constraint::Length(layout::TABS_HEIGHT),
                Constraint::Min(12),
                Constraint::Length(layout::STATUS_HEIGHT),
            ])
            .split(padded);
        let body = chunks[2];
        let left_div_right = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Length(1),
                Constraint::Ratio(2, 3),
            ])
            .split(body);
        let left_column = left_div_right[0];
        let right_column = left_div_right[2];
        let left_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(6)])
            .split(left_column);
        let search_rect = left_split[0];

        match self.focus {
            Focus::Search => {
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
                let prompt_width = 2u16; // "❯ "
                let input_width = self.search_input.chars().count() as u16;
                let max_x = inner.x + inner.width.saturating_sub(1);
                let x = (inner.x + prompt_width + input_width).min(max_x);
                Some((x, inner.y))
            }
            Focus::Chat => {
                if !self.show_chat_panel {
                    return None;
                }
                let horz = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                    .split(right_column);
                let chat_rect = horz[1];
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
                let v_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(2), Constraint::Length(1)])
                    .split(chat_inner);
                let input_area = v_chunks[1];
                if input_area.width == 0 || input_area.height == 0 {
                    return None;
                }
                let prompt_width = 3u16; // " ▸ "
                let input_width = self.chat_input.chars().count() as u16;
                let max_x = input_area.x + input_area.width.saturating_sub(1);
                let x = (input_area.x + prompt_width + input_width).min(max_x);
                Some((x, input_area.y))
            }
            _ => None,
        }
    }

    /// Render the UI and apply terminal cursor placement for active inputs.
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
        use layout::{HEADER_HEIGHT, STATUS_HEIGHT, TABS_HEIGHT};

        let outer = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.style_border_glow())
            .style(Style::default().bg(self.theme.bg));
        outer.render(area, buf);

        let padded = layout::content_area(area, true);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(HEADER_HEIGHT),
                Constraint::Length(TABS_HEIGHT),
                Constraint::Min(12),
                Constraint::Length(STATUS_HEIGHT),
            ])
            .split(padded);

        self.render_header(chunks[0], buf);
        self.render_tabs(chunks[1], buf);

        let body = chunks[2];
        let left_div_right = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Length(1),
                Constraint::Ratio(2, 3),
            ])
            .split(body);
        let left_column = left_div_right[0];
        let div_rect = left_div_right[1];
        let right_column = left_div_right[2];

        let left_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(6)])
            .split(left_column);
        let search_rect = left_split[0];
        let list_rect = left_split[1];

        let (inspector_rect, chat_rect) = if self.show_chat_panel {
            let horz = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(right_column);
            (horz[0], horz[1])
        } else {
            (right_column, right_column)
        };

        self.render_search(search_rect, buf);
        self.render_list(list_rect, buf);
        self.render_vertical_divider(div_rect, buf);
        self.render_inspector(inspector_rect, buf);
        if self.show_chat_panel {
            self.render_chat_panel(chat_rect, buf);
        }
        self.render_status(chunks[3], buf);
        self.render_completion(search_rect, buf);
        self.render_settings_overlay(area, buf);
        self.render_help_overlay(area, buf);
    }
}
