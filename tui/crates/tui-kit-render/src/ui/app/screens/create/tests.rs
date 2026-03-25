use ratatui::layout::Rect;
use tui_kit_runtime::CreateModalFocus;

use crate::{
    theme::Theme,
    ui::app::{Focus, TuiKitUi},
};

use super::{CreateScreenLayout, create_form_lines, is_pending_create_entry};

fn cursor_y(ui: TuiKitUi<'_>, area: Rect) -> u16 {
    ui.create_cursor_position_for_area(area)
        .expect("cursor available")
        .1
}

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
fn create_cursor_positions_match_form_row_definitions() {
    let area = Rect::new(0, 0, 120, 40);
    let theme = Theme::default();
    let layout = CreateScreenLayout::from_root_area(area, true).expect("layout");
    let base_ui = TuiKitUi::new(&theme).focus(Focus::Form);
    let form_lines = create_form_lines(&base_ui);
    let base_y = layout.form_inner_area.expect("inner").y;

    assert_eq!(
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_focus(CreateModalFocus::Name),
            area
        ),
        base_y
            + form_lines
                .focus_row_index(CreateModalFocus::Name)
                .expect("name row")
    );
    assert_eq!(
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_focus(CreateModalFocus::Description),
            area
        ),
        base_y
            + form_lines
                .focus_row_index(CreateModalFocus::Description)
                .expect("description row")
    );
    assert_eq!(
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_focus(CreateModalFocus::Submit),
            area
        ),
        base_y
            + form_lines
                .focus_row_index(CreateModalFocus::Submit)
                .expect("submit row")
    );
}

#[test]
fn create_cursor_positions_are_stable_when_error_is_visible() {
    let area = Rect::new(0, 0, 120, 40);
    let theme = Theme::default();

    assert_eq!(
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_error(Some("boom"))
                .create_focus(CreateModalFocus::Name),
            area
        ),
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_focus(CreateModalFocus::Name),
            area
        )
    );
    assert_eq!(
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_error(Some("boom"))
                .create_focus(CreateModalFocus::Description),
            area
        ),
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_focus(CreateModalFocus::Description),
            area
        )
    );
    assert_eq!(
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_error(Some("boom"))
                .create_focus(CreateModalFocus::Submit),
            area
        ),
        cursor_y(
            TuiKitUi::new(&theme)
                .focus(Focus::Form)
                .create_focus(CreateModalFocus::Submit),
            area
        )
    );
}

#[test]
fn create_cursor_positions_work_without_tabs() {
    let area = Rect::new(0, 0, 120, 40);
    let theme = Theme::default();
    let ui = TuiKitUi::new(&theme)
        .focus(Focus::Form)
        .tab_specs(Vec::new())
        .create_focus(CreateModalFocus::Description);

    assert!(ui.create_cursor_position_for_area(area).is_some());
}

#[test]
fn tabs_focus_marks_name_as_next_entry_target() {
    let theme = Theme::default();
    let ui = TuiKitUi::new(&theme).focus(Focus::Tabs);

    assert!(is_pending_create_entry(&ui, CreateModalFocus::Name));
    assert!(!is_pending_create_entry(&ui, CreateModalFocus::Submit));
}
