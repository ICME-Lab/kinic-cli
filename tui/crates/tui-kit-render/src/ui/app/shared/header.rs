//! Header block: branding logo + live metrics + attribution.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use tui_kit_runtime::kinic_tabs::{TabKind, tab_kind};
use unicode_width::UnicodeWidthStr;

use crate::ui::app::TuiKitUi;

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let line1 = format!(
            "{} {} {}",
            self.ui_config.header.visible_icon,
            self.ui_summaries.len(),
            self.ui_config.header.visible_suffix
        );
        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(20), Constraint::Min(30)])
            .split(area);
        let logo_area = header_chunks[0];
        let tagline_area = header_chunks[1];
        let line2 = self.current_memory_line(tagline_area.width);
        let line3 = self.ui_config.branding.attribution.clone();
        let logo_lines: Vec<Line> = resolve_logo_lines(&self.ui_config.branding, logo_area.height)
            .into_iter()
            .map(|s| Line::from(Span::styled(s, self.theme.style_accent())))
            .collect();
        Paragraph::new(logo_lines).render(logo_area, buf);

        let row_height = tagline_area.height / 3;
        let tagline_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(row_height),
                Constraint::Length(row_height),
                Constraint::Length(tagline_area.height.saturating_sub(2 * row_height)),
            ])
            .split(tagline_area);

        let lines_content = [line1, line2, line3];
        for (i, content) in lines_content.iter().enumerate() {
            if let Some(rect) = tagline_rows.get(i) {
                let line = Line::from(Span::styled(content.as_str(), self.theme.style_dim()));
                Paragraph::new(line)
                    .alignment(Alignment::Right)
                    .render(*rect, buf);
            }
        }
    }

    fn current_memory_line(&self, max_width: u16) -> String {
        let base = match self.current_memory_name() {
            Some(name) => format!(
                "{} {}: {}",
                self.ui_config.header.contexts_icon, self.ui_config.header.memory_prefix, name
            ),
            None => format!(
                "{} {}",
                self.ui_config.header.contexts_icon, self.ui_config.header.memory_empty_label
            ),
        };
        let with_cache = match self.target_size_bytes {
            Some(bytes) => format!(
                "{} · {} {}",
                base,
                self.ui_config.header.data_label,
                format_bytes(bytes)
            ),
            None => base,
        };

        trim_to_width(with_cache.as_str(), max_width)
    }

    fn current_memory_name(&self) -> Option<&str> {
        match tab_kind(self.current_tab_id.0.as_str()) {
            TabKind::Memories => self
                .ui_selected_content
                .map(|content| content.title.as_str())
                .or_else(|| {
                    self.list_selected
                        .and_then(|index| self.ui_summaries.get(index))
                        .map(|summary| summary.name.as_str())
                }),
            TabKind::InsertForm => {
                let insert_memory_id = self.insert_memory_id.trim();
                if !insert_memory_id.is_empty() {
                    Some(insert_memory_id)
                } else {
                    self.insert_memory_placeholder
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                }
            }
            _ => self
                .selected_memory_label
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        }
    }
}

fn resolve_logo_lines(branding: &crate::ui::app::BrandingText, max_height: u16) -> Vec<String> {
    let mut lines = branding.logo_lines.clone();
    if lines.is_empty() {
        lines.push("TUI".to_string());
    }
    lines.into_iter().take(max_height as usize).collect()
}

fn trim_to_width(value: &str, max_width: u16) -> String {
    if max_width == 0 || UnicodeWidthStr::width(value) as u16 <= max_width {
        return value.to_string();
    }

    let mut out = String::new();
    for ch in value.chars() {
        let mut next = out.clone();
        next.push(ch);
        if UnicodeWidthStr::width(next.as_str()) as u16 >= max_width.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{TabId, TuiKitUi, theme::Theme};
    use tui_kit_model::{UiItemContent, UiItemKind, UiItemSummary, UiVisibility};
    use tui_kit_runtime::kinic_tabs::{KINIC_INSERT_TAB_ID, KINIC_MEMORIES_TAB_ID};

    #[test]
    fn current_memory_line_uses_selected_memory_title() {
        let theme = Theme::default();
        let selected_content = UiItemContent {
            id: "aaaaa-aa".to_string(),
            title: "Alpha Memory".to_string(),
            subtitle: None,
            kind: UiItemKind::Custom("memory".to_string()),
            definition: String::new(),
            location: None,
            docs: None,
            badges: Vec::new(),
            sections: Vec::new(),
        };

        let line = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_MEMORIES_TAB_ID))
            .ui_selected_content(Some(&selected_content))
            .current_memory_line(80);

        assert_eq!(line, "📚 memory: Alpha Memory");
    }

    #[test]
    fn current_memory_line_falls_back_to_no_selection_copy() {
        let theme = Theme::default();

        let line = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_MEMORIES_TAB_ID))
            .current_memory_line(80);

        assert_eq!(line, "📚 no memory selected");
    }

    #[test]
    fn current_memory_line_uses_insert_target_placeholder() {
        let theme = Theme::default();

        let line = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_INSERT_TAB_ID))
            .insert_memory_placeholder(Some("Default Memory"))
            .current_memory_line(80);

        assert_eq!(line, "📚 memory: Default Memory");
    }

    #[test]
    fn current_memory_line_keeps_selected_memory_on_other_tabs() {
        let theme = Theme::default();

        let line = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new("kinic-settings"))
            .selected_memory_label(Some("Alpha Memory"))
            .current_memory_line(80);

        assert_eq!(line, "📚 memory: Alpha Memory");
    }

    #[test]
    fn current_memory_line_trims_long_output() {
        let theme = Theme::default();
        let items = vec![UiItemSummary {
            id: "aaaaa-aa".to_string(),
            name: "Memory With A Very Long Name".to_string(),
            leading_marker: None,
            kind: UiItemKind::Custom("memory".to_string()),
            visibility: UiVisibility::Private,
            qualified_name: None,
            subtitle: None,
            tags: Vec::new(),
        }];

        let line = TuiKitUi::new(&theme)
            .current_tab_id(TabId::new(KINIC_MEMORIES_TAB_ID))
            .ui_summaries(&items)
            .list_selected(Some(0))
            .current_memory_line(18);

        assert!(line.ends_with('…'));
        assert!(UnicodeWidthStr::width(line.as_str()) <= 18_usize);
    }
}
