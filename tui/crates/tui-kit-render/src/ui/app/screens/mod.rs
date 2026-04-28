//! Screen-oriented modules for the app body.

pub mod create;
pub mod insert;
pub mod memories;
pub mod placeholder;
pub mod settings;

use ratatui::{buffer::Buffer, layout::Rect, text::Line};
use tui_kit_runtime::CreateSubmitState;
use tui_kit_runtime::kinic_tabs::{TabKind, tab_kind};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::ui::app::TuiKitUi;

struct PlaceholderScreenSpec<'a> {
    title: &'a str,
    lead: &'a str,
    detail: &'a str,
}

fn placeholder_screen_spec(kind: TabKind) -> Option<PlaceholderScreenSpec<'static>> {
    match kind {
        TabKind::PlaceholderMarket => Some(PlaceholderScreenSpec {
            title: "Market",
            lead: "Market tab is reserved for future discovery and purchase flows.",
            detail: "Use Memories to browse and Create to provision a new memory today.",
        }),
        TabKind::PlaceholderSettings => None,
        _ => None,
    }
}

impl<'a> TuiKitUi<'a> {
    pub(crate) fn render_tab_screen(&self, area: Rect, buf: &mut Buffer) -> bool {
        match tab_kind(self.current_tab_id.0.as_str()) {
            TabKind::CreateForm => {
                self.render_create_screen(area, buf);
                true
            }
            TabKind::InsertForm => {
                self.render_insert_screen(area, buf);
                true
            }
            TabKind::PlaceholderSettings => {
                self.render_settings_screen(area, buf);
                true
            }
            kind => {
                let Some(spec) = placeholder_screen_spec(kind) else {
                    return false;
                };
                self.render_placeholder_screen(area, buf, spec.title, spec.lead, spec.detail);
                true
            }
        }
    }
}

fn submit_button_text(
    submit_state: &CreateSubmitState,
    spinner_frame_index: usize,
    idle_label: &str,
    pending_label: &str,
) -> String {
    match submit_state {
        CreateSubmitState::Submitting => {
            format!("{} {}", spinner_frame(spinner_frame_index), pending_label)
        }
        CreateSubmitState::Idle | CreateSubmitState::Error => idle_label.to_string(),
    }
}

fn spinner_frame(frame: usize) -> &'static str {
    const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
    FRAMES[frame % FRAMES.len()]
}

pub(crate) struct FormRows<F> {
    rows: Vec<(F, u16, u16)>,
}

pub(crate) struct VisibleMultilineRows {
    pub rows: Vec<String>,
    pub scroll_row: usize,
}

pub(crate) const MULTILINE_TEXTAREA_HEIGHT: u16 = 5;

impl<F> Default for FormRows<F> {
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

impl<F: Copy + PartialEq> FormRows<F> {
    pub(crate) fn push_unlabeled_row<'a>(
        &mut self,
        lines: &mut Vec<Line<'a>>,
        focus: F,
        row_line: Line<'a>,
        display_value: &str,
    ) {
        let row = u16::try_from(lines.len()).unwrap_or(0);
        let width = UnicodeWidthStr::width(display_value) as u16;
        self.rows.push((focus, row, width));
        lines.push(row_line);
    }

    pub(crate) fn push_labeled_row<'a>(
        &mut self,
        lines: &mut Vec<Line<'a>>,
        label_line: Line<'a>,
        focus: F,
        row_line: Line<'a>,
        display_value: &str,
    ) {
        lines.push(label_line);
        self.push_unlabeled_row(lines, focus, row_line, display_value);
    }

    pub(crate) fn focus_row(&self, focus: F) -> Option<u16> {
        self.rows
            .iter()
            .find(|(field, _, _)| *field == focus)
            .map(|(_, row, _)| *row)
    }

    pub(crate) fn focus_width(&self, focus: F) -> u16 {
        self.rows
            .iter()
            .find(|(field, _, _)| *field == focus)
            .map(|(_, _, width)| *width)
            .unwrap_or(0)
    }
}

pub(crate) fn visible_multiline_rows(
    value: &str,
    placeholder: &str,
    height: u16,
    cursor_row: usize,
    max_width: u16,
) -> VisibleMultilineRows {
    let mut source_rows = if value.is_empty() {
        vec![placeholder.to_string()]
    } else {
        value
            .split('\n')
            .map(|row| row.to_string())
            .collect::<Vec<_>>()
    };
    if source_rows.is_empty() {
        source_rows.push(String::new());
    }
    let height_usize = height as usize;
    let scroll_row = if cursor_row >= height_usize {
        cursor_row + 1 - height_usize
    } else {
        0
    };
    let mut rows = source_rows
        .into_iter()
        .skip(scroll_row)
        .take(height_usize)
        .map(|row| fit_multiline_row(row.as_str(), max_width))
        .collect::<Vec<_>>();
    while rows.len() < height_usize {
        rows.push(String::new());
    }
    VisibleMultilineRows { rows, scroll_row }
}

pub(crate) fn multiline_cursor_x(line: &str, cursor_col: usize, max_width: u16) -> u16 {
    let prefix = line.chars().take(cursor_col).collect::<String>();
    terminal_str_width(fit_multiline_row(prefix.as_str(), max_width).as_str())
}

pub(crate) fn multiline_visible_row(cursor_row: usize, scroll_row: usize, height: u16) -> u16 {
    cursor_row
        .saturating_sub(scroll_row)
        .min(height.saturating_sub(1) as usize) as u16
}

fn fit_multiline_row(value: &str, max_width: u16) -> String {
    if max_width == 0 {
        return String::new();
    }
    if terminal_str_width(value) <= max_width {
        return value.to_string();
    }
    take_prefix_by_width(value, max_width)
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

fn char_width(ch: char) -> u16 {
    UnicodeWidthChar::width(ch)
        .unwrap_or(0)
        .min(u16::MAX as usize) as u16
}

fn terminal_str_width(s: &str) -> u16 {
    UnicodeWidthStr::width(s).min(u16::MAX as usize) as u16
}
