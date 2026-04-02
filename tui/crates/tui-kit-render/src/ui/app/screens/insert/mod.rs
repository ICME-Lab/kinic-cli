//! Insert screen rendering and cursor geometry.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use tui_kit_runtime::{InsertFormFocus, InsertMode};
use unicode_width::UnicodeWidthStr;

use crate::ui::app::{Focus, TuiKitUi, shared};

use super::{FormRows, submit_button_text};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_insert_screen(&self, area: Rect, buf: &mut Buffer) {
        let layout = insert_layout(area, !self.tab_specs.is_empty());
        let form = insert_form_lines(self, layout.form_area.width.saturating_sub(6));
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

struct InsertLayout {
    form_area: Rect,
}

fn insert_layout(area: Rect, has_tabs: bool) -> InsertLayout {
    let body = shared::layout::body_rect_for_area_with_tabs(area, has_tabs);
    InsertLayout { form_area: body }
}

struct InsertForm<'a> {
    lines: Vec<Line<'a>>,
    rows: FormRows<InsertFormFocus>,
}

impl InsertForm<'_> {
    fn focus_row(&self, focus: InsertFormFocus) -> Option<u16> {
        self.rows.focus_row(focus)
    }

    fn focus_width(&self, focus: InsertFormFocus) -> u16 {
        self.rows.focus_width(focus)
    }
}

fn insert_form_lines<'a>(ui: &'a TuiKitUi<'a>, max_width: u16) -> InsertForm<'a> {
    let mut lines = Vec::new();
    let mut rows = FormRows::default();
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
        memory_id_value(ui),
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
    if matches!(
        ui.insert_mode,
        InsertMode::InlineText | InsertMode::ManualEmbedding
    ) {
        push_field(
            &mut lines,
            &mut rows,
            ui,
            InsertFormFocus::Text,
            text_label(ui),
            display_value(ui.insert_text, text_placeholder(ui.insert_mode)),
            max_width,
        );
    }
    if matches!(ui.insert_mode, InsertMode::File) {
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
    if matches!(ui.insert_mode, InsertMode::ManualEmbedding) {
        push_field(
            &mut lines,
            &mut rows,
            ui,
            InsertFormFocus::Embedding,
            ui.ui_config.insert.embedding_label.as_str(),
            display_value(ui.insert_embedding, "<json array>"),
            max_width,
        );
        if let Some(value) = expected_dim_value(ui) {
            push_readonly_field(&mut lines, ui, "Expected Dim", value);
        }
        if let Some(value) = ui.insert_current_dim {
            push_readonly_field(&mut lines, ui, "Current Dim", value.to_string());
        }
        if let Some(message) = ui.insert_validation_message {
            push_validation_message(&mut lines, ui, message);
        }
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
    lines.extend(
        ui.ui_config
            .insert
            .mode_help
            .lines()
            .map(|line| Line::from(Span::styled(line.to_string(), ui.theme.style_muted()))),
    );
    if let Some(error) = ui.insert_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            error.to_string(),
            ui.theme.style_error(),
        )));
    }
    InsertForm { lines, rows }
}

fn push_field(
    lines: &mut Vec<Line<'_>>,
    rows: &mut FormRows<InsertFormFocus>,
    ui: &TuiKitUi<'_>,
    focus: InsertFormFocus,
    label: &str,
    value: String,
    max_width: u16,
) {
    let display = trim_to_width(&value, max_width);
    if !label.is_empty() {
        rows.push_labeled_row(
            lines,
            Line::from(Span::styled(label.to_string(), ui.theme.style_dim())),
            focus,
            Line::from(vec![
                Span::raw("  "),
                Span::styled(display.clone(), field_style(ui, focus)),
            ]),
            display.as_str(),
        );
    } else {
        rows.push_unlabeled_row(
            lines,
            focus,
            Line::from(vec![
                Span::raw("  "),
                Span::styled(display.clone(), field_style(ui, focus)),
            ]),
            display.as_str(),
        );
    }
    lines.push(Line::from(""));
}

fn push_readonly_field(lines: &mut Vec<Line<'_>>, ui: &TuiKitUi<'_>, label: &str, value: String) {
    lines.push(Line::from(Span::styled(
        label.to_string(),
        ui.theme.style_dim(),
    )));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(value, ui.theme.style_muted()),
    ]));
    lines.push(Line::from(""));
}

fn push_validation_message(lines: &mut Vec<Line<'_>>, ui: &TuiKitUi<'_>, message: &str) {
    lines.push(Line::from(Span::styled(
        format!("  {message}"),
        ui.theme.style_error(),
    )));
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

fn memory_id_value(ui: &TuiKitUi<'_>) -> String {
    let placeholder = ui
        .insert_memory_placeholder
        .map(|label| format!("<default memory: {label}>"))
        .unwrap_or_else(|| "<target memory canister>".to_string());
    display_value(ui.insert_memory_id, &placeholder)
}

fn mode_value(mode: InsertMode) -> String {
    let (file, text, embedding) = match mode {
        InsertMode::File => ("[File]", " Inline Text ", " Manual Embedding "),
        InsertMode::InlineText => (" File ", "[Inline Text]", " Manual Embedding "),
        InsertMode::ManualEmbedding => (" File ", " Inline Text ", "[Manual Embedding]"),
    };
    format!("{file} / {text} / {embedding}")
}

fn text_label<'a>(ui: &'a TuiKitUi<'a>) -> &'a str {
    match ui.insert_mode {
        InsertMode::InlineText => ui.ui_config.insert.text_label.as_str(),
        InsertMode::ManualEmbedding => ui.ui_config.insert.payload_text_label.as_str(),
        InsertMode::File => {
            unreachable!("text label is only used for text-capable insert modes")
        }
    }
}

fn text_placeholder(mode: InsertMode) -> &'static str {
    match mode {
        InsertMode::InlineText => "<inline text>",
        InsertMode::ManualEmbedding => "<payload text stored with embedding>",
        _ => unreachable!("text placeholder is only used for text-capable insert modes"),
    }
}

fn expected_dim_value(ui: &TuiKitUi<'_>) -> Option<String> {
    if ui.insert_expected_dim_loading {
        return Some("loading...".to_string());
    }

    ui.insert_expected_dim.map(|value| value.to_string())
}

fn submit_value(ui: &TuiKitUi<'_>) -> String {
    submit_button_text(
        &ui.insert_submit_state,
        ui.insert_spinner_frame,
        ui.ui_config.insert.submit_label.as_str(),
        ui.ui_config.insert.submit_pending_label.as_str(),
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Theme;

    fn render_insert_form(mode: InsertMode) -> String {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme).insert_mode(mode);
        insert_form_lines(&ui, 80)
            .lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn memory_id_value_prefers_default_memory_placeholder_when_empty() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme).insert_memory_placeholder(Some("Alpha Memory"));

        assert_eq!(memory_id_value(&ui), "<default memory: Alpha Memory>");
    }

    #[test]
    fn memory_id_value_prefers_explicit_insert_memory_id() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .insert_memory_placeholder(Some("Alpha Memory"))
            .insert_memory_id("bbbbb-bb");

        assert_eq!(memory_id_value(&ui), "bbbbb-bb");
    }

    #[test]
    fn insert_form_toggles_fields_by_mode() {
        let cases = [
            (InsertMode::InlineText, true, false, false),
            (InsertMode::File, false, true, false),
            (InsertMode::ManualEmbedding, true, false, true),
        ];

        for (mode, has_inline_text, has_file_path, has_embedding) in cases {
            let lines = render_insert_form(mode);
            let has_text_placeholder = match mode {
                InsertMode::InlineText | InsertMode::ManualEmbedding => {
                    lines.contains(text_placeholder(mode))
                }
                InsertMode::File => false,
            };
            assert_eq!(has_text_placeholder, has_inline_text);
            assert_eq!(lines.contains("<file path>"), has_file_path);
            assert_eq!(lines.contains("<json array>"), has_embedding);
        }
    }

    #[test]
    fn insert_form_uses_payload_text_label_for_manual_embedding() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme).insert_mode(InsertMode::ManualEmbedding);
        let rendered = insert_form_lines(&ui, 80)
            .lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Payload Text"));
        assert!(!rendered.contains("Inline Text\n  <payload text stored with embedding>"));
    }

    #[test]
    fn insert_form_keeps_inline_text_label_for_inline_mode() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme).insert_mode(InsertMode::InlineText);
        let rendered = insert_form_lines(&ui, 80)
            .lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Inline Text"));
    }

    #[test]
    fn mode_value_uses_file_inline_text_manual_embedding_labels() {
        assert_eq!(
            mode_value(InsertMode::File),
            "[File] /  Inline Text  /  Manual Embedding "
        );
        assert_eq!(
            mode_value(InsertMode::InlineText),
            " File  / [Inline Text] /  Manual Embedding "
        );
        assert_eq!(
            mode_value(InsertMode::ManualEmbedding),
            " File  /  Inline Text  / [Manual Embedding]"
        );
    }

    #[test]
    fn submit_value_shows_spinner_while_insert_is_submitting() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .insert_submit_state(tui_kit_runtime::CreateSubmitState::Submitting)
            .insert_spinner_frame(1);

        assert_eq!(submit_value(&ui), "/ Inserting...");
    }

    #[test]
    fn expected_dim_value_prefers_loading_state() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .insert_mode(InsertMode::ManualEmbedding)
            .insert_expected_dim(Some(1024))
            .insert_expected_dim_loading(true);

        assert_eq!(expected_dim_value(&ui).as_deref(), Some("loading..."));
    }

    #[test]
    fn insert_form_shows_expected_dim_for_raw_mode() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .insert_mode(InsertMode::ManualEmbedding)
            .insert_expected_dim(Some(1024));
        let rendered = insert_form_lines(&ui, 80)
            .lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Expected Dim"));
        assert!(rendered.contains("1024"));
    }

    #[test]
    fn insert_form_shows_current_dim_for_raw_embedding() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .insert_mode(InsertMode::ManualEmbedding)
            .insert_current_dim(Some("3"));
        let rendered = insert_form_lines(&ui, 80)
            .lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Current Dim"));
        assert!(rendered.contains("3"));
    }

    #[test]
    fn insert_form_shows_invalid_current_dim_and_validation_message() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .insert_mode(InsertMode::ManualEmbedding)
            .insert_current_dim(Some("invalid"))
            .insert_validation_message(Some("Embedding must be a JSON array of floats."));
        let rendered = insert_form_lines(&ui, 80)
            .lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Current Dim"));
        assert!(rendered.contains("invalid"));
        assert!(rendered.contains("Embedding must be a JSON array of floats."));
    }

    #[test]
    fn insert_form_keeps_validation_message_and_submit_error_separate() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme)
            .insert_mode(InsertMode::ManualEmbedding)
            .insert_current_dim(Some("1"))
            .insert_validation_message(Some(
                "Embedding dimension mismatch. Received 1 values, expected 1024.",
            ))
            .insert_error(Some("Insert failed."));
        let rendered = insert_form_lines(&ui, 80)
            .lines
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            rendered.contains("Embedding dimension mismatch. Received 1 values, expected 1024.")
        );
        assert!(rendered.contains("Insert failed."));
    }
}
