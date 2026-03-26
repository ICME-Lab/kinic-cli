//! Insert screen rendering and cursor geometry.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{CreateSubmitState, InsertFormFocus, InsertMode};
use unicode_width::UnicodeWidthStr;

use crate::ui::app::{Focus, TuiKitUi, shared};

const MODE_OPTIONS: &str = "normal / raw / pdf";

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_insert_screen(&self, area: Rect, buf: &mut Buffer) {
        let layout = insert_layout(area, !self.tab_specs.is_empty());
        let form = insert_form_lines(self, layout.form_area.width.saturating_sub(6));
        let intro = insert_intro_lines(self, layout.intro_area.height);
        Paragraph::new(intro)
            .wrap(Wrap { trim: false })
            .render(layout.intro_area, buf);
        Paragraph::new(form.lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if self.focus == Focus::Form {
                        self.theme.style_border_focused()
                    } else {
                        self.theme.style_border()
                    })
                    .title(format!(" {} ", self.ui_config.insert.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .render(layout.form_area, buf);
    }

    pub(crate) fn insert_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if self.focus != Focus::Form {
            return None;
        }
        let layout = insert_layout(area, !self.tab_specs.is_empty());
        let form = insert_form_lines(self, layout.form_area.width.saturating_sub(6));
        let row = form.focus_row(self.insert_focus)?;
        let width = form.focus_width(self.insert_focus);
        Some((
            (layout.form_area.x + 3 + width).min(layout.form_area.right().saturating_sub(2)),
            layout.form_area.y + 1 + row,
        ))
    }
}

fn insert_intro_lines<'a>(ui: &'a TuiKitUi<'a>, intro_height: u16) -> Vec<Line<'a>> {
    let mut intro = vec![
        Line::from(vec![
            Span::styled(" Insert ", ui.theme.style_accent_bold()),
            Span::styled(
                ui.ui_config.insert.intro_description.as_str(),
                ui.theme.style_normal(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Enter ", ui.theme.style_accent()),
            Span::styled("mode or submit", ui.theme.style_muted()),
            Span::styled("  │  ", ui.theme.style_dim()),
            Span::styled(" Tab / Shift+Tab ", ui.theme.style_accent()),
            Span::styled("cycle fields", ui.theme.style_muted()),
        ]),
    ];

    if intro_height >= 5 {
        intro[2] = Line::from(vec![
            Span::styled(" Enter ", ui.theme.style_accent()),
            Span::styled("mode or submit", ui.theme.style_muted()),
            Span::styled("  │  ", ui.theme.style_dim()),
            Span::styled(" Tab / Shift+Tab ", ui.theme.style_accent()),
            Span::styled("cycle fields", ui.theme.style_muted()),
            Span::styled("  │  ", ui.theme.style_dim()),
            Span::styled(" Esc ", ui.theme.style_accent()),
            Span::styled("step back one level", ui.theme.style_muted()),
        ]);
        intro.push(Line::from(vec![
            Span::styled(" While submitting ", ui.theme.style_accent()),
            Span::styled("wait for completion before editing", ui.theme.style_muted()),
        ]));
    }

    intro
}

#[cfg(test)]
fn insert_intro_is_compact(intro_height: u16) -> bool {
    intro_height < 5
}

struct InsertLayout {
    intro_area: Rect,
    form_area: Rect,
}

fn insert_intro_height(body_height: u16) -> u16 {
    if body_height < 16 { 4 } else { 6 }
}

fn insert_layout(area: Rect, has_tabs: bool) -> InsertLayout {
    let body = shared::layout::body_rect_for_area_with_tabs(area, has_tabs);
    let intro_height = insert_intro_height(body.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(intro_height), Constraint::Min(10)])
        .split(body);
    InsertLayout {
        intro_area: chunks[0],
        form_area: chunks[1],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_intro_is_used_for_short_bodies() {
        assert!(insert_intro_is_compact(insert_intro_height(12)));
        assert!(!insert_intro_is_compact(insert_intro_height(20)));
    }

    #[test]
    fn compact_layout_preserves_room_for_form() {
        let area = Rect::new(0, 0, 80, 29);
        let body = shared::layout::body_rect_for_area_with_tabs(area, true);
        let layout = insert_layout(area, true);

        assert_eq!(body.height, 14);
        assert_eq!(layout.intro_area.height, 4);
        assert!(layout.form_area.height >= 10);
    }
}

struct InsertForm<'a> {
    lines: Vec<Line<'a>>,
    rows: Vec<(InsertFormFocus, u16, u16)>,
}

impl InsertForm<'_> {
    fn focus_row(&self, focus: InsertFormFocus) -> Option<u16> {
        self.rows.iter().find(|(field, _, _)| *field == focus).map(|(_, row, _)| *row)
    }

    fn focus_width(&self, focus: InsertFormFocus) -> u16 {
        self.rows
            .iter()
            .find(|(field, _, _)| *field == focus)
            .map(|(_, _, width)| *width)
            .unwrap_or(0)
    }
}

fn insert_form_lines<'a>(ui: &'a TuiKitUi<'a>, max_width: u16) -> InsertForm<'a> {
    let mut lines = Vec::new();
    let mut rows = Vec::new();
    push_field(
        &mut lines,
        &mut rows,
        ui,
        InsertFormFocus::Mode,
        ui.ui_config.insert.mode_label.as_str(),
        mode_value(ui.insert_mode),
        max_width,
    );
    push_field(
        &mut lines,
        &mut rows,
        ui,
        InsertFormFocus::MemoryId,
        ui.ui_config.insert.memory_id_label.as_str(),
        display_value(ui.insert_memory_id, "<target memory canister>"),
        max_width,
    );
    push_field(
        &mut lines,
        &mut rows,
        ui,
        InsertFormFocus::Tag,
        ui.ui_config.insert.tag_label.as_str(),
        display_value(ui.insert_tag, "<tag>"),
        max_width,
    );
    if matches!(ui.insert_mode, InsertMode::Normal | InsertMode::Raw) {
        push_field(
            &mut lines,
            &mut rows,
            ui,
            InsertFormFocus::Text,
            ui.ui_config.insert.text_label.as_str(),
            display_value(ui.insert_text, "<inline text>"),
            max_width,
        );
    }
    if matches!(ui.insert_mode, InsertMode::Normal | InsertMode::Pdf) {
        push_field(
            &mut lines,
            &mut rows,
            ui,
            InsertFormFocus::FilePath,
            ui.ui_config.insert.file_path_label.as_str(),
            display_value(ui.insert_file_path, "<file path>"),
            max_width,
        );
    }
    if matches!(ui.insert_mode, InsertMode::Raw) {
        push_field(
            &mut lines,
            &mut rows,
            ui,
            InsertFormFocus::Embedding,
            ui.ui_config.insert.embedding_label.as_str(),
            display_value(ui.insert_embedding, "<json array>"),
            max_width,
        );
    }
    push_field(
        &mut lines,
        &mut rows,
        ui,
        InsertFormFocus::Submit,
        "",
        submit_value(ui),
        max_width,
    );
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        ui.ui_config.insert.close_hint.as_str(),
        ui.theme.style_muted(),
    )));
    if let Some(error) = ui.insert_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(error.to_string(), ui.theme.style_error())));
    }
    InsertForm { lines, rows }
}

fn push_field(
    lines: &mut Vec<Line<'_>>,
    rows: &mut Vec<(InsertFormFocus, u16, u16)>,
    ui: &TuiKitUi<'_>,
    focus: InsertFormFocus,
    label: &str,
    value: String,
    max_width: u16,
) {
    if !label.is_empty() {
        lines.push(Line::from(Span::styled(label.to_string(), ui.theme.style_dim())));
    }
    let display = trim_to_width(&value, max_width);
    let row = u16::try_from(lines.len()).unwrap_or(0);
    let width = UnicodeWidthStr::width(display.as_str()) as u16;
    rows.push((focus, row, width));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(display, field_style(ui, focus)),
    ]));
    lines.push(Line::from(""));
}

fn field_style(ui: &TuiKitUi<'_>, focus: InsertFormFocus) -> ratatui::style::Style {
    if ui.focus == Focus::Form && ui.insert_focus == focus {
        ui.theme.style_accent_bold()
    } else if ui.focus == Focus::Tabs && focus == InsertFormFocus::Mode {
        ui.theme.style_border_focused()
    } else {
        ui.theme.style_normal()
    }
}

fn display_value(value: &str, placeholder: &str) -> String {
    if value.is_empty() {
        placeholder.to_string()
    } else {
        value.to_string()
    }
}

fn mode_value(mode: InsertMode) -> String {
    match mode {
        InsertMode::Normal => format!("[normal]   {MODE_OPTIONS}"),
        InsertMode::Raw => format!("normal / [raw] / pdf"),
        InsertMode::Pdf => format!("normal / raw / [pdf]"),
    }
}

fn submit_value(ui: &TuiKitUi<'_>) -> String {
    match ui.insert_submit_state {
        CreateSubmitState::Submitting => ui.ui_config.insert.submit_pending_label.clone(),
        CreateSubmitState::Idle | CreateSubmitState::Error => ui.ui_config.insert.submit_label.clone(),
    }
}

fn trim_to_width(value: &str, max_width: u16) -> String {
    if max_width == 0 || UnicodeWidthStr::width(value) as u16 <= max_width {
        return value.to_string();
    }
    let mut trimmed = String::new();
    for ch in value.chars() {
        let next = format!("{trimmed}{ch}");
        if UnicodeWidthStr::width(next.as_str()) as u16 >= max_width.saturating_sub(1) {
            break;
        }
        trimmed.push(ch);
    }
    format!("{trimmed}…")
}
