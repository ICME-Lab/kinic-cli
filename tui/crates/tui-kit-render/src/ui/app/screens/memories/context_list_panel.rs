//! Context list panel for the memories screen.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

use crate::ui::app::{Focus, TuiKitUi};

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_context_list_panel(&self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focus == Focus::Items {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let visible_height = area.height.saturating_sub(2) as usize;
        let selected = self.list_selected.unwrap_or(0);
        let (items_slice, total) =
            if self.context_tree.is_empty() || self.filtered_context_indices.is_empty() {
                (&[][..], 1usize)
            } else {
                let indices = self.filtered_context_indices;
                (indices, indices.len())
            };
        let total = total.max(1);
        let scroll_offset = if visible_height == 0 {
            0
        } else if selected >= visible_height {
            selected.saturating_sub(visible_height - 1)
        } else {
            0
        };

        let items: Vec<ListItem> = if self.context_tree.is_empty() {
            let is_selected = selected == 0;
            let style = if is_selected {
                self.theme.style_selected()
            } else {
                Style::default()
            };
            vec![
                ListItem::new(Line::from(vec![
                    Span::styled(
                        if is_selected { "▸ " } else { "  " },
                        self.theme.style_accent(),
                    ),
                    Span::styled("○ ", self.theme.style_muted()),
                    Span::styled("No context data", self.theme.style_dim()),
                ]))
                .style(style),
            ]
        } else if items_slice.is_empty() {
            let is_selected = selected == 0;
            let style = if is_selected {
                self.theme.style_selected()
            } else {
                Style::default()
            };
            vec![
                ListItem::new(Line::from(vec![
                    Span::styled(
                        if is_selected { "▸ " } else { "  " },
                        self.theme.style_accent(),
                    ),
                    Span::styled("○ ", self.theme.style_muted()),
                    Span::styled("No matches for search", self.theme.style_dim()),
                ]))
                .style(style),
            ]
        } else {
            items_slice
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(visible_height)
                .map(|(display_idx, &tree_idx)| {
                    let (name, _) = &self.context_tree[tree_idx];
                    let is_selected = Some(display_idx) == self.list_selected;
                    let style = if is_selected {
                        self.theme.style_selected()
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            if is_selected { "▸ " } else { "  " },
                            self.theme.style_accent(),
                        ),
                        Span::styled("◇ ", self.theme.style_dim()),
                        Span::styled(name.clone(), self.theme.style_normal()),
                    ]))
                    .style(style)
                })
                .collect()
        };

        let scroll_info = if total > visible_height {
            format!(" [{}/{}]", selected + 1, total)
        } else {
            String::new()
        };
        let title = format!(" Context ({}){} ", total, scroll_info);
        let list_area = Rect {
            width: area.width.saturating_sub(1),
            ..area
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(Style::default().bg(self.theme.bg_panel))
                    .title(title),
            )
            .highlight_style(self.theme.style_selected())
            .highlight_symbol("▸ ");
        Widget::render(list, list_area, buf);

        if total > visible_height {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y,
                width: 1,
                height: area.height,
            };
            self.render_list_scrollbar(
                buf,
                scrollbar_area,
                total,
                visible_height.max(1),
                scroll_offset,
            );
        }
    }
}
