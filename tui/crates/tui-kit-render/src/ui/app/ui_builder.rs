//! Builder-style setters for constructing a render snapshot.

use crate::ui::animation::AnimationState;
use crate::ui::model::{UiContextNode, UiItemContent, UiItemSummary};
use crate::ui::search::CompletionCandidate;
use tui_kit_runtime::{
    AccessControlAction, AccessControlFocus, AccessControlMode, AccessControlRole, CreateCostState,
    CreateModalFocus, CreateSubmitState, InsertFormFocus, InsertMode, PickerState, SearchScope,
    SettingsSnapshot,
};

use super::{Focus, TabId, TabSpec, TuiKitUi, UiConfig};

impl<'a> TuiKitUi<'a> {
    #[must_use]
    pub fn ui_summaries(mut self, items: &'a [UiItemSummary]) -> Self {
        self.ui_summaries = items;
        self
    }

    #[must_use]
    pub fn ui_selected_content(mut self, content: Option<&'a UiItemContent>) -> Self {
        self.ui_selected_content = content;
        self
    }

    #[must_use]
    pub fn ui_context_node(mut self, node: Option<&'a UiContextNode>) -> Self {
        self.ui_context_node = node;
        self
    }

    #[must_use]
    pub fn ui_total_count(mut self, count: usize) -> Self {
        self.ui_total_count = count;
        self
    }

    #[must_use]
    pub fn in_context_items_view(mut self, v: bool) -> Self {
        self.in_context_items_view = v;
        self
    }

    #[must_use]
    pub fn show_context_panel(mut self, show: bool) -> Self {
        self.show_context_panel = show;
        self
    }

    #[must_use]
    pub fn target_size_bytes(mut self, bytes: Option<u64>) -> Self {
        self.target_size_bytes = bytes;
        self
    }

    #[must_use]
    pub fn list_selected(mut self, selected: Option<usize>) -> Self {
        self.list_selected = selected;
        self
    }

    #[must_use]
    pub fn list_scroll(mut self, offset: usize) -> Self {
        self.list_scroll_offset = Some(offset);
        self
    }

    #[must_use]
    pub fn candidates(mut self, candidates: &'a [CompletionCandidate]) -> Self {
        self.candidates = candidates;
        self
    }

    #[must_use]
    pub fn context_tree(mut self, tree: &'a [(String, usize)]) -> Self {
        self.context_tree = tree;
        self
    }

    #[must_use]
    pub fn filtered_context_indices(mut self, indices: &'a [usize]) -> Self {
        self.filtered_context_indices = indices;
        self
    }

    #[must_use]
    pub fn context_details_loading(mut self, loading: bool) -> Self {
        self.context_details_loading = loading;
        self
    }

    #[must_use]
    pub fn context_details_failed(mut self, failed: bool) -> Self {
        self.context_details_failed = failed;
        self
    }

    #[must_use]
    pub fn search_input(mut self, input: &'a str) -> Self {
        self.search_input = input;
        self
    }

    #[must_use]
    pub fn search_scope(mut self, scope: SearchScope) -> Self {
        self.search_scope = scope;
        self
    }

    #[must_use]
    pub fn current_tab_id(mut self, tab_id: TabId) -> Self {
        self.current_tab_id = tab_id;
        self
    }

    #[must_use]
    pub fn tab_specs(mut self, tabs: Vec<TabSpec>) -> Self {
        self.tab_specs = tabs;
        self
    }

    #[must_use]
    pub fn ui_config(mut self, config: UiConfig) -> Self {
        self.tab_specs = config.tabs.clone();
        self.ui_config = config;
        self
    }

    #[must_use]
    pub fn focus(mut self, focus: Focus) -> Self {
        self.focus = focus;
        self
    }

    #[must_use]
    pub fn completion_selected(mut self, index: usize) -> Self {
        self.completion_selected = index;
        self
    }

    #[must_use]
    pub fn show_completion(mut self, show: bool) -> Self {
        self.show_completion = show;
        self
    }

    #[must_use]
    pub fn show_help(mut self, show: bool) -> Self {
        self.show_help = show;
        self
    }

    #[must_use]
    pub fn show_settings(mut self, show: bool) -> Self {
        self.show_settings = show;
        self
    }

    #[must_use]
    pub fn show_create_modal(mut self, show: bool) -> Self {
        self.show_create_modal = show;
        self
    }

    #[must_use]
    pub fn create_name(mut self, value: &'a str) -> Self {
        self.create_name = value;
        self
    }

    #[must_use]
    pub fn create_description(mut self, value: &'a str) -> Self {
        self.create_description = value;
        self
    }

    #[must_use]
    pub fn create_description_cursor(mut self, value: Option<(usize, usize)>) -> Self {
        self.create_description_cursor = value;
        self
    }

    #[must_use]
    pub fn create_submit_state(mut self, value: CreateSubmitState) -> Self {
        self.create_submit_state = value;
        self
    }

    #[must_use]
    pub fn create_spinner_frame(mut self, value: usize) -> Self {
        self.create_spinner_frame = value;
        self
    }

    #[must_use]
    pub fn create_error(mut self, value: Option<&'a str>) -> Self {
        self.create_error = value;
        self
    }

    #[must_use]
    pub fn create_focus(mut self, value: CreateModalFocus) -> Self {
        self.create_focus = value;
        self
    }

    #[must_use]
    pub fn create_cost_state(mut self, value: &'a CreateCostState) -> Self {
        self.create_cost_state = value;
        self
    }

    #[must_use]
    pub fn settings_snapshot(mut self, value: Option<&'a SettingsSnapshot>) -> Self {
        self.settings_snapshot = value;
        self
    }

    #[must_use]
    pub fn picker(mut self, value: &'a PickerState) -> Self {
        self.picker = value;
        self
    }

    #[must_use]
    pub fn saved_default_memory_id(mut self, value: Option<&'a str>) -> Self {
        self.saved_default_memory_id = value;
        self
    }

    #[must_use]
    pub fn insert_mode(mut self, value: InsertMode) -> Self {
        self.insert_mode = value;
        self
    }

    #[must_use]
    pub fn insert_memory_id(mut self, value: &'a str) -> Self {
        self.insert_memory_id = value;
        self
    }

    #[must_use]
    pub fn insert_memory_placeholder(mut self, value: Option<&'a str>) -> Self {
        self.insert_memory_placeholder = value;
        self
    }

    #[must_use]
    pub fn insert_expected_dim(mut self, value: Option<u64>) -> Self {
        self.insert_expected_dim = value;
        self
    }

    #[must_use]
    pub fn insert_expected_dim_loading(mut self, value: bool) -> Self {
        self.insert_expected_dim_loading = value;
        self
    }

    #[must_use]
    pub fn insert_current_dim(mut self, value: Option<&'a str>) -> Self {
        self.insert_current_dim = value;
        self
    }

    #[must_use]
    pub fn insert_validation_message(mut self, value: Option<&'a str>) -> Self {
        self.insert_validation_message = value;
        self
    }

    #[must_use]
    pub fn insert_tag(mut self, value: &'a str) -> Self {
        self.insert_tag = value;
        self
    }

    #[must_use]
    pub fn insert_text(mut self, value: &'a str) -> Self {
        self.insert_text = value;
        self
    }

    #[must_use]
    pub fn insert_text_cursor(mut self, value: Option<(usize, usize)>) -> Self {
        self.insert_text_cursor = value;
        self
    }

    #[must_use]
    pub fn insert_file_path(mut self, value: &'a str) -> Self {
        self.insert_file_path = value;
        self
    }

    #[must_use]
    pub fn insert_embedding(mut self, value: &'a str) -> Self {
        self.insert_embedding = value;
        self
    }

    #[must_use]
    pub fn insert_submit_state(mut self, value: CreateSubmitState) -> Self {
        self.insert_submit_state = value;
        self
    }

    #[must_use]
    pub fn insert_spinner_frame(mut self, value: usize) -> Self {
        self.insert_spinner_frame = value;
        self
    }

    #[must_use]
    pub fn insert_error(mut self, value: Option<&'a str>) -> Self {
        self.insert_error = value;
        self
    }

    #[must_use]
    pub fn insert_focus(mut self, value: InsertFormFocus) -> Self {
        self.insert_focus = value;
        self
    }

    #[must_use]
    pub fn access_control_open(mut self, value: bool) -> Self {
        self.access_control_open = value;
        self
    }

    #[must_use]
    pub fn access_control_mode(mut self, value: AccessControlMode) -> Self {
        self.access_control_mode = value;
        self
    }

    #[must_use]
    pub fn access_control_memory_id(mut self, value: &'a str) -> Self {
        self.access_control_memory_id = value;
        self
    }

    #[must_use]
    pub fn access_control_action(mut self, value: AccessControlAction) -> Self {
        self.access_control_action = value;
        self
    }

    #[must_use]
    pub fn access_control_role(mut self, value: AccessControlRole) -> Self {
        self.access_control_role = value;
        self
    }

    #[must_use]
    pub fn access_control_current_role(mut self, value: AccessControlRole) -> Self {
        self.access_control_current_role = value;
        self
    }

    #[must_use]
    pub fn access_control_principal_id(mut self, value: &'a str) -> Self {
        self.access_control_principal_id = value;
        self
    }

    #[must_use]
    pub fn access_control_confirm_yes(mut self, value: bool) -> Self {
        self.access_control_confirm_yes = value;
        self
    }

    #[must_use]
    pub fn access_control_submit_state(mut self, value: CreateSubmitState) -> Self {
        self.access_control_submit_state = value;
        self
    }

    #[must_use]
    pub fn access_control_error(mut self, value: Option<&'a str>) -> Self {
        self.access_control_error = value;
        self
    }

    #[must_use]
    pub fn access_control_focus(mut self, value: AccessControlFocus) -> Self {
        self.access_control_focus = value;
        self
    }

    #[must_use]
    pub fn status_message(mut self, msg: &'a str) -> Self {
        self.status_message = msg;
        self
    }

    #[must_use]
    pub fn inspector_scroll(mut self, scroll: usize) -> Self {
        self.inspector_scroll = scroll;
        self
    }

    #[must_use]
    pub fn animation_state(mut self, animation: &'a AnimationState) -> Self {
        self.animation = Some(animation);
        self
    }

    #[must_use]
    pub fn show_chat(mut self, show: bool) -> Self {
        self.show_chat_panel = show;
        self
    }

    #[must_use]
    pub fn chat_messages(mut self, messages: &'a [(String, String)]) -> Self {
        self.chat_messages = messages;
        self
    }

    #[must_use]
    pub fn chat_input(mut self, input: &'a str) -> Self {
        self.chat_input = input;
        self
    }

    #[must_use]
    pub fn chat_loading(mut self, loading: bool) -> Self {
        self.chat_loading = loading;
        self
    }

    #[must_use]
    pub fn chat_scroll(mut self, scroll: usize) -> Self {
        self.chat_scroll = scroll;
        self
    }
}
