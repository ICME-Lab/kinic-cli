//! Header block: branding logo + live metrics + attribution.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

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
        let context_count = self.context_tree.len();
        let line2 = if let Some(bytes) = self.target_size_bytes {
            format!(
                "{} {} {} · {} {}",
                self.ui_config.header.contexts_icon,
                context_count,
                self.ui_config.header.contexts_suffix,
                self.ui_config.header.data_label,
                format_bytes(bytes)
            )
        } else {
            format!(
                "{} {} {}",
                self.ui_config.header.contexts_icon,
                context_count,
                self.ui_config.header.contexts_suffix
            )
        };
        let line3 = self.ui_config.branding.attribution.clone();

        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(20), Constraint::Min(30)])
            .split(area);
        let logo_area = header_chunks[0];
        let tagline_area = header_chunks[1];
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
}

fn resolve_logo_lines(branding: &crate::ui::app::BrandingText, max_height: u16) -> Vec<String> {
    let mut lines = branding.logo_lines.clone();
    if lines.is_empty() {
        lines.push("TUI".to_string());
    }
    lines.into_iter().take(max_height as usize).collect()
}
