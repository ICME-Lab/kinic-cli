//! Layout constants and helpers (frame chunks, tabs rect for hit testing).

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Layout constants for the main frame.
pub const HEADER_HEIGHT: u16 = 6;
pub const TABS_HEIGHT: u16 = 3;
pub const STATUS_HEIGHT: u16 = 3;
pub const BODY_MARGIN: u16 = 1;

/// Returns the inner padded area after the outer rounded block.
pub fn content_area(area: Rect, border: bool) -> Rect {
    let inner = if border {
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    } else {
        area
    };
    Rect {
        x: inner.x + BODY_MARGIN,
        y: inner.y + BODY_MARGIN,
        width: inner.width.saturating_sub(2 * BODY_MARGIN),
        // Keep top breathing room, but let footer/status sit flush to the bottom.
        height: inner.height.saturating_sub(BODY_MARGIN),
    }
}

/// Returns the tabs bar Rect for a given full frame area (for mouse hit testing).
pub fn tabs_rect_for_area(area: Rect) -> Option<Rect> {
    tabs_rect_for_area_with_tabs(area, true)
}

pub fn tabs_rect_for_area_with_tabs(area: Rect, has_tabs: bool) -> Option<Rect> {
    if !has_tabs {
        return None;
    }
    let content = content_area(area, true);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Length(TABS_HEIGHT),
            Constraint::Min(12),
            Constraint::Length(STATUS_HEIGHT),
        ])
        .split(content);
    Some(chunks[1])
}

/// Returns the list viewport height (content rows, excluding block borders) for a full frame area.
pub fn list_viewport_height_for_area(area: Rect) -> usize {
    list_viewport_height_for_area_with_tabs(area, true)
}

pub fn list_viewport_height_for_area_with_tabs(area: Rect, has_tabs: bool) -> usize {
    let content = content_area(area, true);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Length(if has_tabs { TABS_HEIGHT } else { 0 }),
            Constraint::Min(12),
            Constraint::Length(STATUS_HEIGHT),
        ])
        .split(content);
    let body = chunks[2];
    let left_div_right = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Length(1),
            Constraint::Ratio(2, 3),
        ])
        .split(body);
    let left_column = left_div_right[0];
    let left_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6)])
        .split(left_column);
    let list_rect = left_split[1];
    list_rect.height.saturating_sub(2) as usize
}
