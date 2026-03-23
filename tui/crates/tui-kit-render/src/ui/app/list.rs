//! List block: items list and context list.

use crate::ui::model::{UiItemKind, UiVisibility};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

use super::types::Focus;
use super::TuiKitUi;

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_list(&self, area: Rect, buf: &mut Buffer) {
        if self.show_context_panel && !self.in_context_items_view {
            self.render_context_list(area, buf);
            return;
        }

        let selected = self.list_selected;
        let highlight_intensity = self.animation.map(|a| a.selection_highlight).unwrap_or(1.0);
        let visible_height = area.height.saturating_sub(2) as usize;
        let total_items = self.ui_summaries.len();
        let scroll_offset = self.list_scroll_offset.unwrap_or_else(|| {
            if let Some(sel) = selected {
                if visible_height == 0 {
                    0
                } else if sel >= visible_height {
                    sel.saturating_sub(visible_height - 1)
                } else {
                    0
                }
            } else {
                0
            }
        });

        let items: Vec<ListItem> = self
            .ui_summaries
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|(idx, item)| {
                let is_selected = Some(idx) == selected;
                let kind_style = match item.kind {
                    UiItemKind::Function => self.theme.style_function(),
                    UiItemKind::Type => self.theme.style_type(),
                    UiItemKind::Trait => self.theme.style_keyword(),
                    UiItemKind::Module => self.theme.style_accent(),
                    UiItemKind::Constant => self.theme.style_string(),
                    _ => self.theme.style_dim(),
                };
                let base_style = if is_selected {
                    if highlight_intensity < 1.0 {
                        self.theme.style_selected().add_modifier(Modifier::BOLD)
                    } else {
                        self.theme.style_selected()
                    }
                } else {
                    self.theme.style_dim()
                };
                let prefix = if is_selected { "▸ " } else { "  " };
                let vis = match item.visibility {
                    UiVisibility::Public => "●",
                    UiVisibility::Internal => "◐",
                    UiVisibility::Private => "○",
                };
                let mut spans = vec![
                    Span::styled(
                        prefix,
                        if is_selected {
                            self.theme.style_accent()
                        } else {
                            self.theme.style_dim()
                        },
                    ),
                    Span::styled(vis, self.theme.style_dim()),
                    Span::raw(" "),
                ];
                if !item.kind.label().is_empty() {
                    spans.push(Span::styled(
                        format!("{:6} ", item.kind.label()),
                        if is_selected {
                            kind_style
                        } else {
                            self.theme.style_dim()
                        },
                    ));
                }
                spans.push(Span::styled(
                    item.name.clone(),
                    if is_selected {
                        self.theme.style_normal().add_modifier(Modifier::BOLD)
                    } else {
                        self.theme.style_dim()
                    },
                ));
                ListItem::new(Line::from(spans)).style(base_style)
            })
            .collect();

        let border_style = if self.focus == Focus::List {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let scroll_indicator = if total_items > visible_height {
            let pos = selected.unwrap_or(0) + 1;
            format!(" [{}/{}]", pos, total_items)
        } else {
            String::new()
        };

        let title = if self.show_context_panel && self.in_context_items_view {
            let (name, version) = self
                .ui_context_node
                .map(|n| {
                    (
                        n.name.clone(),
                        n.version.clone().unwrap_or_else(|| "?".to_string()),
                    )
                })
                .unwrap_or_else(|| ("context".to_string(), "?".to_string()));
            format!(
                " ◇ {} v{} ({} items){} [Esc] ",
                name, version, total_items, scroll_indicator
            )
        } else if self.search_input.is_empty() {
            format!(" Items ({}){} ", total_items, scroll_indicator)
        } else {
            format!(
                " Items ({}/{}){} ",
                total_items, self.ui_total_count, scroll_indicator
            )
        };

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

        let scrollbar_area = Rect {
            x: area.x + area.width.saturating_sub(1),
            y: area.y,
            width: 1,
            height: area.height,
        };
        if total_items > visible_height {
            self.render_custom_scrollbar(
                buf,
                scrollbar_area,
                total_items,
                visible_height.max(1),
                scroll_offset,
            );
        } else if scrollbar_area.height > 0 {
            let knob_y = scrollbar_area.y + scrollbar_area.height / 2;
            if let Some(cell) = buf.cell_mut((scrollbar_area.x, knob_y)) {
                cell.set_symbol("●")
                    .set_style(self.theme.style_muted().add_modifier(Modifier::DIM));
            }
        }
    }

    pub(super) fn render_context_list(&self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focus == Focus::List {
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
            vec![ListItem::new(Line::from(vec![
                Span::styled(
                    if is_selected { "▸ " } else { "  " },
                    self.theme.style_accent(),
                ),
                Span::styled("○ ", self.theme.style_muted()),
                Span::styled("No context data", self.theme.style_dim()),
            ]))
            .style(style)]
        } else if items_slice.is_empty() {
            let is_selected = selected == 0;
            let style = if is_selected {
                self.theme.style_selected()
            } else {
                Style::default()
            };
            vec![ListItem::new(Line::from(vec![
                Span::styled(
                    if is_selected { "▸ " } else { "  " },
                    self.theme.style_accent(),
                ),
                Span::styled("○ ", self.theme.style_muted()),
                Span::styled("No matches for search", self.theme.style_dim()),
            ]))
            .style(style)]
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
            self.render_custom_scrollbar(
                buf,
                scrollbar_area,
                total,
                visible_height.max(1),
                scroll_offset,
            );
        }
    }

    fn render_custom_scrollbar(
        &self,
        buf: &mut Buffer,
        area: Rect,
        total_items: usize,
        visible_items: usize,
        scroll_offset: usize,
    ) {
        if area.width == 0 || area.height == 0 || total_items == 0 {
            return;
        }

        // Draw a dim track first.
        for y in area.y..area.y + area.height {
            if let Some(cell) = buf.cell_mut((area.x, y)) {
                cell.set_symbol("│")
                    .set_style(self.theme.style_muted().add_modifier(Modifier::DIM));
            }
        }

        if total_items <= visible_items {
            return;
        }

        let h = area.height as usize;
        let max_scroll = total_items.saturating_sub(visible_items);
        let thumb_h = ((visible_items * h) / total_items).max(1).min(h);
        let max_start = h.saturating_sub(thumb_h);
        let thumb_start = if max_scroll == 0 {
            0
        } else {
            (scroll_offset.min(max_scroll) * max_start) / max_scroll
        };

        for i in 0..thumb_h {
            let y = area.y + (thumb_start + i) as u16;
            if let Some(cell) = buf.cell_mut((area.x, y)) {
                cell.set_symbol("█").set_style(self.theme.style_accent());
            }
        }
    }
}
