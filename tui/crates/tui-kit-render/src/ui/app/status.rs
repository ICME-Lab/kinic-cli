//! Status bar (footer) block: context-specific commands and counts.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::types::Focus;
use super::TuiKitUi;

impl<'a> TuiKitUi<'a> {
    pub(super) fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let cfg = &self.ui_config.status;
        let focus_indicator = match self.focus {
            Focus::Search => ("🔍", "Search"),
            Focus::List => ("📋", "List"),
            Focus::Tabs => ("🗂", "Tabs"),
            Focus::Inspector => ("🔬", "Content"),
            Focus::Chat => ("💬", "Chat"),
        };

        let status_line = if self.show_context_panel && self.in_context_items_view {
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
                Span::styled("│ ", self.theme.style_dim()),
                Span::styled(" [g] ", self.theme.style_accent()),
                Span::styled(format!("{} ", cfg.github_label), self.theme.style_muted()),
                Span::styled("[s] ", self.theme.style_accent()),
                Span::styled(cfg.sponsor_label.clone(), self.theme.style_muted()),
            ])
        } else if self.show_context_panel {
            Line::from(vec![
                Span::styled(format!("{}: ", cfg.commands_label), self.theme.style_dim()),
                Span::styled("Tab", self.theme.style_accent()),
                Span::styled(" focus ", self.theme.style_muted()),
                Span::styled("↑/↓ j/k", self.theme.style_accent()),
                Span::styled(" list ", self.theme.style_muted()),
                Span::styled("Enter →", self.theme.style_accent()),
                Span::styled(" open ", self.theme.style_muted()),
                Span::styled("/", self.theme.style_accent()),
                Span::styled(" search ", self.theme.style_muted()),
                Span::styled("[o]", self.theme.style_accent()),
                Span::styled(" primary ", self.theme.style_muted()),
                Span::styled("[c]", self.theme.style_accent()),
                Span::styled(" secondary ", self.theme.style_muted()),
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
            Line::from(vec![
                Span::styled(format!("{}: ", cfg.commands_label), self.theme.style_dim()),
                Span::styled("Tab", self.theme.style_accent()),
                Span::styled(" focus ", self.theme.style_muted()),
                Span::styled("↑/↓ j/k", self.theme.style_accent()),
                Span::styled(" list ", self.theme.style_muted()),
                Span::styled("Enter →", self.theme.style_accent()),
                Span::styled(" open ", self.theme.style_muted()),
                Span::styled("/", self.theme.style_accent()),
                Span::styled(" search ", self.theme.style_muted()),
                Span::styled("1-4", self.theme.style_accent()),
                Span::styled(format!(" {} ", cfg.tabs_label), self.theme.style_muted()),
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
            ])
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
