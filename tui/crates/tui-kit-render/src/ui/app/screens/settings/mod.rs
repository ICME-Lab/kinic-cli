//! Settings screen rendering.
//! Where: full-body Settings tab content inside the shared app shell.
//! What: renders current session status and saved preferences from one snapshot.
//! Why: keeps the Settings tab aligned with the Shift+S quick status overlay.

use ratatui::{
    buffer::Buffer,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::SettingsSnapshot;

use crate::ui::app::{Focus, TuiKitUi};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_settings_screen(&self, area: ratatui::layout::Rect, buf: &mut Buffer) {
        let lines = settings_screen_lines(self.settings_snapshot);
        Paragraph::new(
            lines
                .into_iter()
                .map(|line| {
                    if line.starts_with("## ") {
                        Line::from(Span::styled(
                            line.trim_start_matches("## ").to_string(),
                            self.theme.style_accent_bold(),
                        ))
                    } else if line.starts_with("note: ") {
                        Line::from(Span::styled(
                            line.trim_start_matches("note: ").to_string(),
                            self.theme.style_muted(),
                        ))
                    } else if line.is_empty() {
                        Line::from("")
                    } else {
                        Line::from(Span::styled(line, self.theme.style_normal()))
                    }
                })
                .collect::<Vec<_>>(),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if self.focus == Focus::Tabs {
                    self.theme.style_border()
                } else {
                    self.theme.style_border_focused()
                })
                .title(" Settings ")
                .style(Style::default().bg(self.theme.bg_panel)),
        )
        .wrap(Wrap { trim: false })
        .render(area, buf);
    }
}

fn settings_screen_lines(snapshot: Option<&SettingsSnapshot>) -> Vec<String> {
    let mut lines = vec![
        "Read-only session status first, with a small saved preference set below.".to_string(),
        String::new(),
        "Shift+S shows quick status. This tab shows the detailed view.".to_string(),
        String::new(),
    ];

    let Some(snapshot) = snapshot else {
        lines.push("No settings data available yet.".to_string());
        return lines;
    };

    for section in &snapshot.sections {
        lines.push(format!("## {}", section.title));
        for entry in &section.entries {
            lines.push(format!("  {}: {}", entry.label, entry.value));
            if let Some(note) = &entry.note {
                lines.push(format!("note:   {note}"));
            }
        }
        if let Some(footer) = &section.footer {
            lines.push(format!("note: {footer}"));
        }
        lines.push(String::new());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::{SettingsEntry, SettingsSection, SettingsSnapshot};

    #[test]
    fn settings_screen_lines_show_sections_and_status_notes() {
        let snapshot = SettingsSnapshot {
            quick_entries: vec![],
            sections: vec![
                SettingsSection {
                    title: "Current session".to_string(),
                    entries: vec![SettingsEntry {
                        label: "Principal ID".to_string(),
                        value: "unavailable".to_string(),
                        note: Some("read only".to_string()),
                    }],
                    footer: None,
                },
                SettingsSection {
                    title: "Saved preferences".to_string(),
                    entries: vec![SettingsEntry {
                        label: "Preferred network".to_string(),
                        value: "coming soon".to_string(),
                        note: Some("No persisted network preference is stored in v1.".to_string()),
                    }],
                    footer: None,
                },
            ],
        };

        let lines = settings_screen_lines(Some(&snapshot)).join("\n");

        assert!(lines.contains("## Current session"));
        assert!(lines.contains("Principal ID: unavailable"));
        assert!(lines.contains("read only"));
        assert!(lines.contains("Preferred network: coming soon"));
        assert!(lines.contains("Shift+S shows quick status"));
    }
}
