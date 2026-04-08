#[path = "form_tab_flow.rs"]
mod form_tab_flow;

use ratatui_textarea::{Input as TextAreaInput, Key as TextAreaKey, TextArea};
use std::{io, time::Duration};
use tui_kit_render::theme::Theme;
use tui_kit_render::ui::app::list_viewport_height_for_area_with_tabs;
use tui_kit_render::ui::{AnimationState, Focus, TabId, TuiKitUi, UiConfig};
use tui_kit_runtime::{
    CoreAction, CoreEffect, CoreState, DataProvider, PaneFocus, PickerState, apply_snapshot,
    chat_commands::{
        UNKNOWN_SLASH_COMMAND_MESSAGE, chat_slash_command_action, flatten_chat_input_for_display,
        matching_slash_commands, normalize_chat_input_lines, selected_slash_command_action,
    },
    dispatch_action, is_insert_form_locked,
    kinic_tabs::{
        KINIC_CREATE_TAB_ID, KINIC_MEMORIES_TAB_ID, KINIC_SETTINGS_TAB_ID, TabKind, tab_kind,
    },
    selected_settings_row_behavior,
};

use crate::{
    HostGlobalCommand, HostInputEvent, action_from_keycode, execute_effects_to_status,
    global_command_for_key,
    picker::PickerBackend,
    poll_host_input, resolve_tab_action_with_current,
    terminal::{PickFilePathError, pick_file_path, with_terminal},
};
use form_tab_flow::{form_tab_action_from_key, reset_form_focus, reset_form_state_for_tab};

pub struct RuntimeLoopConfig {
    pub initial_tab_id: &'static str,
    pub tab_ids: &'static [&'static str],
    pub initial_focus: PaneFocus,
    pub ui_config: fn() -> UiConfig,
    pub file_picker: Option<Box<dyn PickerBackend>>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveTextarea {
    CreateDescription,
    InsertText,
    ChatInput,
}

// Memories chat uses a dedicated single-line model so chat shortcuts and paste
// behavior stay independent from multiline textarea defaults.
#[derive(Default)]
struct ChatInputState {
    value: String,
    cursor_col: usize,
}

impl ChatInputState {
    #[cfg(test)]
    fn lines(&self) -> Vec<String> {
        vec![self.value.clone()]
    }

    #[cfg(test)]
    fn cursor(&self) -> (usize, usize) {
        (0, self.cursor_col)
    }

    #[cfg(test)]
    fn input(&mut self, input: TextAreaInput) {
        if let Some(action) = chat_edit_from_textarea_input(input) {
            apply_chat_edit(self, action);
        }
    }
}

struct FormTextareas {
    create_description: TextArea<'static>,
    insert_text: TextArea<'static>,
    chat_input: ChatInputState,
    chat_command_selected: usize,
}

impl Default for FormTextareas {
    fn default() -> Self {
        Self {
            create_description: textarea_from_text(""),
            insert_text: textarea_from_text(""),
            chat_input: ChatInputState::default(),
            chat_command_selected: 0,
        }
    }
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
        let RuntimeLoopConfig {
            initial_tab_id,
            tab_ids,
            initial_focus,
            ui_config,
            file_picker,
        } = cfg;
        let mut file_picker = file_picker;
        let theme = Theme::default();
        let mut state = CoreState {
            current_tab_id: initial_tab_id.to_string(),
            focus: initial_focus,
            ..CoreState::default()
        };
        let mut inspector_scroll: usize = 0;
        let mut show_help = false;
        let mut show_settings = false;
        let mut animation = AnimationState::new();
        let mut last_selected_index: Option<usize> = None;
        let mut last_tab_id = state.current_tab_id.clone();
        let mut list_scroll_offset: usize = 0;
        let mut textareas = FormTextareas::default();

        if let Ok(snapshot) = provider.initialize() {
            apply_snapshot(&mut state, snapshot);
        }
        sync_form_textareas_from_state(&mut textareas, &state);

        loop {
            hooks.on_tick(provider, &mut state);
            animation.update();
            sync_form_textareas_from_state(&mut textareas, &state);

            if let Ok(size) = terminal.size() {
                let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                let visible_height =
                    list_viewport_height_for_area_with_tabs(area, !tab_ids.is_empty());
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
                let visible_chat_input = chat_input_display_value(&textareas.chat_input);
                let ui = TuiKitUi::new(&theme)
                    .ui_config(ui_config())
                    .ui_summaries(&state.list_items)
                    .ui_selected_content(state.selected_content.as_ref())
                    .ui_total_count(state.total_count)
                    .list_selected(state.selected_index)
                    .list_scroll(list_scroll_offset)
                    .search_input(&state.query)
                    .search_scope(state.search_scope)
                    .current_tab_id(TabId::new(state.current_tab_id.clone()))
                    .focus(focus)
                    .status_message(state.status_message.as_deref().unwrap_or("ready"))
                    .selected_memory_label(state.selected_memory_label.as_deref())
                    .chat_scope_label_value(state.chat_scope_label.as_deref())
                    .show_help(show_help)
                    .show_settings(show_settings)
                    .show_create_modal(false)
                    .create_name(&state.create_name)
                    .create_description(&state.create_description)
                    .create_description_cursor(textarea_cursor(
                        active_textarea(&state),
                        ActiveTextarea::CreateDescription,
                        &textareas.create_description,
                    ))
                    .create_submit_state(state.create_submit_state.clone())
                    .create_spinner_frame(state.create_spinner_frame)
                    .create_error(state.create_error.as_deref())
                    .create_focus(state.create_focus)
                    .create_cost_state(&state.create_cost_state)
                    .settings_snapshot(Some(&state.settings))
                    .picker(&state.picker)
                    .saved_default_memory_id(state.saved_default_memory_id.as_deref())
                    .insert_mode(state.insert_mode)
                    .insert_memory_id(&state.insert_memory_id)
                    .insert_memory_placeholder(state.insert_memory_placeholder.as_deref())
                    .insert_expected_dim(state.insert_expected_dim)
                    .insert_expected_dim_loading(state.insert_expected_dim_loading)
                    .insert_current_dim(state.insert_current_dim.as_deref())
                    .insert_validation_message(state.insert_validation_message.as_deref())
                    .insert_tag(&state.insert_tag)
                    .insert_text(&state.insert_text)
                    .insert_text_cursor(textarea_cursor(
                        active_textarea(&state),
                        ActiveTextarea::InsertText,
                        &textareas.insert_text,
                    ))
                    .insert_file_path(insert_file_path_display.as_str())
                    .insert_embedding(&state.insert_embedding)
                    .insert_submit_state(state.insert_submit_state.clone())
                    .insert_spinner_frame(state.insert_spinner_frame)
                    .insert_error(state.insert_error.as_deref())
                    .insert_focus(state.insert_focus)
                    .access_control_modal(state.access_control.clone())
                    .add_memory_modal(state.add_memory.clone())
                    .remove_memory_modal(&state.remove_memory)
                    .rename_memory_modal(state.rename_memory.clone())
                    .transfer_modal(state.transfer_modal.clone())
                    .show_completion(false)
                    .context_details_loading(false)
                    .context_details_failed(false)
                    .context_tree(&[])
                    .filtered_context_indices(&[])
                    .candidates(&[])
                    .chat_messages(&state.chat_messages)
                    .chat_input(visible_chat_input.as_str())
                    .chat_input_cursor(chat_input_cursor(
                        active_textarea(&state),
                        &textareas.chat_input,
                    ))
                    .chat_command_selected(chat_command_selection(&state, &textareas))
                    .chat_loading(state.chat_loading)
                    .chat_scroll(state.chat_scroll)
                    .chat_scope(state.chat_scope)
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
            if matches!(input, HostInputEvent::Paste(_))
                && handle_textarea_input(provider, &mut state, hooks, &mut textareas, &input)?
            {
                continue;
            }
            if let HostInputEvent::Paste(text) = &input {
                if handle_paste_input(provider, &mut state, hooks, text.as_str())? {
                    continue;
                }
            }
            let (_key_event, code, modifiers) = match &input {
                HostInputEvent::Key {
                    key_event,
                    code,
                    modifiers,
                } => (Some(*key_event), Some(*code), Some(*modifiers)),
                HostInputEvent::Paste(_) => (None, None, None),
            };

            if let (Some(code), Some(modifiers)) = (code, modifiers) {
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
            }

            if handle_textarea_input(provider, &mut state, hooks, &mut textareas, &input)? {
                continue;
            }

            if let (Some(code), Some(modifiers)) = (code, modifiers) {
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
                        if let Err(error) = dispatch_with_effects(
                            provider,
                            &mut state,
                            hooks,
                            &CoreAction::ToggleChat,
                        ) {
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
                        if let Err(error) = dispatch_with_effects(
                            provider,
                            &mut state,
                            hooks,
                            &CoreAction::ToggleChat,
                        ) {
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
                    HostGlobalCommand::OpenRenameMemory => {
                        if let Err(error) = dispatch_with_effects(
                            provider,
                            &mut state,
                            hooks,
                            &CoreAction::OpenRenameMemory,
                        ) {
                            state.status_message = Some(error);
                        }
                        continue;
                    }
                    HostGlobalCommand::BackFromContent => {
                        state.focus = PaneFocus::Items;
                        continue;
                    }
                    HostGlobalCommand::BackFromItems => {
                        state.focus = PaneFocus::Search;
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
            }

            let action = code.and_then(|code| {
                form_tab_action_from_key(code, &mut state).or_else(|| {
                    if code == crossterm::event::KeyCode::Enter {
                        if let Some(action) = selected_settings_row_behavior(&state)
                            .and_then(|behavior| behavior.enter_action)
                        {
                            return Some(action);
                        }
                    }
                    action_from_keycode(code, state.focus, state.current_tab_id.as_str()).and_then(
                        |a| {
                            resolve_tab_action_with_current(
                                a,
                                tab_ids,
                                Some(state.current_tab_id.as_str()),
                            )
                        },
                    )
                })
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
                    match open_insert_file_dialog(&mut state, terminal, &mut file_picker) {
                        Ok(effects) => {
                            hooks.on_effects(provider, &mut state, &effects);
                            execute_effects_to_status(&mut state, effects);
                        }
                        Err(PickFilePathError::Picker(error)) => {
                            execute_effects_to_status(&mut state, vec![CoreEffect::Notify(error)]);
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
            if !handled && hooks.on_unhandled_input(provider, &mut state, input) {
                continue;
            }
        }
    })
}

fn open_insert_file_dialog(
    state: &mut CoreState,
    terminal: &mut crate::terminal::HostTerminal,
    file_picker: &mut Option<Box<dyn PickerBackend>>,
) -> Result<Vec<CoreEffect>, PickFilePathError> {
    let Some(file_picker) = file_picker.as_deref_mut() else {
        return Ok(vec![CoreEffect::Notify(
            "File picker is unavailable in this build.".to_string(),
        )]);
    };
    let cwd =
        std::env::current_dir().map_err(|error| PickFilePathError::Picker(error.to_string()))?;
    let selection = pick_file_path(terminal, file_picker, cwd.as_path(), state.insert_mode)?;
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
    state.insert_focus = tui_kit_runtime::InsertFormFocus::Submit;
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

#[cfg(test)]
fn build_ui<'a>(
    theme: &'a Theme,
    cfg: &RuntimeLoopConfig,
    state: &'a CoreState,
    textareas: &'a FormTextareas,
    list_scroll_offset: usize,
    inspector_scroll: usize,
    show_help: bool,
    show_settings: bool,
    animation: &'a AnimationState,
) -> TuiKitUi<'a> {
    let focus = ui_focus_from_pane(state.focus);
    TuiKitUi::new(theme)
        .ui_config((cfg.ui_config)())
        .ui_summaries(&state.list_items)
        .ui_selected_content(state.selected_content.as_ref())
        .ui_total_count(state.total_count)
        .list_selected(state.selected_index)
        .list_scroll(list_scroll_offset)
        .search_input(&state.query)
        .search_scope(state.search_scope)
        .current_tab_id(TabId::new(state.current_tab_id.clone()))
        .focus(focus)
        .status_message(state.status_message.as_deref().unwrap_or("ready"))
        .selected_memory_label(state.selected_memory_label.as_deref())
        .chat_scope_label_value(state.chat_scope_label.as_deref())
        .show_help(show_help)
        .show_settings(show_settings)
        .show_create_modal(false)
        .create_name(&state.create_name)
        .create_description(&state.create_description)
        .create_description_cursor(textarea_cursor(
            active_textarea(state),
            ActiveTextarea::CreateDescription,
            &textareas.create_description,
        ))
        .create_submit_state(state.create_submit_state.clone())
        .create_spinner_frame(state.create_spinner_frame)
        .create_error(state.create_error.as_deref())
        .create_focus(state.create_focus)
        .create_cost_state(&state.create_cost_state)
        .settings_snapshot(Some(&state.settings))
        .picker(&state.picker)
        .saved_default_memory_id(state.saved_default_memory_id.as_deref())
        .insert_mode(state.insert_mode)
        .insert_memory_id(&state.insert_memory_id)
        .insert_memory_placeholder(state.insert_memory_placeholder.as_deref())
        .insert_expected_dim(state.insert_expected_dim)
        .insert_expected_dim_loading(state.insert_expected_dim_loading)
        .insert_current_dim(state.insert_current_dim.as_deref())
        .insert_validation_message(state.insert_validation_message.as_deref())
        .insert_tag(&state.insert_tag)
        .insert_text(&state.insert_text)
        .insert_text_cursor(textarea_cursor(
            active_textarea(state),
            ActiveTextarea::InsertText,
            &textareas.insert_text,
        ))
        .insert_file_path(&state.insert_file_path_input)
        .insert_embedding(&state.insert_embedding)
        .insert_submit_state(state.insert_submit_state.clone())
        .insert_spinner_frame(state.insert_spinner_frame)
        .insert_error(state.insert_error.as_deref())
        .insert_focus(state.insert_focus)
        .access_control_modal(state.access_control.clone())
        .add_memory_modal(state.add_memory.clone())
        .remove_memory_modal(&state.remove_memory)
        .rename_memory_modal(state.rename_memory.clone())
        .transfer_modal(state.transfer_modal.clone())
        .show_completion(false)
        .context_details_loading(false)
        .context_details_failed(false)
        .context_tree(&[])
        .filtered_context_indices(&[])
        .candidates(&[])
        .chat_messages(&state.chat_messages)
        .chat_input(&textareas.chat_input.value)
        .chat_input_cursor(chat_input_cursor(
            active_textarea(state),
            &textareas.chat_input,
        ))
        .chat_command_selected(chat_command_selection(state, textareas))
        .chat_loading(state.chat_loading)
        .chat_scroll(state.chat_scroll)
        .chat_scope(state.chat_scope)
        .completion_selected(0)
        .inspector_scroll(inspector_scroll)
        .animation_state(animation)
        .show_chat(state.chat_open)
        .in_context_items_view(false)
}

fn handle_overlay_input<P: DataProvider>(
    provider: &mut P,
    state: &mut CoreState,
    show_settings: bool,
    code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) -> OverlayInputResult {
    if state.access_control.open {
        return dispatch_overlay_action(
            provider,
            state,
            access_control_overlay_action(code, modifiers, state),
            false,
        );
    }

    if state.add_memory.open {
        return dispatch_overlay_action(
            provider,
            state,
            add_memory_overlay_action(code, modifiers, state),
            false,
        );
    }

    if state.remove_memory.open {
        return dispatch_overlay_action(
            provider,
            state,
            remove_memory_overlay_action(code, modifiers, state),
            false,
        );
    }

    if state.rename_memory.form.open {
        return dispatch_overlay_action(
            provider,
            state,
            rename_overlay_action(code, modifiers, state),
            false,
        );
    }

    if state.transfer_modal.open {
        return dispatch_overlay_action(
            provider,
            state,
            transfer_overlay_action(code, modifiers, state),
            false,
        );
    }

    if !matches!(state.picker, PickerState::Closed) {
        return dispatch_overlay_action(
            provider,
            state,
            picker_overlay_action(&state.picker, code, modifiers),
            true,
        );
    }

    if show_settings && matches!(code, crossterm::event::KeyCode::Esc) {
        return OverlayInputResult::CloseSettings;
    }

    OverlayInputResult::NotHandled
}

fn dispatch_overlay_action<P: DataProvider>(
    provider: &mut P,
    state: &mut CoreState,
    action: Option<CoreAction>,
    clear_persistent: bool,
) -> OverlayInputResult {
    let Some(action) = action else {
        return OverlayInputResult::Consumed;
    };
    let result = if clear_persistent {
        dispatch_action_with_persistent_clear(provider, state, &action)
    } else {
        dispatch_action(provider, state, &action)
    };
    match result {
        Ok(effects) => OverlayInputResult::ApplyEffects(effects),
        Err(error) => OverlayInputResult::DispatchError(dispatch_error_message(&error)),
    }
}

fn handle_textarea_input<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    state: &mut CoreState,
    hooks: &mut H,
    textareas: &mut FormTextareas,
    input: &HostInputEvent,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(handled) = handle_chat_input_event(provider, state, hooks, textareas, input)? {
        return Ok(handled);
    }

    let Some(target) = active_textarea(state) else {
        return Ok(false);
    };
    let textarea = textarea_mut(textareas, target);
    let HostInputEvent::Key {
        key_event, code, ..
    } = input
    else {
        textarea.insert_str(match input {
            HostInputEvent::Paste(text) => text.as_str(),
            HostInputEvent::Key { .. } => "",
        });
        sync_state_from_textareas(state, textareas);
        return Ok(true);
    };
    let action = textarea_navigation_action(target, key_event, *code, textarea);

    let Some(action) = action else {
        if key_event.code == crossterm::event::KeyCode::Esc {
            return Ok(false);
        }
        textarea.input(textarea_input_from_key_event(*key_event));
        sync_state_from_textareas(state, textareas);
        return Ok(true);
    };

    match dispatch_with_effects(provider, state, hooks, &action) {
        Ok(()) => Ok(true),
        Err(error) => {
            state.status_message = Some(error);
            Ok(true)
        }
    }
}

fn handle_chat_input_event<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    state: &mut CoreState,
    hooks: &mut H,
    textareas: &mut FormTextareas,
    input: &HostInputEvent,
) -> Result<Option<bool>, Box<dyn std::error::Error>> {
    if !chat_input_active(state) {
        return Ok(None);
    }

    match input {
        HostInputEvent::Paste(text) => {
            insert_chat_text(
                &mut textareas.chat_input,
                flatten_chat_input_for_display(text).as_str(),
            );
            sync_state_from_chat_input(state, textareas);
            Ok(Some(true))
        }
        HostInputEvent::Key {
            key_event,
            code,
            modifiers,
        } => {
            if handle_chat_command_navigation(state, textareas, *code) {
                return Ok(Some(true));
            }
            if let Some(action) = chat_input_action(*key_event, *code, *modifiers) {
                return match action {
                    ChatInputAction::Edit(edit) => {
                        apply_chat_edit(&mut textareas.chat_input, edit);
                        sync_state_from_chat_input(state, textareas);
                        Ok(Some(true))
                    }
                    ChatInputAction::Submit => {
                        handle_chat_submit_or_command(provider, state, hooks, textareas).map(Some)
                    }
                    ChatInputAction::Dispatch(core_action) => {
                        match dispatch_with_effects(provider, state, hooks, &core_action) {
                            Ok(()) => Ok(Some(true)),
                            Err(error) => {
                                state.status_message = Some(error);
                                Ok(Some(true))
                            }
                        }
                    }
                    ChatInputAction::Passthrough => Ok(Some(false)),
                };
            }
            Ok(Some(false))
        }
    }
}

fn handle_paste_input<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    state: &mut CoreState,
    hooks: &mut H,
    text: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let normalized = flatten_single_line_paste(text);
    if normalized.is_empty() {
        return Ok(false);
    }

    if let Some(actions) = paste_actions_for_state(state, normalized.as_str()) {
        for action in actions {
            dispatch_with_effects(provider, state, hooks, &action).map_err(io::Error::other)?;
        }
        return Ok(true);
    }

    Ok(false)
}

fn handle_chat_submit_or_command<P: DataProvider, H: RuntimeLoopHooks<P>>(
    provider: &mut P,
    state: &mut CoreState,
    hooks: &mut H,
    textareas: &mut FormTextareas,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Submit and slash-command matching operate on the normalized single-line payload.
    state.chat_input = chat_input_submit_value(state.chat_input.as_str());
    sync_chat_command_selection(textareas, state.chat_input.as_str());
    if let Some(command_action) =
        selected_slash_command_action(state.chat_input.as_str(), textareas.chat_command_selected)
            .or_else(|| chat_slash_command_action(state.chat_input.as_str()))
    {
        state.chat_input.clear();
        textareas.chat_command_selected = 0;
        return match dispatch_with_effects(provider, state, hooks, &command_action) {
            Ok(()) => Ok(true),
            Err(error) => {
                state.status_message = Some(error);
                Ok(true)
            }
        };
    }

    if state.chat_input.trim_start().starts_with('/') {
        state.status_message = Some(UNKNOWN_SLASH_COMMAND_MESSAGE.to_string());
        return Ok(true);
    }

    match dispatch_with_effects(provider, state, hooks, &CoreAction::ChatSubmit) {
        Ok(()) => Ok(true),
        Err(error) => {
            state.status_message = Some(error);
            Ok(true)
        }
    }
}

fn handle_chat_command_navigation(
    state: &CoreState,
    textareas: &mut FormTextareas,
    code: crossterm::event::KeyCode,
) -> bool {
    let matches = matching_slash_commands(state.chat_input.as_str());
    if matches.is_empty() {
        return false;
    }
    match code {
        crossterm::event::KeyCode::Up => {
            if textareas.chat_command_selected == 0 {
                textareas.chat_command_selected = matches.len().saturating_sub(1);
            } else {
                textareas.chat_command_selected = textareas.chat_command_selected.saturating_sub(1);
            }
            true
        }
        crossterm::event::KeyCode::Down => {
            textareas.chat_command_selected =
                (textareas.chat_command_selected + 1) % matches.len().max(1);
            true
        }
        _ => false,
    }
}

fn paste_actions_for_state(state: &CoreState, text: &str) -> Option<Vec<CoreAction>> {
    if state.access_control.open {
        return paste_actions_for_access_control(state, text);
    }
    if state.add_memory.open {
        return nonempty_actions(char_input_actions(text, CoreAction::AddMemoryInput));
    }
    if state.rename_memory.form.open {
        return paste_actions_for_rename(state, text);
    }
    if state.transfer_modal.open {
        return paste_actions_for_transfer(state, text);
    }
    if let PickerState::Input { .. } = &state.picker {
        return nonempty_actions(char_input_actions(text, CoreAction::PickerInput));
    }
    if form_tab_flow::is_form_input_mode(state) && active_textarea(state).is_none() {
        return nonempty_actions(form_paste_actions(state, text));
    }
    if state.focus == PaneFocus::Search {
        return nonempty_actions(char_input_actions(text, CoreAction::SearchInput));
    }

    None
}

fn paste_actions_for_access_control(state: &CoreState, text: &str) -> Option<Vec<CoreAction>> {
    use tui_kit_runtime::{AccessControlFocus, AccessControlMode};

    if state.access_control.mode == AccessControlMode::Add
        && state.access_control.focus == AccessControlFocus::Principal
    {
        return Some(char_input_actions(text, CoreAction::AccessInput));
    }

    None
}

fn paste_actions_for_transfer(state: &CoreState, text: &str) -> Option<Vec<CoreAction>> {
    use tui_kit_runtime::{TransferModalFocus, TransferModalMode};

    if state.transfer_modal.mode != TransferModalMode::Edit {
        return None;
    }

    match state.transfer_modal.focus {
        TransferModalFocus::Principal | TransferModalFocus::Amount => {
            Some(char_input_actions(text, CoreAction::TransferInput))
        }
        TransferModalFocus::Max | TransferModalFocus::Submit => None,
    }
}

fn paste_actions_for_rename(state: &CoreState, text: &str) -> Option<Vec<CoreAction>> {
    use tui_kit_runtime::RenameModalFocus;

    if state.rename_memory.focus == RenameModalFocus::Name {
        return nonempty_actions(char_input_actions(text, CoreAction::RenameMemoryInput));
    }

    None
}

fn form_paste_actions(state: &CoreState, text: &str) -> Vec<CoreAction> {
    let mut state_for_actions = state.clone();
    text.chars()
        .filter_map(|c| {
            form_tab_flow::form_tab_action_from_key(
                crossterm::event::KeyCode::Char(c),
                &mut state_for_actions,
            )
        })
        .collect()
}

fn char_input_actions(text: &str, action: impl Fn(char) -> CoreAction) -> Vec<CoreAction> {
    text.chars().map(action).collect()
}

fn flatten_single_line_paste(text: &str) -> String {
    canonicalize_paste_line_endings(text)
        .lines()
        .collect::<Vec<_>>()
        .join(" ")
}

fn canonicalize_paste_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn nonempty_actions(actions: Vec<CoreAction>) -> Option<Vec<CoreAction>> {
    if actions.is_empty() {
        return None;
    }
    Some(actions)
}

fn sync_chat_command_selection(textareas: &mut FormTextareas, input: &str) {
    let matches = matching_slash_commands(input);
    if matches.is_empty() {
        textareas.chat_command_selected = 0;
    } else {
        textareas.chat_command_selected = textareas
            .chat_command_selected
            .min(matches.len().saturating_sub(1));
    }
}

fn chat_command_selection(state: &CoreState, textareas: &FormTextareas) -> Option<usize> {
    let matches = matching_slash_commands(state.chat_input.as_str());
    if matches.is_empty() {
        None
    } else {
        Some(
            textareas
                .chat_command_selected
                .min(matches.len().saturating_sub(1)),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChatEdit {
    MoveLeft,
    MoveRight,
    MoveHome,
    MoveEnd,
    Backspace,
    Delete,
    InsertChar(char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ChatInputAction {
    Edit(ChatEdit),
    Submit,
    Dispatch(CoreAction),
    Passthrough,
}

#[cfg(test)]
fn chat_edit_from_textarea_input(input: TextAreaInput) -> Option<ChatEdit> {
    match input.key {
        TextAreaKey::Char(c) if !input.ctrl && !input.alt => Some(ChatEdit::InsertChar(c)),
        TextAreaKey::Left => Some(ChatEdit::MoveLeft),
        TextAreaKey::Right => Some(ChatEdit::MoveRight),
        TextAreaKey::Home => Some(ChatEdit::MoveHome),
        TextAreaKey::End => Some(ChatEdit::MoveEnd),
        TextAreaKey::Backspace => Some(ChatEdit::Backspace),
        TextAreaKey::Delete => Some(ChatEdit::Delete),
        _ => None,
    }
}

fn chat_input_action(
    _key_event: crossterm::event::KeyEvent,
    code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) -> Option<ChatInputAction> {
    match (code, modifiers) {
        (crossterm::event::KeyCode::Tab, _) => {
            Some(ChatInputAction::Dispatch(CoreAction::FocusNext))
        }
        (crossterm::event::KeyCode::BackTab, _) => {
            Some(ChatInputAction::Dispatch(CoreAction::FocusPrev))
        }
        (crossterm::event::KeyCode::Enter, _) => Some(ChatInputAction::Submit),
        (crossterm::event::KeyCode::Left, modifiers)
            if modifiers.contains(crossterm::event::KeyModifiers::SHIFT)
                && !modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                && !modifiers.contains(crossterm::event::KeyModifiers::ALT) =>
        {
            Some(ChatInputAction::Dispatch(CoreAction::ChatScopePrev))
        }
        (crossterm::event::KeyCode::Right, modifiers)
            if modifiers.contains(crossterm::event::KeyModifiers::SHIFT)
                && !modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                && !modifiers.contains(crossterm::event::KeyModifiers::ALT) =>
        {
            Some(ChatInputAction::Dispatch(CoreAction::ChatScopeNext))
        }
        (crossterm::event::KeyCode::PageUp, _) => Some(ChatInputAction::Passthrough),
        (crossterm::event::KeyCode::PageDown, _) => Some(ChatInputAction::Passthrough),
        (crossterm::event::KeyCode::Home, modifiers)
            if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            Some(ChatInputAction::Passthrough)
        }
        (crossterm::event::KeyCode::End, modifiers)
            if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            Some(ChatInputAction::Passthrough)
        }
        (crossterm::event::KeyCode::Up, _) | (crossterm::event::KeyCode::Down, _) => {
            Some(ChatInputAction::Passthrough)
        }
        (crossterm::event::KeyCode::Esc, _) => Some(ChatInputAction::Passthrough),
        (crossterm::event::KeyCode::Left, _) => Some(ChatInputAction::Edit(ChatEdit::MoveLeft)),
        (crossterm::event::KeyCode::Right, _) => Some(ChatInputAction::Edit(ChatEdit::MoveRight)),
        (crossterm::event::KeyCode::Home, _) => Some(ChatInputAction::Edit(ChatEdit::MoveHome)),
        (crossterm::event::KeyCode::End, _) => Some(ChatInputAction::Edit(ChatEdit::MoveEnd)),
        (crossterm::event::KeyCode::Backspace, _) => {
            Some(ChatInputAction::Edit(ChatEdit::Backspace))
        }
        (crossterm::event::KeyCode::Delete, _) => Some(ChatInputAction::Edit(ChatEdit::Delete)),
        (crossterm::event::KeyCode::Char(c), modifiers)
            if !c.is_control()
                && !modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                && !modifiers.contains(crossterm::event::KeyModifiers::ALT) =>
        {
            Some(ChatInputAction::Edit(ChatEdit::InsertChar(c)))
        }
        _ => None,
    }
}

fn apply_chat_edit(chat_input: &mut ChatInputState, edit: ChatEdit) {
    match edit {
        ChatEdit::MoveLeft => {
            chat_input.cursor_col = chat_input.cursor_col.saturating_sub(1);
        }
        ChatEdit::MoveRight => {
            chat_input.cursor_col = (chat_input.cursor_col + 1).min(chat_input_len(chat_input));
        }
        ChatEdit::MoveHome => {
            chat_input.cursor_col = 0;
        }
        ChatEdit::MoveEnd => {
            chat_input.cursor_col = chat_input_len(chat_input);
        }
        ChatEdit::Backspace => {
            if chat_input.cursor_col > 0 {
                let remove_at = chat_input.cursor_col - 1;
                let start = chat_input_byte_index(chat_input.value.as_str(), remove_at);
                let end = chat_input_byte_index(chat_input.value.as_str(), chat_input.cursor_col);
                chat_input.value.replace_range(start..end, "");
                chat_input.cursor_col = remove_at;
            }
        }
        ChatEdit::Delete => {
            let len = chat_input_len(chat_input);
            if chat_input.cursor_col < len {
                let start = chat_input_byte_index(chat_input.value.as_str(), chat_input.cursor_col);
                let end =
                    chat_input_byte_index(chat_input.value.as_str(), chat_input.cursor_col + 1);
                chat_input.value.replace_range(start..end, "");
            }
        }
        ChatEdit::InsertChar(c) => {
            insert_chat_text(chat_input, c.to_string().as_str());
        }
    }
}

fn insert_chat_text(chat_input: &mut ChatInputState, text: &str) {
    let insert_at = chat_input_byte_index(chat_input.value.as_str(), chat_input.cursor_col);
    chat_input.value.insert_str(insert_at, text);
    chat_input.cursor_col += text.chars().count();
}

fn chat_input_len(chat_input: &ChatInputState) -> usize {
    chat_input.value.chars().count()
}

fn chat_input_byte_index(value: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

fn textarea_navigation_action(
    target: ActiveTextarea,
    _key_event: &crossterm::event::KeyEvent,
    code: crossterm::event::KeyCode,
    textarea: &TextArea<'static>,
) -> Option<CoreAction> {
    match code {
        crossterm::event::KeyCode::Tab => Some(textarea_next_field_action(target)),
        crossterm::event::KeyCode::BackTab => Some(textarea_prev_field_action(target)),
        crossterm::event::KeyCode::Up if textarea_is_at_first_row(textarea) => {
            Some(textarea_prev_field_action(target))
        }
        crossterm::event::KeyCode::Down if textarea_is_at_last_row(textarea) => {
            Some(textarea_next_field_action(target))
        }
        _ => None,
    }
}

fn textarea_next_field_action(target: ActiveTextarea) -> CoreAction {
    match target {
        ActiveTextarea::CreateDescription => CoreAction::CreateNextField,
        ActiveTextarea::InsertText => CoreAction::InsertNextField,
        ActiveTextarea::ChatInput => CoreAction::FocusNext,
    }
}

fn textarea_prev_field_action(target: ActiveTextarea) -> CoreAction {
    match target {
        ActiveTextarea::CreateDescription => CoreAction::CreatePrevField,
        ActiveTextarea::InsertText => CoreAction::InsertPrevField,
        ActiveTextarea::ChatInput => CoreAction::FocusPrev,
    }
}

fn textarea_is_at_first_row(textarea: &TextArea<'static>) -> bool {
    textarea.cursor().0 == 0
}

fn textarea_is_at_last_row(textarea: &TextArea<'static>) -> bool {
    textarea.cursor().0 + 1 >= textarea.lines().len()
}

fn active_textarea(state: &CoreState) -> Option<ActiveTextarea> {
    if state.focus != PaneFocus::Form && state.focus != PaneFocus::Extra {
        return None;
    }

    if state.current_tab_id == KINIC_CREATE_TAB_ID
        && state.create_focus == tui_kit_runtime::CreateModalFocus::Description
    {
        return Some(ActiveTextarea::CreateDescription);
    }

    if state.current_tab_id == tui_kit_runtime::kinic_tabs::KINIC_INSERT_TAB_ID
        && state.insert_focus == tui_kit_runtime::InsertFormFocus::Text
        && matches!(
            state.insert_mode,
            tui_kit_runtime::InsertMode::InlineText | tui_kit_runtime::InsertMode::ManualEmbedding
        )
    {
        return Some(ActiveTextarea::InsertText);
    }

    if chat_input_active(state) {
        return Some(ActiveTextarea::ChatInput);
    }

    None
}

fn textarea_mut(textareas: &mut FormTextareas, target: ActiveTextarea) -> &mut TextArea<'static> {
    match target {
        ActiveTextarea::CreateDescription => &mut textareas.create_description,
        ActiveTextarea::InsertText => &mut textareas.insert_text,
        ActiveTextarea::ChatInput => unreachable!("chat input no longer uses textarea"),
    }
}

fn sync_form_textareas_from_state(textareas: &mut FormTextareas, state: &CoreState) {
    sync_textarea_from_string(
        &mut textareas.create_description,
        state.create_description.as_str(),
    );
    sync_textarea_from_string(&mut textareas.insert_text, state.insert_text.as_str());
    sync_chat_input_from_state(&mut textareas.chat_input, state.chat_input.as_str());
}

fn sync_textarea_from_string(textarea: &mut TextArea<'static>, value: &str) {
    if textarea.lines().join("\n") == value {
        return;
    }
    *textarea = textarea_from_text(value);
}

fn textarea_from_text(value: &str) -> TextArea<'static> {
    TextArea::from(value.split('\n'))
}

#[cfg(test)]
fn chat_input_from_text(value: &str) -> ChatInputState {
    let single_line_value = flatten_chat_input_for_display(value);
    ChatInputState {
        cursor_col: single_line_value.chars().count(),
        value: single_line_value,
    }
}

fn chat_input_active(state: &CoreState) -> bool {
    state.current_tab_id == KINIC_MEMORIES_TAB_ID && state.focus == PaneFocus::Extra
}

// Chat input is rendered and edited as a true single-line widget.
fn chat_input_display_value(chat_input: &ChatInputState) -> String {
    chat_input.value.clone()
}

fn chat_input_submit_value(value: &str) -> String {
    normalize_chat_input_lines(value)
}

fn chat_input_cursor(
    active: Option<ActiveTextarea>,
    chat_input: &ChatInputState,
) -> Option<(usize, usize)> {
    if active != Some(ActiveTextarea::ChatInput) {
        return None;
    }
    Some((0, chat_input.cursor_col))
}

fn sync_chat_input_from_state(chat_input: &mut ChatInputState, value: &str) {
    let single_line_value = flatten_chat_input_for_display(value);
    if chat_input.value == single_line_value {
        chat_input.cursor_col = chat_input.cursor_col.min(chat_input_len(chat_input));
        return;
    }
    chat_input.value = single_line_value;
    chat_input.cursor_col = chat_input_len(chat_input);
}

fn sync_state_from_textareas(state: &mut CoreState, textareas: &FormTextareas) {
    let create_description = textareas.create_description.lines().join("\n");
    if state.create_description != create_description {
        state.create_description = create_description;
        state.create_error = None;
        if state.create_submit_state == tui_kit_runtime::CreateSubmitState::Error {
            state.create_submit_state = tui_kit_runtime::CreateSubmitState::Idle;
        }
    }

    let insert_text = textareas.insert_text.lines().join("\n");
    if state.insert_text != insert_text {
        state.insert_text = insert_text;
        state.insert_error = None;
        if state.insert_submit_state == tui_kit_runtime::CreateSubmitState::Error {
            state.insert_submit_state = tui_kit_runtime::CreateSubmitState::Idle;
        }
    }

    let display_chat_input = chat_input_display_value(&textareas.chat_input);
    if state.chat_input != display_chat_input {
        state.chat_input = display_chat_input;
    }
}

fn sync_state_from_chat_input(state: &mut CoreState, textareas: &mut FormTextareas) {
    let display_chat_input = chat_input_display_value(&textareas.chat_input);
    if state.chat_input != display_chat_input {
        state.chat_input = display_chat_input;
    }
    sync_chat_command_selection(textareas, state.chat_input.as_str());
}

fn textarea_cursor(
    active: Option<ActiveTextarea>,
    target: ActiveTextarea,
    textarea: &TextArea<'static>,
) -> Option<(usize, usize)> {
    if active != Some(target) {
        return None;
    }
    Some(textarea.cursor())
}

fn textarea_input_from_key_event(key_event: crossterm::event::KeyEvent) -> TextAreaInput {
    let key = match key_event.code {
        crossterm::event::KeyCode::Char(c) => TextAreaKey::Char(c),
        crossterm::event::KeyCode::Backspace => TextAreaKey::Backspace,
        crossterm::event::KeyCode::Enter => TextAreaKey::Enter,
        crossterm::event::KeyCode::Left => TextAreaKey::Left,
        crossterm::event::KeyCode::Right => TextAreaKey::Right,
        crossterm::event::KeyCode::Up => TextAreaKey::Up,
        crossterm::event::KeyCode::Down => TextAreaKey::Down,
        crossterm::event::KeyCode::Tab => TextAreaKey::Tab,
        crossterm::event::KeyCode::Delete => TextAreaKey::Delete,
        crossterm::event::KeyCode::Home => TextAreaKey::Home,
        crossterm::event::KeyCode::End => TextAreaKey::End,
        crossterm::event::KeyCode::PageUp => TextAreaKey::PageUp,
        crossterm::event::KeyCode::PageDown => TextAreaKey::PageDown,
        crossterm::event::KeyCode::Esc => TextAreaKey::Esc,
        crossterm::event::KeyCode::F(value) => TextAreaKey::F(value),
        _ => TextAreaKey::Null,
    };
    TextAreaInput {
        key,
        ctrl: key_event
            .modifiers
            .contains(crossterm::event::KeyModifiers::CONTROL),
        alt: key_event
            .modifiers
            .contains(crossterm::event::KeyModifiers::ALT),
        shift: key_event
            .modifiers
            .contains(crossterm::event::KeyModifiers::SHIFT),
    }
}

fn access_control_overlay_action(
    code: crossterm::event::KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
    state: &CoreState,
) -> Option<CoreAction> {
    use tui_kit_runtime::{AccessControlFocus, AccessControlMode};

    if state.access_control.mode == AccessControlMode::Confirm {
        return confirm_overlay_action(
            code,
            CoreAction::CloseAccessControl,
            CoreAction::AccessSubmit,
            CoreAction::AccessCycleAction,
        );
    }

    if let Some(action) = match code {
        crossterm::event::KeyCode::Esc => Some(CoreAction::CloseAccessControl),
        crossterm::event::KeyCode::Tab if state.access_control.mode == AccessControlMode::Add => {
            Some(CoreAction::AccessNextField)
        }
        crossterm::event::KeyCode::BackTab
            if state.access_control.mode == AccessControlMode::Add =>
        {
            Some(CoreAction::AccessPrevField)
        }
        _ => None,
    } {
        return Some(action);
    }

    let directional_action = match state.access_control.mode {
        AccessControlMode::Action | AccessControlMode::Confirm => overlay_directional_action(
            code,
            CoreAction::AccessCycleActionPrev,
            CoreAction::AccessCycleAction,
        ),
        AccessControlMode::Add if state.access_control.focus == AccessControlFocus::Role => {
            overlay_directional_action(
                code,
                CoreAction::AccessCycleRolePrev,
                CoreAction::AccessCycleRole,
            )
        }
        _ => None,
    };
    if directional_action.is_some() {
        return directional_action;
    }

    match code {
        crossterm::event::KeyCode::Backspace
            if state.access_control.mode == AccessControlMode::Add =>
        {
            Some(CoreAction::AccessBackspace)
        }
        crossterm::event::KeyCode::Enter => Some(CoreAction::AccessSubmit),
        crossterm::event::KeyCode::Char(c)
            if !c.is_control()
                && state.access_control.mode == AccessControlMode::Add
                && state.access_control.focus == AccessControlFocus::Principal =>
        {
            Some(CoreAction::AccessInput(c))
        }
        _ => None,
    }
}

fn picker_overlay_action(
    picker: &PickerState,
    code: crossterm::event::KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
) -> Option<CoreAction> {
    match picker {
        PickerState::Closed => None,
        PickerState::List { .. } => match code {
            crossterm::event::KeyCode::Esc => Some(CoreAction::ClosePicker),
            crossterm::event::KeyCode::Enter => Some(CoreAction::SubmitPicker),
            crossterm::event::KeyCode::Down => Some(CoreAction::MovePickerNext),
            crossterm::event::KeyCode::Up => Some(CoreAction::MovePickerPrev),
            crossterm::event::KeyCode::Char('d') => Some(CoreAction::DeleteSelectedPickerItem),
            _ => None,
        },
        PickerState::Input { .. } => match code {
            crossterm::event::KeyCode::Esc => Some(CoreAction::ClosePicker),
            crossterm::event::KeyCode::Enter => Some(CoreAction::SubmitPicker),
            crossterm::event::KeyCode::Backspace => Some(CoreAction::PickerBackspace),
            crossterm::event::KeyCode::Char(c) if !c.is_control() => {
                Some(CoreAction::PickerInput(c))
            }
            _ => None,
        },
    }
}

fn simple_text_overlay_action(
    code: crossterm::event::KeyCode,
    close_action: CoreAction,
    submit_action: CoreAction,
    backspace_action: CoreAction,
    input_action: impl Fn(char) -> CoreAction,
) -> Option<CoreAction> {
    match code {
        crossterm::event::KeyCode::Esc => Some(close_action),
        crossterm::event::KeyCode::Enter => Some(submit_action),
        crossterm::event::KeyCode::Backspace => Some(backspace_action),
        crossterm::event::KeyCode::Char(c) if !c.is_control() => Some(input_action(c)),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn focusable_text_overlay_action(
    code: crossterm::event::KeyCode,
    editable: bool,
    close_action: CoreAction,
    submit_action: CoreAction,
    next_action: CoreAction,
    prev_action: CoreAction,
    backspace_action: CoreAction,
    input_action: impl Fn(char) -> CoreAction,
) -> Option<CoreAction> {
    match code {
        crossterm::event::KeyCode::Esc => Some(close_action),
        crossterm::event::KeyCode::Tab => Some(next_action),
        crossterm::event::KeyCode::BackTab => Some(prev_action),
        crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Up => Some(prev_action),
        crossterm::event::KeyCode::Right | crossterm::event::KeyCode::Down => Some(next_action),
        crossterm::event::KeyCode::Backspace if editable => Some(backspace_action),
        crossterm::event::KeyCode::Enter if editable => Some(next_action),
        crossterm::event::KeyCode::Enter => Some(submit_action),
        crossterm::event::KeyCode::Char(c) if editable && !c.is_control() => Some(input_action(c)),
        _ => None,
    }
}

fn confirm_overlay_action(
    code: crossterm::event::KeyCode,
    close_action: CoreAction,
    submit_action: CoreAction,
    toggle_action: CoreAction,
) -> Option<CoreAction> {
    match code {
        crossterm::event::KeyCode::Esc => Some(close_action),
        crossterm::event::KeyCode::Enter => Some(submit_action),
        crossterm::event::KeyCode::Tab
        | crossterm::event::KeyCode::BackTab
        | crossterm::event::KeyCode::Left
        | crossterm::event::KeyCode::Right
        | crossterm::event::KeyCode::Up
        | crossterm::event::KeyCode::Down => Some(toggle_action),
        _ => None,
    }
}

fn overlay_directional_action(
    code: crossterm::event::KeyCode,
    prev_action: CoreAction,
    next_action: CoreAction,
) -> Option<CoreAction> {
    match code {
        crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Up => Some(prev_action),
        crossterm::event::KeyCode::Right | crossterm::event::KeyCode::Down => Some(next_action),
        _ => None,
    }
}

fn add_memory_overlay_action(
    code: crossterm::event::KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
    _state: &CoreState,
) -> Option<CoreAction> {
    simple_text_overlay_action(
        code,
        CoreAction::CloseAddMemory,
        CoreAction::AddMemorySubmit,
        CoreAction::AddMemoryBackspace,
        CoreAction::AddMemoryInput,
    )
}

fn remove_memory_overlay_action(
    code: crossterm::event::KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
    _state: &CoreState,
) -> Option<CoreAction> {
    confirm_overlay_action(
        code,
        CoreAction::CloseRemoveMemory,
        CoreAction::RemoveMemorySubmit,
        CoreAction::RemoveMemoryToggleConfirm,
    )
}

fn transfer_overlay_action(
    code: crossterm::event::KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
    state: &CoreState,
) -> Option<CoreAction> {
    use tui_kit_runtime::{TransferModalFocus, TransferModalMode};

    if state.transfer_modal.mode == TransferModalMode::Confirm {
        return confirm_overlay_action(
            code,
            CoreAction::CloseTransferModal,
            CoreAction::TransferSubmit,
            CoreAction::TransferConfirmToggle,
        );
    }

    if let Some(action) = match code {
        crossterm::event::KeyCode::Esc => Some(CoreAction::CloseTransferModal),
        crossterm::event::KeyCode::Tab => Some(CoreAction::TransferNextField),
        crossterm::event::KeyCode::BackTab => Some(CoreAction::TransferPrevField),
        _ => None,
    } {
        return Some(action);
    }

    if let Some(action) = overlay_directional_action(
        code,
        CoreAction::TransferPrevField,
        CoreAction::TransferNextField,
    ) {
        return Some(action);
    }

    match code {
        crossterm::event::KeyCode::Backspace => Some(CoreAction::TransferBackspace),
        crossterm::event::KeyCode::Enter => match state.transfer_modal.focus {
            TransferModalFocus::Max => Some(CoreAction::TransferApplyMax),
            TransferModalFocus::Submit => Some(CoreAction::TransferSubmit),
            TransferModalFocus::Principal | TransferModalFocus::Amount => {
                Some(CoreAction::TransferNextField)
            }
        },
        crossterm::event::KeyCode::Char(c) if !c.is_control() => match state.transfer_modal.focus {
            TransferModalFocus::Principal | TransferModalFocus::Amount => {
                Some(CoreAction::TransferInput(c))
            }
            TransferModalFocus::Max | TransferModalFocus::Submit => None,
        },
        _ => None,
    }
}

fn rename_overlay_action(
    code: crossterm::event::KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
    state: &CoreState,
) -> Option<CoreAction> {
    use tui_kit_runtime::RenameModalFocus;

    focusable_text_overlay_action(
        code,
        state.rename_memory.focus == RenameModalFocus::Name,
        CoreAction::CloseRenameMemory,
        CoreAction::RenameMemorySubmit,
        CoreAction::RenameMemoryNextField,
        CoreAction::RenameMemoryPrevField,
        CoreAction::RenameMemoryBackspace,
        CoreAction::RenameMemoryInput,
    )
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
            | CoreAction::InsertPrevMode
            | CoreAction::InsertNextMode
            | CoreAction::InsertSubmit
            | CoreAction::OpenPicker(_)
            | CoreAction::MovePickerNext
            | CoreAction::MovePickerPrev
            | CoreAction::DeleteSelectedPickerItem
            | CoreAction::SubmitPicker
            | CoreAction::PickerInput(_)
            | CoreAction::PickerBackspace
            | CoreAction::CreateInput(_)
            | CoreAction::CreateBackspace
            | CoreAction::CreateSubmit
            | CoreAction::OpenRenameMemory
            | CoreAction::CloseRenameMemory
            | CoreAction::RenameMemoryInput(_)
            | CoreAction::RenameMemoryBackspace
            | CoreAction::RenameMemoryNextField
            | CoreAction::RenameMemoryPrevField
            | CoreAction::RenameMemorySubmit
            | CoreAction::OpenTransferModal
            | CoreAction::CloseTransferModal
            | CoreAction::TransferInput(_)
            | CoreAction::TransferBackspace
            | CoreAction::TransferNextField
            | CoreAction::TransferPrevField
            | CoreAction::TransferApplyMax
            | CoreAction::TransferSubmit
            | CoreAction::TransferConfirmToggle
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
