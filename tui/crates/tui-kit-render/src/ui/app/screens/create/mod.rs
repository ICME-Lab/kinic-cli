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
        let screen = CreateScreenState::from_body_area(self, area);
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
            .render(screen.layout.intro_area, buf);

        Paragraph::new(screen.form_lines.lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(create_form_border_style(self))
                    .title(format!(" {} ", self.ui_config.create.title))
                    .style(Style::default().bg(self.theme.bg_panel)),
            )
            .wrap(Wrap { trim: false })
            .render(screen.layout.form_area, buf);
    }

    pub(crate) fn create_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if self.focus != Focus::Form {
            return None;
        }
        let has_tabs = !self.tab_specs.is_empty();
        let screen = CreateScreenState::from_root_area(self, area, has_tabs)?;
        let field_y = screen
            .layout
            .field_y(self.create_focus, &screen.form_lines)?;
        let prompt_x = screen.layout.field_x(self.create_focus);
        let input_display_width: u16 = match self.create_focus {
            CreateModalFocus::Name => terminal_str_width(self.create_name),
            CreateModalFocus::Description => terminal_str_width(self.create_description),
            CreateModalFocus::Submit => 0,
        };
        let x = if self.create_focus == CreateModalFocus::Submit {
            prompt_x
        } else {
            (prompt_x + input_display_width).min(screen.layout.max_field_x())
        };
        Some((x, field_y))
    }
}
struct CreateScreenState<'a> {
    layout: CreateScreenLayout,
    form_lines: CreateFormLines<'a>,
}
impl<'a> CreateScreenState<'a> {
    fn from_body_area(ui: &'a TuiKitUi<'a>, area: Rect) -> Self {
        Self {
            layout: CreateScreenLayout::from_body_area(area),
            form_lines: create_form_lines(ui),
        }
    }

    fn from_root_area(ui: &'a TuiKitUi<'a>, area: Rect, has_tabs: bool) -> Option<Self> {
        Some(Self {
            layout: CreateScreenLayout::from_root_area(area, has_tabs)?,
            form_lines: create_form_lines(ui),
        })
    }
}
#[derive(Clone, Copy)]
struct CreateScreenLayout {
    intro_area: Rect,
    form_area: Rect,
    form_inner_area: Option<Rect>,
}
impl CreateScreenLayout {
    const INTRO_HEIGHT: u16 = 6;
    const INPUT_INDENT_WIDTH: u16 = 2;

    fn from_root_area(area: Rect, has_tabs: bool) -> Option<Self> {
        let padded = shared::layout::content_area(area, true);
        let tabs_height = if has_tabs {
            shared::layout::TABS_HEIGHT
        } else {
            0
        };
        let root_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(shared::layout::HEADER_HEIGHT),
                Constraint::Length(tabs_height),
                Constraint::Min(12),
                Constraint::Length(shared::layout::STATUS_HEIGHT),
            ])
            .split(padded);
        Some(Self::from_body_area(root_chunks[2]))
    }

    fn from_body_area(body: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(Self::INTRO_HEIGHT), Constraint::Min(12)])
            .split(body);
        let form_inner_area = inner_bordered_area(chunks[1]);
        Self {
            intro_area: chunks[0],
            form_area: chunks[1],
            form_inner_area,
        }
    }

    fn field_x(self, focus: CreateModalFocus) -> u16 {
        self.form_inner_area
            .map_or(0, |area| area.x + create_field_x_offset(focus))
    }

    fn field_y(self, focus: CreateModalFocus, form_lines: &CreateFormLines) -> Option<u16> {
        let base_y = self.form_inner_area?.y;
        let row_index = form_lines.focus_row_index(focus)?;
        Some(base_y + row_index)
    }

    fn max_field_x(self) -> u16 {
        self.form_inner_area
            .map_or(0, |area| area.x + area.width.saturating_sub(1))
    }
}
fn inner_bordered_area(area: Rect) -> Option<Rect> {
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    (inner.width > 0 && inner.height > 0).then_some(inner)
}
struct CreateFormLines<'a> {
    lines: Vec<Line<'a>>,
    name_row: u16,
    description_row: u16,
    submit_row: u16,
}
impl CreateFormLines<'_> {
    fn focus_row_index(&self, focus: CreateModalFocus) -> Option<u16> {
        match self {
            _ if focus == CreateModalFocus::Name => Some(self.name_row),
            _ if focus == CreateModalFocus::Description => Some(self.description_row),
            _ if focus == CreateModalFocus::Submit => Some(self.submit_row),
            _ => None,
        }
    }
}
fn create_form_lines<'a>(ui: &'a TuiKitUi<'a>) -> CreateFormLines<'a> {
    let name_style = create_field_style(ui, CreateModalFocus::Name);
    let description_style = create_field_style(ui, CreateModalFocus::Description);
    let submit_style = create_field_style(ui, CreateModalFocus::Submit);
    let close_hint = create_close_hint(ui);
    let submit_text = create_submit_text(ui);
    let mut lines = Vec::with_capacity(if ui.create_error.is_some() { 11 } else { 9 });

    lines.push(Line::from(Span::styled(
        ui.ui_config.create.name_label.as_str(),
        ui.theme.style_dim(),
    )));
    let name_row = u16::try_from(lines.len()).expect("name row fits into u16");
    lines.push(Line::from(vec![
        create_input_indent(),
        Span::styled(
            display_create_value(ui.create_name, "<enter a memory name>"),
            name_style,
        ),
        next_entry_hint(ui, CreateModalFocus::Name),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        ui.ui_config.create.description_label.as_str(),
        ui.theme.style_dim(),
    )));
    let description_row = u16::try_from(lines.len()).expect("description row fits into u16");
    lines.push(Line::from(vec![
        create_input_indent(),
        Span::styled(
            display_create_value(ui.create_description, "<enter a short description>"),
            description_style,
        ),
        next_entry_hint(ui, CreateModalFocus::Description),
    ]));
    lines.push(Line::from(""));
    let submit_row = u16::try_from(lines.len()).expect("submit row fits into u16");
    lines.push(Line::from(vec![
        create_input_indent(),
        Span::styled(submit_text, submit_style),
        next_entry_hint(ui, CreateModalFocus::Submit),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(close_hint, ui.theme.style_muted())));

    if let Some(error) = ui.create_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {error}"),
            ui.theme.style_error().add_modifier(Modifier::BOLD),
        )));
    }

    CreateFormLines {
        lines,
        name_row,
        description_row,
        submit_row,
    }
}
fn create_form_border_style(ui: &TuiKitUi<'_>) -> Style {
    if ui.focus == Focus::Tabs {
        ui.theme.style_border()
    } else {
        ui.theme.style_border_focused()
    }
}
fn create_field_x_offset(focus: CreateModalFocus) -> u16 {
    match focus {
        CreateModalFocus::Name | CreateModalFocus::Description | CreateModalFocus::Submit => {
            CreateScreenLayout::INPUT_INDENT_WIDTH
        }
    }
}
fn create_input_indent() -> Span<'static> {
    Span::raw(" ".repeat(CreateScreenLayout::INPUT_INDENT_WIDTH as usize))
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
fn create_submit_text<'a>(ui: &'a TuiKitUi<'a>) -> &'a str {
    if ui.create_submitting {
        "Creating..."
    } else {
        ui.ui_config.create.submit_label.as_str()
    }
}
fn create_close_hint<'a>(ui: &'a TuiKitUi<'a>) -> &'a str {
    if ui.focus == Focus::Tabs {
        "Tabs focused. Press Enter or Tab to edit from Name, or Esc for Memories."
    } else {
        ui.ui_config.create.close_hint.as_str()
    }
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
mod tests;
