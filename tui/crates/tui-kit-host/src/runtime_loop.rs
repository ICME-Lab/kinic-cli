#[path = "form_tab_flow.rs"]
mod form_tab_flow;

use std::{io, time::Duration};
use tui_kit_render::theme::Theme;
use tui_kit_render::ui::app::list_viewport_height_for_area_with_tabs;
use tui_kit_render::ui::{AnimationState, Focus, TabId, TuiKitUi, UiConfig};
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreState, DataProvider, PaneFocus, apply_snapshot, dispatch_action,
    is_insert_form_locked, kinic_tabs::{
        KINIC_CREATE_TAB_ID, KINIC_MEMORIES_TAB_ID, KINIC_SETTINGS_TAB_ID, TabKind, tab_kind,
    },
    should_open_default_memory_picker,
};

use crate::{
    HostGlobalCommand, HostInputEvent, action_from_keycode, execute_effects_to_status,
    global_command_for_key, poll_host_input, resolve_tab_action_with_current,
    terminal::{FilePickerFn, PickFilePathError, pick_file_path, with_terminal},
};
use form_tab_flow::{form_tab_action_from_key, reset_form_focus, reset_form_state_for_tab};

pub struct RuntimeLoopConfig {
    pub initial_tab_id: &'static str,
    pub tab_ids: &'static [&'static str],
    pub initial_focus: PaneFocus,
    pub ui_config: fn() -> UiConfig,
    pub file_picker: Option<FilePickerFn>,
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
    DispatchError(String),
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
                let insert_file_path_display = insert_file_path_display(&state);
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
                    .default_memory_selector_selected_id(
                        state.default_memory_selector_selected_id.as_deref(),
                    )
                    .default_memory_selector_context(state.default_memory_selector_context)
                    .insert_mode(state.insert_mode)
                    .insert_memory_id(&state.insert_memory_id)
                    .insert_memory_placeholder(state.insert_memory_placeholder.as_deref())
                    .insert_tag(&state.insert_tag)
                    .insert_text(&state.insert_text)
                    .insert_file_path(insert_file_path_display.as_str())
                    .insert_embedding(&state.insert_embedding)
                    .insert_submit_state(state.insert_submit_state.clone())
                    .insert_spinner_frame(state.insert_spinner_frame)
                    .insert_error(state.insert_error.as_deref())
                    .insert_focus(state.insert_focus)
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
                OverlayInputResult::DispatchError(error) => {
                    state.status_message = Some(error);
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
                    if let Err(error) =
                        dispatch_with_effects(provider, &mut state, hooks, &CoreAction::ToggleChat)
                    {
                        state.status_message = Some(error);
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
                    if let Err(error) =
                        switch_to_tab(provider, &mut state, hooks, KINIC_MEMORIES_TAB_ID)
                    {
                        state.status_message = Some(error);
                    }
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
                    if let Err(error) =
                        dispatch_with_effects(provider, &mut state, hooks, &CoreAction::ToggleChat)
                    {
                        state.status_message = Some(error);
                    }
                    continue;
                }
                HostGlobalCommand::ToggleSettings => {
                    match dispatch_with_effects(
                        provider,
                        &mut state,
                        hooks,
                        &CoreAction::ToggleSettings,
                    ) {
                        Ok(()) => show_settings = true,
                        Err(error) => state.status_message = Some(error),
                    }
                    continue;
                }
                HostGlobalCommand::SetDefaultFromSelection => {
                    if let Err(error) = dispatch_with_effects(
                        provider,
                        &mut state,
                        hooks,
                        &CoreAction::SetDefaultMemoryFromSelection,
                    ) {
                        state.status_message = Some(error);
                    }
                    continue;
                }
                HostGlobalCommand::BackFromContent => {
                    state.focus = PaneFocus::Items;
                    continue;
                }
                HostGlobalCommand::RefreshCurrentView => {
                    if let Err(error) = dispatch_with_effects(
                        provider,
                        &mut state,
                        hooks,
                        &CoreAction::RefreshCurrentView,
                    ) {
                        state.status_message = Some(error);
                    }
                    continue;
                }
                HostGlobalCommand::ClearQuery => {
                    if let Err(error) = dispatch_with_effects(
                        provider,
                        &mut state,
                        hooks,
                        &CoreAction::SetQuery(String::new()),
                    ) {
                        state.status_message = Some(error);
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
                if should_open_insert_file_dialog(&action, &state) {
                    match open_insert_file_dialog(&mut state, terminal, cfg.file_picker) {
                        Ok(effects) => {
                            hooks.on_effects(provider, &mut state, &effects);
                            execute_effects_to_status(&mut state, effects);
                        }
                        Err(PickFilePathError::Picker(error)) => {
                            execute_effects_to_status(
                                &mut state,
                                vec![CoreEffect::Notify(error)],
                            );
                        }
                        Err(PickFilePathError::TerminalState(error)) => {
                            return Err(Box::new(io::Error::other(error)));
                        }
                    }
                    continue;
                }
                match dispatch_action_with_persistent_clear(provider, &mut state, &action) {
                    Ok(effects) => {
                        if matches!(&action, CoreAction::SetTab(_)) {
                            normalize_focus_after_set_tab(&mut state);
                        }
                        hooks.on_effects(provider, &mut state, &effects);
                        execute_effects_to_status(&mut state, effects)
                    }
                    Err(e) => state.status_message = Some(dispatch_error_message(&e)),
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

fn open_insert_file_dialog(
    state: &mut CoreState,
    terminal: &mut crate::terminal::HostTerminal,
    file_picker: Option<FilePickerFn>,
) -> Result<Vec<CoreEffect>, PickFilePathError> {
    let Some(file_picker) = file_picker else {
        return Ok(vec![CoreEffect::Notify(
            "File picker is unavailable in this build.".to_string(),
        )]);
    };
    let selection = pick_file_path(terminal, file_picker, state.insert_mode)?;
    Ok(apply_insert_file_dialog_selection(state, selection))
}

fn apply_insert_file_dialog_selection(
    state: &mut CoreState,
    selection: Option<std::path::PathBuf>,
) -> Vec<CoreEffect> {
    let Some(path) = selection else {
        return vec![CoreEffect::Notify("File selection canceled.".to_string())];
    };

    let display_path = path.display().to_string();
    state.insert_file_path_input = display_path.clone();
    state.insert_selected_file_path = Some(path);
    state.insert_error = None;
    if state.insert_submit_state == tui_kit_runtime::CreateSubmitState::Error {
        state.insert_submit_state = tui_kit_runtime::CreateSubmitState::Idle;
    }
    vec![CoreEffect::Notify(format!("Selected file: {display_path}"))]
}

fn insert_file_path_display(state: &CoreState) -> String {
    state
        .insert_selected_file_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| state.insert_file_path_input.clone())
}

fn should_open_insert_file_dialog(action: &CoreAction, state: &CoreState) -> bool {
    matches!(action, CoreAction::InsertOpenFileDialog) && !is_insert_form_locked(state)
}

fn normalize_focus_after_set_tab(state: &mut CoreState) {
    if matches!(
        tab_kind(state.current_tab_id.as_str()),
        TabKind::InsertForm | TabKind::CreateForm
    ) {
        reset_form_focus(state);
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
        return match dispatch_action_with_persistent_clear(provider, state, &action) {
            Ok(effects) => OverlayInputResult::ApplyEffects(effects),
            Err(error) => OverlayInputResult::DispatchError(dispatch_error_message(&error)),
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
    let should_reset_form_state = reset_form_state
        && state.current_tab_id != tab_id
        && matches!(tab_kind(tab_id), TabKind::InsertForm | TabKind::CreateForm);
    match dispatch_tab_with_rollback(provider, state, hooks, tab_id) {
        Ok(()) => {
            if should_reset_form_state {
                reset_form_state_for_tab(state, tab_id);
            }
            normalize_focus_after_set_tab(state);
        }
        Err(error) => state.status_message = Some(error),
    }
}

fn dispatch_with_effects<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    state: &mut CoreState,
    hooks: &mut H,
    action: &CoreAction,
) -> Result<(), String> {
    match dispatch_action_with_persistent_clear(provider, state, action) {
        Ok(effects) => {
            hooks.on_effects(provider, state, &effects);
            execute_effects_to_status(state, effects);
            Ok(())
        }
        Err(error) => Err(dispatch_error_message(&error)),
    }
}

fn dispatch_action_with_persistent_clear(
    provider: &mut impl DataProvider,
    state: &mut CoreState,
    action: &CoreAction,
) -> tui_kit_runtime::CoreResult<Vec<CoreEffect>> {
    if should_clear_persistent_status(action) {
        state.persistent_status_message = None;
        state.status_message = None;
    }
    dispatch_action(provider, state, action)
}

fn should_clear_persistent_status(action: &CoreAction) -> bool {
    matches!(
        action,
        CoreAction::InsertInput(_)
            | CoreAction::InsertBackspace
            | CoreAction::InsertCycleModePrev
            | CoreAction::InsertCycleMode
            | CoreAction::InsertSubmit
            | CoreAction::OpenDefaultMemoryPicker
            | CoreAction::SubmitDefaultMemoryPicker
            | CoreAction::CreateInput(_)
            | CoreAction::CreateBackspace
            | CoreAction::CreateSubmit
            | CoreAction::SearchInput(_)
            | CoreAction::SearchBackspace
            | CoreAction::SetQuery(_)
            | CoreAction::SearchSubmit
            | CoreAction::SetTab(_)
    )
}

fn switch_to_tab<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    state: &mut CoreState,
    hooks: &mut H,
    tab_id: &str,
) -> Result<(), String> {
    dispatch_tab_with_rollback(provider, state, hooks, tab_id)?;
    normalize_focus_after_set_tab(state);
    Ok(())
}

fn dispatch_tab_with_rollback<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    state: &mut CoreState,
    hooks: &mut H,
    tab_id: &str,
) -> Result<(), String> {
    let previous_state = state.clone();
    match dispatch_with_effects(provider, state, hooks, &CoreAction::SetTab(tab_id.into())) {
        Ok(()) => Ok(()),
        Err(error) => {
            *state = previous_state;
            Err(error)
        }
    }
}

fn dispatch_error_message(error: &dyn std::fmt::Display) -> String {
    format!("Dispatch error: {}", error)
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
#[path = "runtime_loop_tests.rs"]
mod tests;
