use crate::app::interaction::UiEffect;
use crate::app::{App, Tab};
use crate::ui::Focus;

pub fn apply_escape_domain_fallback(app: &mut App) -> bool {
    if app.current_tab == Tab::Crates && app.selected_installed_crate.is_some() {
        app.clear_installed_crate();
        return true;
    }
    if !app.search_input.is_empty() {
        app.clear_search();
        return true;
    }
    false
}

pub fn apply_crates_qualified_enter(app: &mut App) {
    if app.current_tab == Tab::Crates && app.selected_installed_crate.is_some() {
        app.search_qualified_path();
        app.filter_items();
        app.focus = Focus::List;
    }
}

pub fn apply_list_enter(app: &mut App) {
    if app.current_tab == Tab::Crates && app.selected_installed_crate.is_none() {
        if let Some(name) = app.selected_dependency_name() {
            if app.dependency_root_name() != Some(name.as_str()) {
                let _ = app.select_installed_crate(&name);
                app.list_state.select(Some(0));
                app.refresh_ui_cache();
            } else {
                app.focus = Focus::Inspector;
            }
        } else {
            app.focus = Focus::Inspector;
        }
    } else {
        app.focus = Focus::Inspector;
    }
}

pub fn apply_list_open_docs(app: &App, effects: &mut Vec<UiEffect>) {
    if app.current_tab == Tab::Crates {
        if let Some(name) = app.selected_crate_name_for_display() {
            let url = format!("https://docs.rs/{}", name);
            effects.push(UiEffect::OpenUrl {
                url: url.clone(),
                success_message: Some(format!("Opened {} in browser", name)),
                failure_message: Some(format!("Failed to open {}", url)),
            });
        }
    }
}

pub fn apply_inspector_open_docs(app: &App, effects: &mut Vec<UiEffect>) {
    apply_list_open_docs(app, effects);
}

pub fn apply_list_open_crates_io(app: &App, effects: &mut Vec<UiEffect>) {
    if app.current_tab == Tab::Crates {
        if let Some(name) = app.selected_crate_name_for_display() {
            let url = format!("https://crates.io/crates/{}", name);
            effects.push(UiEffect::OpenUrl {
                url: url.clone(),
                success_message: Some(format!("Opened {} in browser", name)),
                failure_message: Some(format!("Failed to open {}", url)),
            });
        }
    }
}

pub fn apply_inspector_open_crates_io(app: &App, effects: &mut Vec<UiEffect>) {
    apply_list_open_crates_io(app, effects);
}

pub fn apply_list_left(app: &mut App) {
    if app.current_tab == Tab::Crates && app.selected_installed_crate.is_some() {
        app.clear_installed_crate();
    } else {
        app.focus = Focus::Search;
    }
}
