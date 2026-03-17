use std::time::Duration;
use tui_kit_render::ui::app::list_viewport_height_for_area;
use tui_kit_render::theme::{Theme, ThemeKind};
use tui_kit_render::ui::{AnimationState, Focus, TabId, TuiKitUi, UiConfig};
use tui_kit_runtime::{
    apply_snapshot, dispatch_action, CoreAction, CoreEffect, CoreState, DataProvider, PaneFocus,
};

use crate::{
    action_from_keycode, execute_effects_to_status, global_command_for_key, poll_host_input,
    resolve_tab_action_with_current, terminal::with_terminal, HostGlobalCommand, HostInputEvent,
};

pub struct RuntimeLoopConfig {
    pub initial_tab_id: &'static str,
    pub tab_ids: &'static [&'static str],
    pub ui_config: fn() -> UiConfig,
}

pub trait RuntimeLoopHooks<P: DataProvider> {
    fn on_tick(&mut self, _provider: &mut P, _state: &mut CoreState) {}

    fn on_unhandled_input(
        &mut self,
        _provider: &mut P,
        _state: &mut CoreState,
        _input: HostInputEvent,
    ) -> bool {
        false
    }

    fn on_effects(
        &mut self,
        _provider: &mut P,
        _state: &mut CoreState,
        _effects: &[CoreEffect],
    ) {
    }
}

#[derive(Default)]
pub struct NoopRuntimeHooks;

impl<P: DataProvider> RuntimeLoopHooks<P> for NoopRuntimeHooks {}

pub fn run_provider_app<P: DataProvider>(
    provider: &mut P,
    cfg: RuntimeLoopConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    run_provider_app_with_hooks(provider, cfg, &mut NoopRuntimeHooks)
}

pub fn run_provider_app_with_hooks<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    cfg: RuntimeLoopConfig,
    hooks: &mut H,
) -> Result<(), Box<dyn std::error::Error>> {
    with_terminal(|terminal| {
        let mut theme = Theme::default();
        let mut state = CoreState {
            current_tab_id: cfg.initial_tab_id.to_string(),
            focus: PaneFocus::List,
            ..CoreState::default()
        };
        let mut inspector_scroll: usize = 0;
        let mut show_help = false;
        let mut show_settings = false;
        let mut animation = AnimationState::new();
        let mut last_selected_index: Option<usize> = None;
        let mut last_tab_id = state.current_tab_id.clone();
        let mut list_scroll_offset: usize = 0;

        if let Ok(snapshot) = provider.initialize() {
            apply_snapshot(&mut state, snapshot);
        }

        loop {
            hooks.on_tick(provider, &mut state);
            animation.update();

        if let Ok(size) = terminal.size() {
            let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
            let visible_height = list_viewport_height_for_area(area);
            list_scroll_offset = keep_selection_visible_scroll(
                list_scroll_offset,
                state.selected_index,
                visible_height,
                state.list_items.len(),
            );
        }

        if state.selected_index != last_selected_index {
            inspector_scroll = 0;
            animation.on_selection_change();
            last_selected_index = state.selected_index;
        }
        if state.current_tab_id != last_tab_id {
            animation.on_tab_change();
            last_tab_id = state.current_tab_id.clone();
        }

        terminal.draw(|frame| {
            let focus = ui_focus_from_pane(state.focus);
            let ui = TuiKitUi::new(&theme)
                .ui_config((cfg.ui_config)())
                .ui_summaries(&state.list_items)
                .ui_selected_detail(state.selected_detail.as_ref())
                .ui_total_count(state.total_count)
                .list_selected(state.selected_index)
                .list_scroll(list_scroll_offset)
                .search_input(&state.query)
                .current_tab_id(TabId::new(state.current_tab_id.clone()))
                .focus(focus)
                .status_message(state.status_message.as_deref().unwrap_or("ready"))
                .show_help(show_help)
                .show_settings(show_settings)
                .show_completion(false)
                .context_details_loading(false)
                .context_details_failed(false)
                .context_tree(&[])
                .filtered_context_indices(&[])
                .candidates(&[])
                .chat_messages(&state.chat_messages)
                .chat_input(&state.chat_input)
                .chat_loading(state.chat_loading)
                .chat_scroll(state.chat_scroll)
                .completion_selected(0)
                .inspector_scroll(inspector_scroll)
                .animation_state(&animation)
                .show_chat(state.chat_open)
                .in_context_items_view(false);
            let area = frame.area();
            ui.render_in_frame(frame, area);
        })?;

        let poll_duration = if animation.is_animating() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(80)
        };
        let Some(input) = poll_host_input(poll_duration)? else {
            continue;
        };
        let HostInputEvent::KeyPress { code, modifiers } = input else {
            if hooks.on_unhandled_input(provider, &mut state, input) {
                continue;
            }
            continue;
        };

        match global_command_for_key(
            code,
            modifiers,
            state.focus,
            show_help,
            show_settings,
            state.query.is_empty(),
        ) {
            HostGlobalCommand::None => {}
            HostGlobalCommand::CloseHelp => {
                show_help = false;
                continue;
            }
            HostGlobalCommand::CloseSettings => {
                show_settings = false;
                continue;
            }
            HostGlobalCommand::ToggleHelp => {
                show_help = true;
                continue;
            }
            HostGlobalCommand::ToggleTheme => {
                let next = ThemeKind::from_name(&theme.name).next();
                theme = Theme::from_kind(next);
                continue;
            }
            HostGlobalCommand::ToggleChat => {
                if let Ok(effects) = dispatch_action(provider, &mut state, &CoreAction::ToggleChat) {
                    hooks.on_effects(provider, &mut state, &effects);
                    execute_effects_to_status(&mut state, effects);
                }
                continue;
            }
            HostGlobalCommand::ToggleSettings => {
                show_settings = true;
                continue;
            }
            HostGlobalCommand::BackFromDetail => {
                state.focus = PaneFocus::List;
                continue;
            }
            HostGlobalCommand::ClearQuery => {
                if let Ok(effects) =
                    dispatch_action(provider, &mut state, &CoreAction::SetQuery(String::new()))
                {
                    hooks.on_effects(provider, &mut state, &effects);
                    execute_effects_to_status(&mut state, effects);
                }
                continue;
            }
            HostGlobalCommand::Quit => break Ok(()),
        }

        let action = action_from_keycode(code, state.focus).and_then(|a| {
            resolve_tab_action_with_current(a, cfg.tab_ids, Some(state.current_tab_id.as_str()))
        });
        let mut handled = false;

        if let Some(action) = action {
            handled = true;
            if matches!(
                action,
                CoreAction::MovePageDown | CoreAction::MovePageUp | CoreAction::MoveHome
            ) && state.focus == PaneFocus::Detail
            {
                match action {
                    CoreAction::MovePageDown => inspector_scroll = inspector_scroll.saturating_add(10),
                    CoreAction::MovePageUp => inspector_scroll = inspector_scroll.saturating_sub(10),
                    CoreAction::MoveHome => inspector_scroll = 0,
                    _ => {}
                }
                continue;
            }

            let should_reset_scroll = matches!(
                action,
                CoreAction::MoveNext
                    | CoreAction::MovePrev
                    | CoreAction::MoveHome
                    | CoreAction::MoveEnd
                    | CoreAction::MovePageDown
                    | CoreAction::MovePageUp
                    | CoreAction::OpenSelected
            );
            match dispatch_action(provider, &mut state, &action) {
                Ok(effects) => {
                    hooks.on_effects(provider, &mut state, &effects);
                    execute_effects_to_status(&mut state, effects)
                }
                Err(e) => state.status_message = Some(format!("Dispatch error: {}", e)),
            }
            if should_reset_scroll {
                inspector_scroll = 0;
            }
        }
        if !handled
            && hooks.on_unhandled_input(
                provider,
                &mut state,
                HostInputEvent::KeyPress { code, modifiers },
            )
        {
            continue;
        }
        }
    })
}

fn ui_focus_from_pane(focus: PaneFocus) -> Focus {
    match focus {
        PaneFocus::Search => Focus::Search,
        PaneFocus::List => Focus::List,
        PaneFocus::Tabs => Focus::Tabs,
        PaneFocus::Detail => Focus::Inspector,
        PaneFocus::Extra => Focus::Chat,
    }
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
