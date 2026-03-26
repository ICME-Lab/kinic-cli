//! Status bar (footer) block: context-specific commands and counts.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use tui_kit_runtime::kinic_tabs::{KINIC_SETTINGS_TAB_ID, TabKind, tab_kind};

use crate::ui::app::{Focus, TuiKitUi};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let cfg = &self.ui_config.status;
        let tab_id = self.current_tab_id.0.as_str();
        let focus_indicator = match self.focus {
            Focus::Search => ("🔍", "Search"),
            Focus::Items => ("📋", "Items"),
            Focus::Tabs => ("🗂", "Tabs"),
            Focus::Content => ("🔬", "Content"),
            Focus::Form => ("✍", "Form"),
            Focus::Chat => ("💬", "Chat"),
        };

        let status_line = if matches!(tab_kind(tab_id), TabKind::InsertForm | TabKind::CreateForm) {
            let tab_label = self
                .tab_specs
                .iter()
                .find(|tab| tab.id == self.current_tab_id)
                .map(|tab| tab.title.as_str())
                .unwrap_or("Form");
            let mut spans = vec![
                Span::styled(tab_label, self.theme.style_accent_bold()),
                Span::styled(" │ ", self.theme.style_dim()),
                Span::styled("Tab/Shift+Tab", self.theme.style_accent()),
                Span::styled(" fields ", self.theme.style_muted()),
                Span::styled("Enter", self.theme.style_accent()),
                Span::styled(" enter/submit ", self.theme.style_muted()),
                Span::styled("1-5", self.theme.style_accent()),
                Span::styled(format!(" {} ", cfg.tabs_label), self.theme.style_muted()),
                Span::styled("│ ", self.theme.style_dim()),
                Span::styled(focus_indicator.0, self.theme.style_accent()),
                Span::styled(format!(" {}", focus_indicator.1), self.theme.style_dim()),
            ];
            if self.focus == Focus::Form {
                spans.splice(
                    6..6,
                    [
                        Span::styled("Esc", self.theme.style_accent()),
                        Span::styled(" tabs ", self.theme.style_muted()),
                    ],
                );
            }
            Line::from(spans)
        } else if matches!(
            tab_kind(tab_id),
            TabKind::PlaceholderMarket | TabKind::PlaceholderSettings
        ) {
            if tab_id == KINIC_SETTINGS_TAB_ID && self.focus == Focus::Content {
                Line::from(vec![
                    Span::styled("↑/↓", self.theme.style_accent()),
                    Span::styled(" choose row ", self.theme.style_muted()),
                    Span::styled("Enter", self.theme.style_accent()),
                    Span::styled(" open Default memory ", self.theme.style_muted()),
                    Span::styled("Esc", self.theme.style_accent()),
                    Span::styled(" tabs ", self.theme.style_muted()),
                    Span::styled("│ ", self.theme.style_dim()),
                    Span::styled("Shift+D", self.theme.style_accent()),
                    Span::styled(" saves current memory from Memories", self.theme.style_muted()),
                ])
            } else {
                let mut spans = vec![
                    Span::styled("Tab", self.theme.style_accent()),
                    Span::styled(" switch ", self.theme.style_muted()),
                    Span::styled("1-5", self.theme.style_accent()),
                    Span::styled(format!(" {} ", cfg.tabs_label), self.theme.style_muted()),
                    Span::styled("│ ", self.theme.style_dim()),
                    Span::styled(focus_indicator.0, self.theme.style_accent()),
                    Span::styled(format!(" {}", focus_indicator.1), self.theme.style_dim()),
                ];
                if self.focus == Focus::Content {
                    spans.splice(
                        4..4,
                        [
                            Span::styled("Esc", self.theme.style_accent()),
                            Span::styled(" tabs ", self.theme.style_muted()),
                        ],
                    );
                }
                Line::from(spans)
            }
        } else if self.show_context_panel && self.in_context_items_view {
            let selection_info = if let Some(selected) = self.list_selected {
                format!("[{}/{}]", selected + 1, self.ui_summaries.len())
            } else {
                format!("[0/{}]", self.ui_summaries.len())
            };
            let context_name = self
                .ui_context_node
                .map(|n| n.name.clone())
                .unwrap_or_else(|| "context".to_string());
            let context_ver = self
                .ui_context_node
                .and_then(|n| n.version.clone())
                .unwrap_or_else(|| "?".to_string());
            Line::from(vec![
                Span::styled(" ◇ ", self.theme.style_accent()),
                Span::styled(context_name, self.theme.style_normal()),
                Span::styled(format!(" v{}", context_ver), self.theme.style_dim()),
                Span::styled(" │ ", self.theme.style_muted()),
                Span::styled(selection_info, self.theme.style_muted()),
                Span::styled(" │ Tab ↑/↓ Enter / Esc back ", self.theme.style_muted()),
            ])
        } else if self.show_context_panel {
            Line::from(vec![
                Span::styled(format!("{}: ", cfg.commands_label), self.theme.style_dim()),
                Span::styled("Tab", self.theme.style_accent()),
                Span::styled(" focus ", self.theme.style_muted()),
                Span::styled("↑/↓", self.theme.style_accent()),
                Span::styled(" list ", self.theme.style_muted()),
                Span::styled("Enter →", self.theme.style_accent()),
                Span::styled(" open ", self.theme.style_muted()),
                Span::styled("/", self.theme.style_accent()),
                Span::styled(" search ", self.theme.style_muted()),
                Span::styled("│ ", self.theme.style_dim()),
                Span::styled(
                    format!("◇ Context ({}) ", self.filtered_context_indices.len()),
                    self.theme.style_normal(),
                ),
            ])
        } else if !self.status_message.is_empty() {
            Line::from(vec![
                Span::styled(
                    format!(" {} ", self.status_message),
                    self.theme.style_string(),
                ),
                Span::styled(" │ ", self.theme.style_muted()),
                Span::styled("Tab", self.theme.style_accent()),
                Span::styled(" focus ", self.theme.style_muted()),
                Span::styled("↑/↓ Enter / ", self.theme.style_accent()),
                Span::styled(
                    format!("? help q {} ", cfg.quit_label),
                    self.theme.style_muted(),
                ),
                Span::styled("│ ", self.theme.style_dim()),
                Span::styled(focus_indicator.0, self.theme.style_accent()),
                Span::styled(format!(" {}", focus_indicator.1), self.theme.style_dim()),
            ])
        } else {
            let selection_info = if let Some(selected) = self.list_selected {
                format!("[{}/{}]", selected + 1, self.ui_summaries.len())
            } else {
                format!("[0/{}]", self.ui_summaries.len())
            };
            let mut spans = vec![
                Span::styled(format!("{}: ", cfg.commands_label), self.theme.style_dim()),
                Span::styled("Tab", self.theme.style_accent()),
                Span::styled(" focus ", self.theme.style_muted()),
                Span::styled("↑/↓", self.theme.style_accent()),
                Span::styled(" list ", self.theme.style_muted()),
                Span::styled("Enter →", self.theme.style_accent()),
                Span::styled(" open ", self.theme.style_muted()),
                Span::styled("/", self.theme.style_accent()),
                Span::styled(" search ", self.theme.style_muted()),
            ];
            if !self.tab_specs.is_empty() {
                spans.push(Span::styled("1-5", self.theme.style_accent()));
                spans.push(Span::styled(
                    format!(" {} ", cfg.tabs_label),
                    self.theme.style_muted(),
                ));
            }
            spans.extend([
                Span::styled("? ", self.theme.style_accent()),
                Span::styled("help ", self.theme.style_muted()),
                Span::styled("q ", self.theme.style_accent()),
                Span::styled(format!("{} ", cfg.quit_label), self.theme.style_muted()),
                Span::styled("│ ", self.theme.style_dim()),
                Span::styled("visible ", self.theme.style_muted()),
                Span::styled(
                    format!("{}/{} ", self.ui_summaries.len(), self.ui_total_count),
                    self.theme.style_normal(),
                ),
                Span::styled("selection ", self.theme.style_muted()),
                Span::styled(selection_info, self.theme.style_dim()),
            ]);
            Line::from(spans)
        };

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(self.theme.style_border())
            .style(Style::default().bg(self.theme.bg_panel));
        let inner = block.inner(area);
        block.render(area, buf);
        Paragraph::new(status_line)
            .alignment(Alignment::Left)
            .render(inner, buf);
    }
}
