use ratatui::layout::Rect;
use tui_kit_runtime::CreateModalFocus;

use crate::{
    theme::Theme,
    ui::app::{Focus, TuiKitUi, UiConfig},
};

use super::{
    CreateScreenLayout, create_form_border_style, create_form_lines, create_submit_text,
    fit_single_line, is_pending_create_entry,
};

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
    let layout = CreateScreenLayout::from_root_area(area, true);
    let base_ui = TuiKitUi::new(&theme).focus(Focus::Form);
    let form_lines = create_form_lines(&base_ui, layout);
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
fn create_cursor_uses_visible_suffix_for_long_name() {
    let area = Rect::new(0, 0, 32, 20);
    let theme = Theme::default();
    let long_name = "0123456789abcdefghijklmnopqrstuvwxyz";
    let ui = TuiKitUi::new(&theme)
        .focus(Focus::Form)
        .create_name(long_name)
        .create_focus(CreateModalFocus::Name);
    let screen = super::CreateScreenState::from_root_area(&ui, area);
    let cursor = ui
        .create_cursor_position_for_area(area)
        .expect("name cursor");

    assert_eq!(
        cursor.0,
        (screen.layout.field_x() + screen.form_lines.name_display_width)
            .min(screen.layout.max_field_x())
    );
    assert!(fit_single_line(long_name, screen.layout.input_width(0), true) != long_name);
}

#[test]
fn create_cursor_uses_visible_suffix_for_long_description() {
    let area = Rect::new(0, 0, 30, 20);
    let theme = Theme::default();
    let long_description = "description text that is intentionally too wide";
    let ui = TuiKitUi::new(&theme)
        .focus(Focus::Form)
        .create_description(long_description)
        .create_focus(CreateModalFocus::Description);
    let screen = super::CreateScreenState::from_root_area(&ui, area);
    let cursor = ui
        .create_cursor_position_for_area(area)
        .expect("description cursor");

    assert_eq!(
        cursor.0,
        (screen.layout.field_x() + screen.form_lines.description_display_width)
            .min(screen.layout.max_field_x())
    );
    assert!(fit_single_line(long_description, screen.layout.input_width(0), true) != long_description);
}

#[test]
fn tabs_focus_marks_name_as_next_entry_target() {
    let theme = Theme::default();
    let ui = TuiKitUi::new(&theme).focus(Focus::Tabs);

    assert!(is_pending_create_entry(&ui, CreateModalFocus::Name));
    assert!(!is_pending_create_entry(&ui, CreateModalFocus::Submit));
}

#[test]
fn create_form_border_is_focused_only_for_form_focus() {
    let theme = Theme::default();

    assert_eq!(
        create_form_border_style(&TuiKitUi::new(&theme).focus(Focus::Form)),
        theme.style_border_focused()
    );
    assert_eq!(
        create_form_border_style(&TuiKitUi::new(&theme).focus(Focus::Tabs)),
        theme.style_border()
    );
    assert_eq!(
        create_form_border_style(&TuiKitUi::new(&theme).focus(Focus::Inspector)),
        theme.style_border()
    );
}

#[test]
fn create_screen_strings_come_from_ui_config() {
    let theme = Theme::default();
    let mut config = UiConfig::default();
    config.create.intro_description = "Custom intro".to_string();
    config.create.tabs_focus_hint = "Custom tabs hint".to_string();
    config.create.submit_pending_label = "Provisioning".to_string();

    let tabs_ui = TuiKitUi::new(&theme).focus(Focus::Tabs).ui_config(config.clone());
    let form_ui = TuiKitUi::new(&theme)
        .focus(Focus::Form)
        .create_submitting(true)
        .ui_config(config);
    let layout = CreateScreenLayout::from_root_area(Rect::new(0, 0, 80, 24), true);
    let lines = create_form_lines(&tabs_ui, layout);

    assert_eq!(tabs_ui.ui_config.create.intro_description, "Custom intro");
    assert_eq!(
        lines.lines[8].spans[0].content.as_ref(),
        "Custom tabs hint"
    );
    assert_eq!(create_submit_text(&form_ui), "Provisioning");
}
