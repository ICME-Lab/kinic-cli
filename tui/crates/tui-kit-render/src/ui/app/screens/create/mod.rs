//! Create screen rendering and cursor geometry.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use tui_kit_runtime::{CreateCostDetails, CreateCostState, CreateModalFocus, CreateSubmitState};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::ui::app::{Focus, TuiKitUi, shared, types::CreateOverlayText};

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_create_screen(&self, area: Rect, buf: &mut Buffer) {
        let screen = CreateScreenState::from_root_area(self, area);
        let intro = vec![
            Line::from(vec![
                Span::styled(" Create ", self.theme.style_accent_bold()),
                Span::styled(
                    self.ui_config.create.intro_description.as_str(),
                    self.theme.style_normal(),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Enter ", self.theme.style_accent()),
                Span::styled(
                    self.ui_config.create.intro_enter_hint.as_str(),
                    self.theme.style_muted(),
                ),
                Span::styled("  │  ", self.theme.style_dim()),
                Span::styled(" Tab / Shift+Tab ", self.theme.style_accent()),
                Span::styled(
                    self.ui_config.create.intro_cycle_hint.as_str(),
                    self.theme.style_muted(),
                ),
                Span::styled("  │  ", self.theme.style_dim()),
                Span::styled(" Esc ", self.theme.style_accent()),
                Span::styled(
                    self.ui_config.create.intro_escape_hint.as_str(),
                    self.theme.style_muted(),
                ),
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
            .render(screen.layout.form_area, buf);
    }

    pub(crate) fn create_cursor_position_for_area(&self, area: Rect) -> Option<(u16, u16)> {
        if self.focus != Focus::Form {
            return None;
        }
        let screen = CreateScreenState::from_root_area(self, area);
        let field_y = screen
            .layout
            .field_y(self.create_focus, &screen.form_lines)?;
        let prompt_x = screen.layout.field_x();
        let input_display_width = screen.form_lines.focus_display_width(self.create_focus);
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
    fn from_root_area(ui: &'a TuiKitUi<'a>, area: Rect) -> Self {
        let layout = CreateScreenLayout::from_root_area(area, !ui.tab_specs.is_empty());
        let form_lines = create_form_lines(ui, layout);
        Self { layout, form_lines }
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

    fn from_root_area(area: Rect, has_tabs: bool) -> Self {
        let body = shared::layout::body_rect_for_area_with_tabs(area, has_tabs);
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

    fn field_x(self) -> u16 {
        self.form_inner_area
            .map_or(0, |area| area.x + Self::INPUT_INDENT_WIDTH)
    }

    fn field_y(self, focus: CreateModalFocus, form_lines: &CreateFormLines) -> Option<u16> {
        let base_y = self.form_inner_area?.y;
        let row_index = form_lines.focus_row_index(focus)?;
        Some(base_y + row_index)
    }

    fn input_width(self, hint_width: u16) -> u16 {
        self.form_inner_area.map_or(0, |area| {
            area.width
                .saturating_sub(Self::INPUT_INDENT_WIDTH)
                .saturating_sub(hint_width)
        })
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
    name_display_width: u16,
    description_display_width: u16,
}
impl CreateFormLines<'_> {
    fn focus_row_index(&self, focus: CreateModalFocus) -> Option<u16> {
        match focus {
            CreateModalFocus::Name => Some(self.name_row),
            CreateModalFocus::Description => Some(self.description_row),
            CreateModalFocus::Submit => Some(self.submit_row),
        }
    }

    fn focus_display_width(&self, focus: CreateModalFocus) -> u16 {
        match focus {
            CreateModalFocus::Name => self.name_display_width,
            CreateModalFocus::Description => self.description_display_width,
            CreateModalFocus::Submit => 0,
        }
    }
}
fn create_form_lines<'a>(ui: &'a TuiKitUi<'a>, layout: CreateScreenLayout) -> CreateFormLines<'a> {
    let name_style = create_field_style(ui, CreateModalFocus::Name);
    let description_style = create_field_style(ui, CreateModalFocus::Description);
    let submit_style = create_field_style(ui, CreateModalFocus::Submit);
    let close_hint = create_close_hint(ui);
    let submit_text = create_submit_text(ui);
    let name_hint = next_entry_hint(ui, CreateModalFocus::Name);
    let description_hint = next_entry_hint(ui, CreateModalFocus::Description);
    let submit_hint = next_entry_hint(ui, CreateModalFocus::Submit);
    let name_value = fit_single_line(
        display_create_value(ui.create_name, "<enter a memory name>"),
        layout.input_width(terminal_str_width(name_hint.content.as_ref())),
        !ui.create_name.is_empty(),
    );
    let description_value = fit_single_line(
        display_create_value(ui.create_description, "<enter a short description>"),
        layout.input_width(terminal_str_width(description_hint.content.as_ref())),
        !ui.create_description.is_empty(),
    );
    let submit_value = fit_single_line(
        submit_text.as_str(),
        layout.input_width(terminal_str_width(submit_hint.content.as_ref())),
        false,
    );
    let mut lines = Vec::with_capacity(if ui.create_error.is_some() { 20 } else { 18 });

    lines.push(Line::from(Span::styled(
        ui.ui_config.create.name_label.as_str(),
        ui.theme.style_dim(),
    )));
    let name_row = u16::try_from(lines.len()).expect("name row fits into u16");
    lines.push(Line::from(vec![
        create_input_indent(),
        Span::styled(name_value.clone(), name_style),
        name_hint,
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        ui.ui_config.create.description_label.as_str(),
        ui.theme.style_dim(),
    )));
    let description_row = u16::try_from(lines.len()).expect("description row fits into u16");
    lines.push(Line::from(vec![
        create_input_indent(),
        Span::styled(description_value.clone(), description_style),
        description_hint,
    ]));
    lines.push(Line::from(""));
    let submit_row = u16::try_from(lines.len()).expect("submit row fits into u16");
    lines.push(Line::from(vec![
        create_input_indent(),
        Span::styled(submit_value, submit_style),
        submit_hint,
    ]));
    lines.push(Line::from(""));
    lines.extend(create_cost_lines(ui, layout));
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
        name_display_width: terminal_str_width(name_value.as_str()),
        description_display_width: terminal_str_width(description_value.as_str()),
    }
}
fn create_cost_lines(ui: &TuiKitUi<'_>, layout: CreateScreenLayout) -> Vec<Line<'static>> {
    if matches!(ui.create_cost_state, CreateCostState::Hidden) {
        return Vec::new();
    }

    let create_cfg = &ui.ui_config.create;
    let mut lines = vec![Line::from(Span::styled(
        create_cfg.account_title.clone(),
        ui.theme.style_dim(),
    ))];
    let detail_label_width = create_cost_label_width(create_cfg);

    match ui.create_cost_state {
        CreateCostState::Hidden => {}
        CreateCostState::Loading => {
            lines.push(Line::from(Span::styled(
                create_cfg.loading_message.clone(),
                ui.theme.style_muted(),
            )));
        }
        CreateCostState::Unavailable => {
            lines.push(Line::from(Span::styled(
                create_cfg.unavailable_message.clone(),
                ui.theme.style_muted(),
            )));
        }
        CreateCostState::Error(errors) => {
            for error in errors {
                lines.push(Line::from(Span::styled(
                    format!("{}: {error}", create_cfg.error_prefix),
                    ui.theme.style_error(),
                )));
            }
        }
        CreateCostState::Ready { details, issues } => {
            lines.push(create_cost_detail_line(
                ui,
                layout,
                detail_label_width,
                create_cfg.principal_label.as_str(),
                details.principal.clone(),
            ));
            lines.push(create_cost_detail_line(
                ui,
                layout,
                detail_label_width,
                create_cfg.balance_label.as_str(),
                format_kinic_value(details.balance_kinic.as_str()),
            ));
            lines.push(create_cost_detail_line(
                ui,
                layout,
                detail_label_width,
                create_cfg.price_label.as_str(),
                format_kinic_value(details.required_total_kinic.as_str()),
            ));
            lines.push(create_cost_detail_line(
                ui,
                layout,
                detail_label_width,
                create_cfg.status_label.as_str(),
                create_status_text(create_cfg, details),
            ));
            for issue in issues {
                lines.push(Line::from(Span::styled(
                    format!("{}: {issue}", create_cfg.error_prefix),
                    ui.theme.style_error(),
                )));
            }
        }
    }

    lines
}
fn create_cost_detail_line(
    ui: &TuiKitUi<'_>,
    layout: CreateScreenLayout,
    label_width: u16,
    label: &str,
    value: String,
) -> Line<'static> {
    let padded_label = pad_terminal_width(label, label_width as usize);
    let fitted_value = fit_single_line(value.as_str(), layout.input_width(label_width + 2), true);
    Line::from(vec![
        create_input_indent(),
        Span::styled(format!("{padded_label}: "), ui.theme.style_dim()),
        Span::styled(fitted_value, ui.theme.style_normal()),
    ])
}

fn create_cost_label_width(create_cfg: &CreateOverlayText) -> u16 {
    [
        create_cfg.principal_label.as_str(),
        create_cfg.balance_label.as_str(),
        create_cfg.price_label.as_str(),
        create_cfg.status_label.as_str(),
    ]
    .into_iter()
    .map(terminal_str_width)
    .max()
    .unwrap_or(0)
}

fn format_kinic_value(kinic: &str) -> String {
    format!("{} KINIC", round_kinic_display(kinic))
}

fn round_kinic_display(value: &str) -> String {
    let mut chars = value.chars();
    let sign = if matches!(chars.clone().next(), Some('+') | Some('-')) {
        chars.next().unwrap_or_default().to_string()
    } else {
        String::new()
    };
    let unsigned = chars.as_str();
    let Some((whole, frac)) = unsigned.split_once('.') else {
        return format!("{sign}{unsigned}.000");
    };
    let mut frac_digits = frac.chars();
    let first = frac_digits.next().unwrap_or('0');
    let second = frac_digits.next().unwrap_or('0');
    let third = frac_digits.next().unwrap_or('0');
    format!("{sign}{whole}.{first}{second}{third}")
}

fn pad_terminal_width(value: &str, width: usize) -> String {
    let current_width = terminal_str_width(value) as usize;
    if current_width >= width {
        return value.to_string();
    }
    format!("{value}{}", " ".repeat(width - current_width))
}
fn create_status_text(create_cfg: &CreateOverlayText, details: &CreateCostDetails) -> String {
    if details.sufficient_balance {
        create_cfg.status_ready_label.clone()
    } else {
        create_cfg.status_insufficient_label.clone()
    }
}
fn create_form_border_style(ui: &TuiKitUi<'_>) -> Style {
    if ui.focus == Focus::Form {
        ui.theme.style_border_focused()
    } else {
        ui.theme.style_border()
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
    ui.focus == Focus::Tabs && focus == first_create_focus()
}
fn display_create_value<'a>(value: &'a str, placeholder: &'a str) -> &'a str {
    if value.is_empty() { placeholder } else { value }
}
fn create_submit_text(ui: &TuiKitUi<'_>) -> String {
    match ui.create_submit_state {
        CreateSubmitState::Submitting => format!(
            "{} {}",
            spinner_frame(ui.create_spinner_frame),
            ui.ui_config.create.submit_pending_label
        ),
        CreateSubmitState::Idle | CreateSubmitState::Error => {
            ui.ui_config.create.submit_label.clone()
        }
    }
}
fn create_close_hint<'a>(ui: &'a TuiKitUi<'a>) -> &'a str {
    if ui.focus == Focus::Tabs {
        ui.ui_config.create.tabs_focus_hint.as_str()
    } else {
        ui.ui_config.create.close_hint.as_str()
    }
}
fn fit_single_line(value: &str, max_width: u16, keep_end: bool) -> String {
    if max_width == 0 {
        return String::new();
    }
    if terminal_str_width(value) <= max_width {
        return value.to_string();
    }
    if keep_end {
        take_suffix_by_width(value, max_width)
    } else {
        take_prefix_by_width(value, max_width)
    }
}
fn first_create_focus() -> CreateModalFocus {
    CreateModalFocus::Name
}
fn take_prefix_by_width(value: &str, max_width: u16) -> String {
    let mut width = 0;
    let mut out = String::new();
    for ch in value.chars() {
        let ch_width = char_width(ch);
        if width + ch_width > max_width {
            break;
        }
        width += ch_width;
        out.push(ch);
    }
    out
}
fn take_suffix_by_width(value: &str, max_width: u16) -> String {
    let mut width = 0;
    let mut rev_chars = Vec::new();
    for ch in value.chars().rev() {
        let ch_width = char_width(ch);
        if width + ch_width > max_width {
            break;
        }
        width += ch_width;
        rev_chars.push(ch);
    }
    rev_chars.into_iter().rev().collect()
}
fn char_width(ch: char) -> u16 {
    UnicodeWidthChar::width(ch)
        .unwrap_or(0)
        .min(u16::MAX as usize) as u16
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
fn spinner_frame(frame: usize) -> &'static str {
    const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
    FRAMES[frame % FRAMES.len()]
}
#[cfg(test)]
mod tests;
