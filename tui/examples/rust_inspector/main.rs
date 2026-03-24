#![allow(dead_code, unused_imports)]

//! Oracle example app built on top of tui-kit crates.

mod adapter;
mod app;
mod config;
mod domain_rust;
mod error;
mod provider;
mod utils;

pub use tui_kit_render::ui;

use anyhow::Result;
use app::{
    App, UiEffect, apply_intent, apply_runtime_set_tab, build_render_context, intents_for_key,
    try_apply_runtime_action,
};
use crossterm::execute;
use ratatui::layout::Rect;
use std::{env, io, path::PathBuf, time::Duration};
use tui_kit_host::terminal::{HostTerminal, with_terminal};
use tui_kit_host::{HostInputEvent, poll_host_input};
use ui::{
    AnimationState, TuiKitUi,
    app::{list_viewport_height_for_area, tabs_rect_for_area},
};

fn main() -> Result<()> {
    // Load .env so GITHUB_TOKEN etc. are available (cwd first, then project path overrides)
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().collect();
    let mut project_path = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or(PathBuf::from(".")));
    // Resolve to absolute path so we always analyze the directory the user expects
    if project_path.exists() {
        if let Ok(canon) = std::fs::canonicalize(&project_path) {
            project_path = canon;
        }
    }
    let _ = dotenvy::from_path(project_path.join(".env"));

    // Create and run app
    let mut app = App::new();

    // Try to load settings (ignore errors, use defaults)
    let _ = app.load_settings();

    // Analyze the project
    if let Err(e) = app.analyze_project(project_path.as_path()) {
        app.status_message = format!("Analysis failed: {}", e);
    }
    app.refresh_ui_cache();

    if let Err(err) = with_terminal(|terminal| run_app(terminal, &mut app).map_err(|e| e.into())) {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut HostTerminal, app: &mut App) -> Result<()> {
    let mut animation = AnimationState::new();
    let mut inspector_scroll: usize = 0;
    let mut last_selected: Option<usize> = None;
    let mut list_scroll_offset: usize = 0;

    loop {
        // Update animations
        animation.update();

        if let Ok(size) = terminal.size() {
            let area = Rect::new(0, 0, size.width, size.height);
            let visible_height = list_viewport_height_for_area(area);
            list_scroll_offset = keep_selection_visible_scroll(
                list_scroll_offset,
                app.list_state.selected(),
                visible_height,
                app.ui_summaries.len(),
            );
        }

        // Reset inspector scroll on selection change
        let current_selected = app.list_state.selected();
        if current_selected != last_selected {
            inspector_scroll = 0;
            animation.on_selection_change();
            last_selected = current_selected;
        }

        // Poll Chat chat response (from background thread)
        if let Ok(response) = app.chat_rx.try_recv() {
            app.chat_messages.push(("assistant".to_string(), response));
            app.chat_loading = false;
        }

        // Poll crate docs channel and maybe start fetch for selected dependency
        app.poll_crate_docs_rx();
        app.maybe_start_crate_doc_fetch();

        // Draw UI
        let render_context = build_render_context(app);

        terminal.draw(|frame| {
            let selected = app.list_state.selected();
            let ui = TuiKitUi::new(&app.theme)
                .ui_summaries(&app.ui_summaries)
                .list_selected(selected)
                .list_scroll(list_scroll_offset)
                .ui_total_count(app.ui_total_count)
                .in_context_items_view(render_context.in_context_items_view)
                .candidates(&app.filtered_candidates)
                .context_tree(render_context.context_tree)
                .filtered_context_indices(render_context.filtered_context_indices)
                .context_details_loading(render_context.context_details_loading)
                .context_details_failed(render_context.context_details_failed)
                .ui_selected_detail(app.ui_selected_detail.as_ref())
                .ui_context_node(render_context.ui_context_node)
                .target_size_bytes(app.target_size_bytes)
                .search_input(&app.search_input)
                .current_tab_id(app.current_tab.id())
                .show_context_panel(render_context.show_context_panel)
                .ui_config(app.ui_config.clone())
                .focus(app.focus)
                .completion_selected(app.completion_selected)
                .show_completion(app.show_completion)
                .show_help(app.show_help)
                .show_settings(app.show_settings)
                .show_create_modal(app.create_modal_open)
                .create_name(&app.create_name)
                .create_description(&app.create_description)
                .create_submitting(app.create_submitting)
                .create_error(app.create_error.as_deref())
                .create_focus(app.create_focus)
                .status_message(&app.status_message)
                .inspector_scroll(inspector_scroll)
                .animation_state(&animation)
                .show_chat(app.chat_open)
                .chat_messages(&app.chat_messages)
                .chat_input(&app.chat_input)
                .chat_loading(app.chat_loading)
                .chat_scroll(app.chat_scroll);

            let area = frame.area();
            ui.render_in_frame(frame, area);
        })?;

        if app.should_quit {
            break;
        }

        // Handle events with shorter poll time when animating
        let poll_duration = if animation.is_animating() {
            Duration::from_millis(16) // ~60fps when animating
        } else {
            Duration::from_millis(50)
        };

        if let Some(input) = poll_host_input(poll_duration)? {
            match input {
                HostInputEvent::KeyPress { code, modifiers } => {
                    // Runtime-first path:
                    // 1) apply shared/core interaction when possible
                    // 2) fallback only for app-specific behaviors (completion/chat/crates extras)
                    let runtime_result =
                        try_apply_runtime_action(app, code, modifiers, &mut inspector_scroll);
                    if runtime_result.consumed {
                        if runtime_result.tab_changed {
                            animation.on_tab_change();
                        }
                        continue;
                    }
                    let intents = intents_for_key(app, code, modifiers);
                    for intent in intents {
                        let effects =
                            apply_intent(app, intent, &mut inspector_scroll, &mut animation);
                        execute_effects(app, effects);
                    }
                }
                HostInputEvent::MouseLeftDown { column: col, row } => {
                    if let Ok(size) = terminal.size() {
                        let area = Rect::new(0, 0, size.width, size.height);
                        if let Some(tabs_rect) = tabs_rect_for_area(area) {
                            if col >= tabs_rect.x
                                && col < tabs_rect.x + tabs_rect.width
                                && row >= tabs_rect.y
                                && row < tabs_rect.y + tabs_rect.height
                            {
                                let tab_specs = &app.ui_config.tabs;
                                let tab_count = tab_specs.len() as u16;
                                if tab_count == 0 {
                                    continue;
                                }
                                let inner_w = tabs_rect.width.saturating_sub(2);
                                if inner_w >= tab_count {
                                    let tab_width = inner_w / tab_count;
                                    let inner_x = tabs_rect.x + 1;
                                    let rel = col.saturating_sub(inner_x);
                                    let idx = (rel / tab_width).min(tab_count - 1) as usize;
                                    if let Some(spec) = tab_specs.get(idx) {
                                        let runtime_result =
                                            apply_runtime_set_tab(app, spec.id.clone());
                                        if runtime_result.tab_changed {
                                            animation.on_tab_change();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn keep_selection_visible_scroll(
    current_offset: usize,
    selected: Option<usize>,
    visible_height: usize,
    total_items: usize,
) -> usize {
    if total_items == 0 || visible_height == 0 {
        return 0;
    }
    let max_offset = total_items.saturating_sub(visible_height);
    let mut offset = current_offset.min(max_offset);
    let Some(sel) = selected else {
        return offset;
    };
    if sel < offset {
        offset = sel;
    } else if sel >= offset + visible_height {
        offset = sel + 1 - visible_height;
    }
    offset.min(max_offset)
}

fn execute_effects(app: &mut App, effects: Vec<UiEffect>) {
    for effect in effects {
        match effect {
            UiEffect::OpenUrl {
                url,
                success_message,
                failure_message,
            } => {
                if tui_kit_host::open_external(&url).is_ok() {
                    if let Some(msg) = success_message {
                        app.status_message = msg;
                    }
                } else if let Some(msg) = failure_message {
                    app.status_message = msg;
                }
            }
        }
    }
}
