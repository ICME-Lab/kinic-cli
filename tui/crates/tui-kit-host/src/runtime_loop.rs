#[path = "form_tab_flow.rs"]
mod form_tab_flow;

use std::time::Duration;
use tui_kit_render::theme::Theme;
use tui_kit_render::ui::app::list_viewport_height_for_area_with_tabs;
use tui_kit_render::ui::{AnimationState, Focus, TabId, TuiKitUi, UiConfig};
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreState, DataProvider, PaneFocus, apply_snapshot, dispatch_action,
    kinic_tabs::{
        KINIC_CREATE_TAB_ID, KINIC_MEMORIES_TAB_ID, KINIC_SETTINGS_TAB_ID, TabKind, tab_kind,
    },
    should_open_default_memory_picker,
};

use crate::{
    HostGlobalCommand, HostInputEvent, action_from_keycode, execute_effects_to_status,
    global_command_for_key, poll_host_input, resolve_tab_action_with_current,
    terminal::with_terminal,
};
use form_tab_flow::{form_tab_action_from_key, reset_create_focus, reset_create_form_state};

pub struct RuntimeLoopConfig {
    pub initial_tab_id: &'static str,
    pub tab_ids: &'static [&'static str],
    pub initial_focus: PaneFocus,
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

    fn on_effects(&mut self, _provider: &mut P, _state: &mut CoreState, _effects: &[CoreEffect]) {}
}

#[derive(Default)]
pub struct NoopRuntimeHooks;

impl<P: DataProvider> RuntimeLoopHooks<P> for NoopRuntimeHooks {}

enum OverlayInputResult {
    NotHandled,
    Consumed,
    CloseSettings,
    ApplyEffects(Vec<CoreEffect>),
}

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
        let theme = Theme::default();
        let mut state = CoreState {
            current_tab_id: cfg.initial_tab_id.to_string(),
            focus: cfg.initial_focus,
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
                let visible_height =
                    list_viewport_height_for_area_with_tabs(area, !cfg.tab_ids.is_empty());
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
                    .ui_selected_content(state.selected_content.as_ref())
                    .ui_total_count(state.total_count)
                    .list_selected(state.selected_index)
                    .list_scroll(list_scroll_offset)
                    .search_input(&state.query)
                    .current_tab_id(TabId::new(state.current_tab_id.clone()))
                    .focus(focus)
                    .status_message(state.status_message.as_deref().unwrap_or("ready"))
                    .show_help(show_help)
                    .show_settings(show_settings)
                    .show_create_modal(false)
                    .create_name(&state.create_name)
                    .create_description(&state.create_description)
                    .create_submit_state(state.create_submit_state.clone())
                    .create_spinner_frame(state.create_spinner_frame)
                    .create_error(state.create_error.as_deref())
                    .create_focus(state.create_focus)
                    .create_cost_state(&state.create_cost_state)
                    .settings_snapshot(Some(&state.settings))
                    .default_memory_selector_open(state.default_memory_selector_open)
                    .default_memory_selector_index(state.default_memory_selector_index)
                    .default_memory_selector_items(&state.default_memory_selector_items)
                    .default_memory_selector_labels(&state.default_memory_selector_labels)
                    .default_memory_selector_selected_id(
                        state.default_memory_selector_selected_id.as_deref(),
                    )
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

            match handle_overlay_input(provider, &mut state, show_settings, code, modifiers) {
                OverlayInputResult::NotHandled => {}
                OverlayInputResult::Consumed => continue,
                OverlayInputResult::CloseSettings => {
                    show_settings = false;
                    continue;
                }
                OverlayInputResult::ApplyEffects(effects) => {
                    hooks.on_effects(provider, &mut state, &effects);
                    execute_effects_to_status(&mut state, effects);
                    continue;
                }
            }

            match global_command_for_key(
                code,
                modifiers,
                state.focus,
                state.current_tab_id.as_str(),
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
                HostGlobalCommand::CloseChat => {
                    if let Ok(effects) =
                        dispatch_action(provider, &mut state, &CoreAction::ToggleChat)
                    {
                        hooks.on_effects(provider, &mut state, &effects);
                        execute_effects_to_status(&mut state, effects);
                    }
                    continue;
                }
                HostGlobalCommand::BackToTabs => {
                    state.focus = PaneFocus::Tabs;
                    continue;
                }
                HostGlobalCommand::BackFromFormToTabs => {
                    state.focus = PaneFocus::Tabs;
                    continue;
                }
                HostGlobalCommand::BackToMemoriesTab => {
                    if let Ok(effects) = dispatch_action(
                        provider,
                        &mut state,
                        &CoreAction::SetTab(KINIC_MEMORIES_TAB_ID.into()),
                    ) {
                        hooks.on_effects(provider, &mut state, &effects);
                        execute_effects_to_status(&mut state, effects);
                    }
                    normalize_focus_after_set_tab(&mut state);
                    continue;
                }
                HostGlobalCommand::OpenCreateTab => {
                    open_form_tab(provider, &mut state, hooks, KINIC_CREATE_TAB_ID, true);
                    continue;
                }
                HostGlobalCommand::ToggleHelp => {
                    show_help = true;
                    continue;
                }
                HostGlobalCommand::ToggleChat => {
                    if let Ok(effects) =
                        dispatch_action(provider, &mut state, &CoreAction::ToggleChat)
                    {
                        hooks.on_effects(provider, &mut state, &effects);
                        execute_effects_to_status(&mut state, effects);
                    }
                    continue;
                }
                HostGlobalCommand::ToggleSettings => {
                    if let Ok(effects) =
                        dispatch_action(provider, &mut state, &CoreAction::ToggleSettings)
                    {
                        hooks.on_effects(provider, &mut state, &effects);
                        execute_effects_to_status(&mut state, effects);
                    }
                    show_settings = true;
                    continue;
                }
                HostGlobalCommand::SetDefaultFromSelection => {
                    if let Ok(effects) = dispatch_action(
                        provider,
                        &mut state,
                        &CoreAction::SetDefaultMemoryFromSelection,
                    ) {
                        hooks.on_effects(provider, &mut state, &effects);
                        execute_effects_to_status(&mut state, effects);
                    }
                    continue;
                }
                HostGlobalCommand::BackFromContent => {
                    state.focus = PaneFocus::Items;
                    continue;
                }
                HostGlobalCommand::RefreshCurrentView => {
                    if let Ok(effects) =
                        dispatch_action(provider, &mut state, &CoreAction::RefreshCurrentView)
                    {
                        hooks.on_effects(provider, &mut state, &effects);
                        execute_effects_to_status(&mut state, effects);
                    }
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

            let action = form_tab_action_from_key(code, &mut state).or_else(|| {
                if code == crossterm::event::KeyCode::Enter
                    && should_open_default_memory_picker(&state)
                {
                    return Some(CoreAction::OpenDefaultMemoryPicker);
                }
                action_from_keycode(code, state.focus, state.current_tab_id.as_str()).and_then(
                    |a| {
                        resolve_tab_action_with_current(
                            a,
                            cfg.tab_ids,
                            Some(state.current_tab_id.as_str()),
                        )
                    },
                )
            });
            let mut handled = false;

            if let Some(action) = action {
                handled = true;
                if state.focus == PaneFocus::Content
                    && state.current_tab_id != KINIC_SETTINGS_TAB_ID
                    && apply_content_scroll_action(&action, &mut inspector_scroll)
                {
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
                        if matches!(&action, CoreAction::SetTab(_)) {
                            normalize_focus_after_set_tab(&mut state);
                        }
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

fn normalize_focus_after_set_tab(state: &mut CoreState) {
    if matches!(tab_kind(state.current_tab_id.as_str()), TabKind::Form) {
        reset_create_focus(state);
    }
    state.focus = PaneFocus::Tabs;
}

fn ui_focus_from_pane(focus: PaneFocus) -> Focus {
    match focus {
        PaneFocus::Search => Focus::Search,
        PaneFocus::Items => Focus::Items,
        PaneFocus::Tabs => Focus::Tabs,
        PaneFocus::Content => Focus::Content,
        PaneFocus::Form => Focus::Form,
        PaneFocus::Extra => Focus::Chat,
    }
}

fn handle_overlay_input<P: DataProvider>(
    provider: &mut P,
    state: &mut CoreState,
    show_settings: bool,
    code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) -> OverlayInputResult {
    if state.default_memory_selector_open {
        let Some(action) = selector_overlay_action(code, modifiers) else {
            return OverlayInputResult::Consumed;
        };
        return match dispatch_action(provider, state, &action) {
            Ok(effects) => OverlayInputResult::ApplyEffects(effects),
            Err(_) => OverlayInputResult::Consumed,
        };
    }

    if show_settings && matches!(code, crossterm::event::KeyCode::Esc) {
        return OverlayInputResult::CloseSettings;
    }

    OverlayInputResult::NotHandled
}

fn selector_overlay_action(
    code: crossterm::event::KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
) -> Option<CoreAction> {
    match code {
        crossterm::event::KeyCode::Esc => Some(CoreAction::CloseDefaultMemoryPicker),
        crossterm::event::KeyCode::Enter => Some(CoreAction::SubmitDefaultMemoryPicker),
        crossterm::event::KeyCode::Down => Some(CoreAction::MoveDefaultMemoryPickerNext),
        crossterm::event::KeyCode::Up => Some(CoreAction::MoveDefaultMemoryPickerPrev),
        _ => None,
    }
}

fn open_form_tab<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    state: &mut CoreState,
    hooks: &mut H,
    tab_id: &str,
    reset_form_state: bool,
) {
    if reset_form_state
        && state.current_tab_id != tab_id
        && matches!(tab_kind(tab_id), TabKind::Form)
    {
        reset_create_form_state(state);
    }
    if let Ok(effects) = dispatch_action(provider, state, &CoreAction::SetTab(tab_id.into())) {
        hooks.on_effects(provider, state, &effects);
        execute_effects_to_status(state, effects);
    }
    normalize_focus_after_set_tab(state);
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

fn apply_content_scroll_action(action: &CoreAction, inspector_scroll: &mut usize) -> bool {
    match action {
        CoreAction::ScrollContentPageDown => {
            *inspector_scroll = inspector_scroll.saturating_add(10);
            true
        }
        CoreAction::ScrollContentPageUp => {
            *inspector_scroll = inspector_scroll.saturating_sub(10);
            true
        }
        CoreAction::ScrollContentHome => {
            *inspector_scroll = 0;
            true
        }
        CoreAction::ScrollContentEnd => {
            *inspector_scroll = inspector_scroll.saturating_add(9999);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tui_kit_runtime::kinic_tabs::{
        KINIC_CREATE_TAB_ID, KINIC_MARKET_TAB_ID, KINIC_MEMORIES_TAB_ID,
    };

    #[test]
    fn normalize_focus_keeps_memories_on_tabs_after_tab_switch() {
        let mut state = CoreState {
            current_tab_id: KINIC_MEMORIES_TAB_ID.to_string(),
            focus: PaneFocus::Items,
            ..CoreState::default()
        };

        normalize_focus_after_set_tab(&mut state);

        assert_eq!(state.focus, PaneFocus::Tabs);
    }

    #[test]
    fn normalize_focus_resets_create_tab_to_tabs_and_name_field() {
        let mut state = CoreState {
            current_tab_id: KINIC_CREATE_TAB_ID.to_string(),
            focus: PaneFocus::Content,
            create_focus: tui_kit_runtime::CreateModalFocus::Submit,
            ..CoreState::default()
        };

        normalize_focus_after_set_tab(&mut state);

        assert_eq!(state.focus, PaneFocus::Tabs);
        assert_eq!(state.create_focus, tui_kit_runtime::CreateModalFocus::Name);
    }

    #[test]
    fn normalize_focus_keeps_placeholder_tabs_on_tabs() {
        let mut state = CoreState {
            current_tab_id: KINIC_MARKET_TAB_ID.to_string(),
            focus: PaneFocus::Content,
            ..CoreState::default()
        };

        normalize_focus_after_set_tab(&mut state);

        assert_eq!(state.focus, PaneFocus::Tabs);
    }

    #[test]
    fn selector_overlay_action_maps_arrow_keys_only() {
        assert_eq!(
            selector_overlay_action(
                crossterm::event::KeyCode::Down,
                crossterm::event::KeyModifiers::NONE
            ),
            Some(CoreAction::MoveDefaultMemoryPickerNext)
        );
        assert_eq!(
            selector_overlay_action(
                crossterm::event::KeyCode::Up,
                crossterm::event::KeyModifiers::NONE
            ),
            Some(CoreAction::MoveDefaultMemoryPickerPrev)
        );
        assert_eq!(
            selector_overlay_action(
                crossterm::event::KeyCode::Char('j'),
                crossterm::event::KeyModifiers::NONE
            ),
            None
        );
    }

    #[test]
    fn content_scroll_helper_handles_scroll_end_only() {
        let mut inspector_scroll = 3usize;

        assert!(apply_content_scroll_action(
            &CoreAction::ScrollContentEnd,
            &mut inspector_scroll
        ));
        assert_eq!(inspector_scroll, 10002);

        assert!(!apply_content_scroll_action(
            &CoreAction::MoveEnd,
            &mut inspector_scroll
        ));
        assert_eq!(inspector_scroll, 10002);
    }
}
