//! Create screen rendering and cursor geometry.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::CreateModalFocus;
use unicode_width::UnicodeWidthStr;

use crate::ui::app::{Focus, TuiKitUi, shared};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_create_screen(&self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(12)])
            .split(area);
        let intro = vec![
            Line::from(vec![
                Span::styled(" Create ", self.theme.style_accent_bold()),
                Span::styled(
                    "Provision a new memory canister without leaving the tab view.",
                    self.theme.style_normal(),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Enter ", self.theme.style_accent()),
                Span::styled(
                    "enter the form when tabs are focused",
                    self.theme.style_muted(),
                ),
                Span::styled("  │  ", self.theme.style_dim()),
                Span::styled(" Tab / Shift+Tab ", self.theme.style_accent()),
                Span::styled("cycle fields", self.theme.style_muted()),
                Span::styled("  │  ", self.theme.style_dim()),
                Span::styled(" Esc ", self.theme.style_accent()),
                Span::styled("step back one level", self.theme.style_muted()),
            ]),
        ];
        Paragraph::new(intro)
            .wrap(Wrap { trim: false })
            .render(chunks[0], buf);

        let form_border = if self.focus == Focus::Tabs {
            self.theme.style_border()
        } else {
            self.theme.style_border_focused()
        };
        let name_style = create_field_style(self, CreateModalFocus::Name);
        let desc_style = create_field_style(self, CreateModalFocus::Description);
        let submit_style = create_field_style(self, CreateModalFocus::Submit);
        let submit_text = if self.create_submitting {
            "Creating..."
        } else {
            self.ui_config.create.submit_label.as_str()
        };

        let mut lines = vec![
            Line::from(Span::styled(
                self.ui_config.create.name_label.as_str(),
                self.theme.style_dim(),
            )),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    display_create_value(self.create_name, "<enter a memory name>"),
                    name_style,
                ),
                next_entry_hint(self, CreateModalFocus::Name),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                self.ui_config.create.description_label.as_str(),
                self.theme.style_dim(),
            )),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    display_create_value(self.create_description, "<enter a short description>"),
                    desc_style,
                ),
                next_entry_hint(self, CreateModalFocus::Description),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(submit_text, submit_style),
                next_entry_hint(self, CreateModalFocus::Submit),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                if self.focus == Focus::Tabs {
                    "Tabs focused. Press Enter or Tab to edit from Name, or Esc for Memories."
                } else {
                    self.ui_config.create.close_hint.as_str()
                },
                self.theme.style_muted(),
            )),
        ];

        if let Some(error) = self.create_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {error}"),
                self.theme.style_error().add_modifier(Modifier::BOLD),
            )));
        }

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(form_border)
                    .title(format!(" {} ", self.ui_config.create.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(chunks[1], buf);
    }

    pub(crate) fn create_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if self.focus != Focus::Form {
            return None;
        }
        let padded = shared::layout::content_area(area, true);
        let tabs_height = if self.tab_specs.is_empty() {
            0
        } else {
            shared::layout::TABS_HEIGHT
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(shared::layout::HEADER_HEIGHT),
                Constraint::Length(tabs_height),
                Constraint::Min(12),
                Constraint::Length(shared::layout::STATUS_HEIGHT),
            ])
            .split(padded);
        let form_area = create_form_inner_area(chunks[2])?;
        let field_y = match self.create_focus {
            CreateModalFocus::Name => form_area.y + 1,
            CreateModalFocus::Description => form_area.y + 4,
            CreateModalFocus::Submit => form_area.y + 7,
        };
        let prompt_x = form_area.x + 2;
        let input_display_width: u16 = match self.create_focus {
            CreateModalFocus::Name => terminal_str_width(self.create_name),
            CreateModalFocus::Description => terminal_str_width(self.create_description),
            CreateModalFocus::Submit => 0,
        };
        let x = if self.create_focus == CreateModalFocus::Submit {
            prompt_x
        } else {
            (prompt_x + input_display_width).min(form_area.x + form_area.width.saturating_sub(1))
        };
        Some((x, field_y))
    }
}

fn create_form_inner_area(body: Rect) -> Option<Rect> {
    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(12)])
        .split(body);
    let form_area = Rect {
        x: form_chunks[1].x + 1,
        y: form_chunks[1].y + 1,
        width: form_chunks[1].width.saturating_sub(2),
        height: form_chunks[1].height.saturating_sub(2),
    };
    if form_area.width == 0 || form_area.height == 0 {
        None
    } else {
        Some(form_area)
    }
}

fn create_field_style(ui: &TuiKitUi<'_>, focus: CreateModalFocus) -> Style {
    if is_pending_create_entry(ui, focus) {
        return ui
            .theme
            .style_accent()
            .add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
    }
    if ui.focus == Focus::Tabs {
        return ui.theme.style_dim();
    }
    if ui.create_focus == focus {
        if focus == CreateModalFocus::Submit {
            ui.theme.style_accent_bold()
        } else {
            ui.theme.style_accent()
        }
    } else {
        ui.theme.style_normal()
    }
}

fn is_pending_create_entry(ui: &TuiKitUi<'_>, focus: CreateModalFocus) -> bool {
    ui.focus == Focus::Tabs && focus == CreateModalFocus::Name
}

fn display_create_value<'a>(value: &'a str, placeholder: &'a str) -> &'a str {
    if value.is_empty() { placeholder } else { value }
}

fn terminal_str_width(s: &str) -> u16 {
    UnicodeWidthStr::width(s).min(u16::MAX as usize) as u16
}

fn next_entry_hint(ui: &TuiKitUi<'_>, focus: CreateModalFocus) -> Span<'static> {
    if is_pending_create_entry(ui, focus) {
        return Span::styled("  ← next input", ui.theme.style_muted());
    }
    Span::raw("")
}

#[cfg(test)]
mod tests {
    use crate::theme::Theme;

    use super::*;

    #[test]
    fn create_cursor_positions_follow_field_order() {
        let area = Rect::new(0, 0, 120, 40);
        let theme = Theme::default();

        let name = TuiKitUi::new(&theme)
            .focus(Focus::Form)
            .create_focus(CreateModalFocus::Name)
            .create_cursor_position_for_area(area)
            .expect("name cursor");
        let description = TuiKitUi::new(&theme)
            .focus(Focus::Form)
            .create_focus(CreateModalFocus::Description)
            .create_cursor_position_for_area(area)
            .expect("description cursor");
        let submit = TuiKitUi::new(&theme)
            .focus(Focus::Form)
            .create_focus(CreateModalFocus::Submit)
            .create_cursor_position_for_area(area)
            .expect("submit cursor");

        assert!(name.1 < description.1);
        assert!(description.1 < submit.1);
    }

    #[test]
    fn tabs_focus_marks_name_as_next_entry_target() {
        let theme = Theme::default();
        let ui = TuiKitUi::new(&theme).focus(Focus::Tabs);

        assert!(is_pending_create_entry(&ui, CreateModalFocus::Name));
        assert!(!is_pending_create_entry(&ui, CreateModalFocus::Submit));
    }
}
